use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::Field;
use super::ProtoEntry;
use super::ProtoIdent;
use super::ProtoLabel;
use super::ProtoSchema;
use super::ServiceMethod;
use super::Variant;
use super::utils::WrapperKind;
use super::utils::entry_sort_key;
use super::utils::resolve_transparent_ident;
use super::utils::to_snake_case;
use super::utils::wrapper_is_map;
use super::utils::wrapper_kind_for;
use super::utils::wrapper_label;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct GenericSpecialization {
    pub(crate) name: String,
    pub(crate) args: Vec<ProtoIdent>,
}

pub(crate) fn collect_imports(
    entries: &[&ProtoSchema],
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    file_name: &str,
    package_name: &str,
) -> std::io::Result<BTreeSet<String>> {
    let mut imports = BTreeSet::new();

    for entry in entries {
        match entry.content {
            ProtoEntry::Import { paths } => {
                for path in paths {
                    imports.insert(path.to_string());
                }
            }
            ProtoEntry::Struct { fields } => {
                collect_field_imports(&mut imports, ident_index, fields, file_name, package_name)?;
            }
            ProtoEntry::SimpleEnum { .. } => {}
            ProtoEntry::ComplexEnum { variants } => {
                for variant in variants {
                    collect_field_imports(&mut imports, ident_index, variant.fields, file_name, package_name)?;
                }
            }
            ProtoEntry::Service { methods, .. } => {
                collect_service_imports(&mut imports, ident_index, methods, file_name, package_name)?;
            }
        }
    }

    Ok(imports)
}

pub(crate) fn collect_generic_specializations(
    entries: &[&ProtoSchema],
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
) -> BTreeMap<ProtoIdent, Vec<GenericSpecialization>> {
    let mut specializations: BTreeMap<ProtoIdent, Vec<GenericSpecialization>> = BTreeMap::new();
    let generic_entries: BTreeMap<ProtoIdent, &ProtoSchema> = entries
        .iter()
        .filter(|entry| entry.generics.iter().any(|generic| matches!(generic.kind, super::GenericKind::Type)))
        .map(|entry| (entry.id, *entry))
        .collect();

    let mut register_specialization = |base: ProtoIdent, args: &[&ProtoIdent]| {
        if !generic_entries.contains_key(&base) {
            return;
        }
        let concrete_args: Vec<ProtoIdent> = args.iter().map(|arg| **arg).collect();
        let name = specialized_proto_name(base, &concrete_args);
        let entry = specializations.entry(base).or_default();
        if entry.iter().all(|existing| existing.name != name) {
            entry.push(GenericSpecialization { name, args: concrete_args });
        }
    };

    for entry in entries {
        match entry.content {
            ProtoEntry::Struct { fields } => {
                for field in fields {
                    if !field.generic_args.is_empty() {
                        register_specialization(field.proto_ident, field.generic_args);
                    }
                }
            }
            ProtoEntry::ComplexEnum { variants } => {
                for variant in variants {
                    for field in variant.fields {
                        if !field.generic_args.is_empty() {
                            register_specialization(field.proto_ident, field.generic_args);
                        }
                    }
                }
            }
            ProtoEntry::Service { methods, .. } => {
                for method in methods {
                    if !method.request_generic_args.is_empty() {
                        register_specialization(method.request, method.request_generic_args);
                    }
                    if !method.response_generic_args.is_empty() {
                        register_specialization(method.response, method.response_generic_args);
                    }
                }
            }
            ProtoEntry::SimpleEnum { .. } | ProtoEntry::Import { .. } => {}
        }
    }

    for entry in specializations.values_mut() {
        entry.sort_by(|left, right| left.name.cmp(&right.name));
    }

    let mut ordered = specializations;
    for (base, specs) in &mut ordered {
        let base_entry = ident_index.get(base);
        if let Some(base_entry) = base_entry {
            let param_count = base_entry.generics.iter().filter(|generic| matches!(generic.kind, super::GenericKind::Type)).count();
            specs.retain(|spec| spec.args.len() == param_count);
        }
    }

    ordered
}

pub(crate) fn render_entries(
    entries: &[&ProtoSchema],
    package_name: &str,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    specializations: &BTreeMap<ProtoIdent, Vec<GenericSpecialization>>,
) -> Vec<String> {
    let mut ordered_entries = entries.to_vec();
    ordered_entries.sort_by(|left, right| entry_sort_key(left).cmp(&entry_sort_key(right)));

    // Collect all proto_types that will be rendered via specializations
    // to avoid rendering concrete variant schemas twice
    let mut specialized_types = std::collections::BTreeSet::new();
    for (base_id, specs) in specializations {
        // Only collect if the base type actually has generics and is in our entries
        if entries.iter().any(|e| e.id == *base_id && !e.generics.is_empty()) {
            for spec in specs {
                specialized_types.insert(spec.name.as_str());
            }
        }
    }

    let mut rendered = Vec::new();
    let mut seen_proto_types = std::collections::BTreeSet::new();

    for entry in ordered_entries {
        // Skip duplicate proto_types (ensures stable ordering - first occurrence wins)
        if !seen_proto_types.insert(entry.id.proto_type) {
            continue;
        }

        // Skip concrete variant schemas that will be rendered via specializations
        // These are identifiable by having a proto_type that differs from their name
        // and matches a specialized type
        if entry.id.proto_type != entry.id.name && specialized_types.contains(entry.id.proto_type) {
            continue;
        }

        let specs = specializations.get(&entry.id);
        rendered.extend(render_entry(entry, package_name, ident_index, specs));
    }
    rendered
}

fn render_entry(
    entry: &ProtoSchema,
    package_name: &str,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    specializations: Option<&Vec<GenericSpecialization>>,
) -> Vec<String> {
    let type_generics: Vec<&str> =
        entry.generics.iter().filter(|generic| matches!(generic.kind, super::GenericKind::Type)).map(|generic| generic.name).collect();

    let has_type_generics = !type_generics.is_empty();
    if has_type_generics {
        let Some(specs) = specializations else {
            return Vec::new();
        };
        let mut rendered = Vec::new();
        for spec in specs {
            let substitution = build_substitution(&type_generics, &spec.args);
            let definition = match entry.content {
                ProtoEntry::Struct { fields } => render_struct(&spec.name, fields, package_name, ident_index, Some(&substitution)),
                ProtoEntry::SimpleEnum { variants } => render_simple_enum(&spec.name, variants),
                ProtoEntry::ComplexEnum { variants } => {
                    render_complex_enum(&spec.name, variants, package_name, ident_index, Some(&substitution))
                }
                ProtoEntry::Import { .. } => continue,
                ProtoEntry::Service { methods, .. } => render_service(&spec.name, methods, package_name, ident_index, Some(&substitution)),
            };
            rendered.push(definition);
        }
        return rendered;
    }

    let definition = match entry.content {
        ProtoEntry::Struct { fields } => render_struct(entry.id.proto_type, fields, package_name, ident_index, None),
        ProtoEntry::SimpleEnum { variants } => render_simple_enum(entry.id.proto_type, variants),
        ProtoEntry::ComplexEnum { variants } => render_complex_enum(entry.id.proto_type, variants, package_name, ident_index, None),
        ProtoEntry::Import { .. } => return Vec::new(),
        ProtoEntry::Service { methods, .. } => render_service(entry.id.proto_type, methods, package_name, ident_index, None),
    };

    vec![definition]
}

fn build_substitution<'a>(type_generics: &'a [&'a str], args: &'a [ProtoIdent]) -> BTreeMap<&'a str, ProtoIdent> {
    let mut substitution = BTreeMap::new();
    for (idx, name) in type_generics.iter().enumerate() {
        if let Some(arg) = args.get(idx) {
            substitution.insert(*name, *arg);
        }
    }
    substitution
}

fn render_struct(
    name: &str,
    fields: &[&Field],
    package_name: &str,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    substitution: Option<&BTreeMap<&str, ProtoIdent>>,
) -> String {
    if fields.is_empty() {
        return format!("message {name} {{}}\n");
    }

    let mut lines = Vec::new();
    for (idx, field) in fields.iter().enumerate() {
        lines.push(render_field(field, idx, package_name, ident_index, substitution));
    }

    format!("message {name} {{\n{}\n}}\n", lines.join("\n"))
}

fn render_simple_enum(name: &str, variants: &[&Variant]) -> String {
    let mut lines = Vec::new();
    for variant in variants {
        let value = variant.discriminant.unwrap_or_default();
        lines.push(format!("  {} = {};", variant.name, value));
    }
    format!("enum {name} {{\n{}\n}}\n", lines.join("\n"))
}

fn render_complex_enum(
    name: &str,
    variants: &[&Variant],
    package_name: &str,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    substitution: Option<&BTreeMap<&str, ProtoIdent>>,
) -> String {
    let mut nested_messages = Vec::new();
    let mut oneof_fields = Vec::new();

    for (idx, variant) in variants.iter().enumerate() {
        let tag = idx + 1;
        let variant_name = variant.name;
        let field_name = to_snake_case(variant_name);

        if variant.fields.is_empty() {
            let msg_name = format!("{name}{variant_name}");
            nested_messages.push(format!("message {msg_name} {{}}"));
            oneof_fields.push(format!("    {msg_name} {field_name} = {tag};"));
            continue;
        }

        if variant.fields.len() == 1 && variant.fields[0].name.is_none() {
            let proto_type = field_type_name(variant.fields[0], package_name, ident_index, substitution);
            oneof_fields.push(format!("    {proto_type} {field_name} = {tag};"));
            continue;
        }

        let msg_name = format!("{name}{variant_name}");
        let field_defs = render_named_fields(variant.fields, package_name, ident_index, substitution);
        nested_messages.push(format!("message {msg_name} {{\n{field_defs}\n}}"));
        oneof_fields.push(format!("    {msg_name} {field_name} = {tag};"));
    }

    format!(
        "{}\nmessage {} {{\n  oneof value {{\n{}\n  }}\n}}\n",
        nested_messages.join("\n\n"),
        name,
        oneof_fields.join("\n")
    )
}

fn render_named_fields(
    fields: &[&Field],
    package_name: &str,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    substitution: Option<&BTreeMap<&str, ProtoIdent>>,
) -> String {
    let mut lines = Vec::new();
    for (idx, field) in fields.iter().enumerate() {
        lines.push(render_field(field, idx, package_name, ident_index, substitution));
    }
    lines.join("\n")
}

fn render_field(
    field: &Field,
    idx: usize,
    package_name: &str,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    substitution: Option<&BTreeMap<&str, ProtoIdent>>,
) -> String {
    let name = field.name.map_or_else(|| format!("field_{idx}"), ToString::to_string);
    let label = match wrapper_label(field.wrapper, field.proto_ident, field.proto_label) {
        ProtoLabel::None => "",
        ProtoLabel::Optional => "optional ",
        ProtoLabel::Repeated => "repeated ",
    };
    let proto_type = field_type_name(field, package_name, ident_index, substitution);
    format!("  {label}{proto_type} {name} = {};", field.tag)
}

fn render_service(
    name: &str,
    methods: &[&ServiceMethod],
    package_name: &str,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    substitution: Option<&BTreeMap<&str, ProtoIdent>>,
) -> String {
    let mut lines = Vec::new();
    lines.push(format!("service {name} {{"));

    for method in methods {
        let request_type = method_type_name(
            method.request,
            method.request_generic_args,
            method.request_wrapper,
            package_name,
            ident_index,
            substitution,
        );
        let response_type = method_type_name(
            method.response,
            method.response_generic_args,
            method.response_wrapper,
            package_name,
            ident_index,
            substitution,
        );
        let response_type = if method.server_streaming {
            format!("stream {response_type}")
        } else {
            response_type
        };
        lines.push(format!("  rpc {}({}) returns ({});", method.name, request_type, response_type));
    }

    lines.push("}".to_string());
    lines.join("\n")
}

fn field_type_name(
    field: &Field,
    package_name: &str,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    substitution: Option<&BTreeMap<&str, ProtoIdent>>,
) -> String {
    if wrapper_is_map(field.wrapper, field.proto_ident)
        && let Some(map_type) = map_wrapper_type_name(field, package_name, ident_index, substitution)
    {
        return map_type;
    }

    if let Some(inner_type) = wrapper_inner_type_name(field, package_name, ident_index, substitution) {
        return inner_type;
    }

    let ident = resolve_transparent_ident(field.proto_ident, ident_index);
    if ident.proto_type.starts_with("map<") {
        return ident.proto_type.to_string();
    }

    proto_ident_type_name_with_generics(ident, field.generic_args, package_name, ident_index, substitution)
}

fn method_type_name(
    ident: ProtoIdent,
    generic_args: &[&ProtoIdent],
    wrapper: Option<ProtoIdent>,
    package_name: &str,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    substitution: Option<&BTreeMap<&str, ProtoIdent>>,
) -> String {
    if wrapper_is_map(wrapper, ident)
        && let Some(map_type) = method_map_type_name(generic_args, package_name, ident_index, substitution)
    {
        return map_type;
    }

    if let Some(inner_type) = method_wrapper_inner_type_name(ident, generic_args, wrapper, package_name, ident_index, substitution) {
        return inner_type;
    }

    proto_ident_type_name_with_generics(ident, generic_args, package_name, ident_index, substitution)
}

fn method_wrapper_inner_type_name(
    ident: ProtoIdent,
    generic_args: &[&ProtoIdent],
    wrapper: Option<ProtoIdent>,
    package_name: &str,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    substitution: Option<&BTreeMap<&str, ProtoIdent>>,
) -> Option<String> {
    let kind = wrapper_kind_for(wrapper, ident)?;
    if matches!(kind, WrapperKind::HashMap | WrapperKind::BTreeMap) {
        return None;
    }

    let inner = generic_args
        .first()
        .copied()
        .copied()
        .map(|ident| apply_substitution(ident, substitution))
        .map(|ident| resolve_transparent_ident(ident, ident_index))
        .map(|ident| proto_ident_type_name(ident, package_name, ident_index));

    match kind {
        WrapperKind::Option
        | WrapperKind::Vec
        | WrapperKind::VecDeque
        | WrapperKind::HashSet
        | WrapperKind::BTreeSet
        | WrapperKind::Box
        | WrapperKind::Arc
        | WrapperKind::Mutex
        | WrapperKind::ArcSwap
        | WrapperKind::ArcSwapOption
        | WrapperKind::CachePadded => inner.or_else(|| {
            let fallback = resolve_transparent_ident(ident, ident_index);
            Some(proto_ident_type_name(fallback, package_name, ident_index))
        }),
        WrapperKind::HashMap | WrapperKind::BTreeMap => None,
    }
}

fn method_map_type_name(
    generic_args: &[&ProtoIdent],
    package_name: &str,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    substitution: Option<&BTreeMap<&str, ProtoIdent>>,
) -> Option<String> {
    let key = generic_args.first().copied().copied()?;
    let value = generic_args.get(1).copied().copied()?;
    let key_ident = resolve_transparent_ident(apply_substitution(key, substitution), ident_index);
    let value_ident = resolve_transparent_ident(apply_substitution(value, substitution), ident_index);
    let key_type = proto_ident_type_name(key_ident, package_name, ident_index);
    let value_type = proto_ident_type_name(value_ident, package_name, ident_index);
    Some(format!("map<{key_type}, {value_type}>"))
}

fn wrapper_inner_type_name(
    field: &Field,
    package_name: &str,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    substitution: Option<&BTreeMap<&str, ProtoIdent>>,
) -> Option<String> {
    let kind = wrapper_kind_for(field.wrapper, field.proto_ident)?;
    if matches!(kind, WrapperKind::HashMap | WrapperKind::BTreeMap) {
        return None;
    }

    let ident = field
        .generic_args
        .first()
        .copied()
        .copied()
        .map(|ident| apply_substitution(ident, substitution))
        .map(|ident| resolve_transparent_ident(ident, ident_index))
        .map(|ident| proto_ident_type_name(ident, package_name, ident_index));

    match kind {
        WrapperKind::Option
        | WrapperKind::Vec
        | WrapperKind::VecDeque
        | WrapperKind::HashSet
        | WrapperKind::BTreeSet
        | WrapperKind::Box
        | WrapperKind::Arc
        | WrapperKind::Mutex
        | WrapperKind::ArcSwap
        | WrapperKind::ArcSwapOption
        | WrapperKind::CachePadded => ident.or_else(|| {
            let fallback = resolve_transparent_ident(field.proto_ident, ident_index);
            Some(proto_ident_type_name(fallback, package_name, ident_index))
        }),
        WrapperKind::HashMap | WrapperKind::BTreeMap => None,
    }
}

fn map_wrapper_type_name(
    field: &Field,
    package_name: &str,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    substitution: Option<&BTreeMap<&str, ProtoIdent>>,
) -> Option<String> {
    let key = field.generic_args.first().copied().copied()?;
    let value = field.generic_args.get(1).copied().copied()?;
    let key_ident = resolve_transparent_ident(apply_substitution(key, substitution), ident_index);
    let value_ident = resolve_transparent_ident(apply_substitution(value, substitution), ident_index);
    let key_type = proto_ident_type_name(key_ident, package_name, ident_index);
    let value_type = proto_ident_type_name(value_ident, package_name, ident_index);
    Some(format!("map<{key_type}, {value_type}>"))
}

fn proto_ident_type_name_with_generics(
    ident: ProtoIdent,
    generic_args: &[&ProtoIdent],
    package_name: &str,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    substitution: Option<&BTreeMap<&str, ProtoIdent>>,
) -> String {
    let ident = apply_substitution(ident, substitution);
    if generic_args.is_empty() {
        return proto_ident_type_name(ident, package_name, ident_index);
    }

    // Check if proto_type already represents a specialized/concrete type
    // (e.g., proto_type="EnvelopeGoonPong" but name="Envelope")
    // If so, don't append generic args again to avoid duplication
    if ident.proto_type != ident.name {
        return proto_ident_type_name(ident, package_name, ident_index);
    }

    let mut resolved_args = Vec::new();
    for arg in generic_args {
        let resolved = apply_substitution(**arg, substitution);
        resolved_args.push(resolved);
    }

    let specialized_name = specialized_proto_name(ident, &resolved_args);
    let ident = resolve_transparent_ident(ident, ident_index);
    if ident.proto_package_name.is_empty() || ident.proto_package_name == package_name {
        specialized_name
    } else {
        format!("{}.{}", ident.proto_package_name, specialized_name)
    }
}

fn specialized_proto_name(base: ProtoIdent, args: &[ProtoIdent]) -> String {
    let mut name = base.proto_type.to_string();
    for arg in args {
        name.push_str(arg.proto_type);
    }
    name
}

fn proto_ident_type_name(ident: ProtoIdent, package_name: &str, ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>) -> String {
    let ident = resolve_transparent_ident(ident, ident_index);
    if ident.proto_package_name.is_empty() || ident.proto_package_name == package_name {
        ident.proto_type.to_string()
    } else {
        format!("{}.{}", ident.proto_package_name, ident.proto_type)
    }
}

fn apply_substitution(ident: ProtoIdent, substitution: Option<&BTreeMap<&str, ProtoIdent>>) -> ProtoIdent {
    let Some(substitution) = substitution else {
        return ident;
    };
    substitution.get(ident.proto_type).copied().unwrap_or(ident)
}

fn collect_field_imports(
    imports: &mut BTreeSet<String>,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    fields: &[&Field],
    file_name: &str,
    package_name: &str,
) -> std::io::Result<()> {
    for field in fields {
        let ident = resolve_transparent_ident(field.proto_ident, ident_index);
        collect_proto_ident_imports(imports, ident_index, &ident, file_name, package_name)?;
        for arg in field.generic_args {
            let arg_ident = resolve_transparent_ident(**arg, ident_index);
            collect_proto_ident_imports(imports, ident_index, &arg_ident, file_name, package_name)?;
        }
    }
    Ok(())
}

fn collect_service_imports(
    imports: &mut BTreeSet<String>,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    methods: &[&ServiceMethod],
    file_name: &str,
    package_name: &str,
) -> std::io::Result<()> {
    for method in methods {
        let request = resolve_transparent_ident(method.request, ident_index);
        let response = resolve_transparent_ident(method.response, ident_index);
        collect_proto_ident_imports(imports, ident_index, &request, file_name, package_name)?;
        collect_proto_ident_imports(imports, ident_index, &response, file_name, package_name)?;
        for arg in method.request_generic_args {
            let arg_ident = resolve_transparent_ident(**arg, ident_index);
            collect_proto_ident_imports(imports, ident_index, &arg_ident, file_name, package_name)?;
        }
        for arg in method.response_generic_args {
            let arg_ident = resolve_transparent_ident(**arg, ident_index);
            collect_proto_ident_imports(imports, ident_index, &arg_ident, file_name, package_name)?;
        }
    }
    Ok(())
}

fn collect_proto_ident_imports(
    imports: &mut BTreeSet<String>,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    ident: &ProtoIdent,
    file_name: &str,
    package_name: &str,
) -> std::io::Result<()> {
    if ident.proto_file_path.is_empty() {
        return Ok(());
    }

    if ident.proto_file_path == file_name {
        return Ok(());
    }

    if ident.proto_package_name.is_empty() && ident.proto_file_path.is_empty() {
        return Ok(());
    }

    if ident.proto_package_name != package_name || ident.proto_file_path != file_name {
        if !ident.module_path.is_empty() && !ident_index.contains_key(ident) {
            return Err(std::io::Error::other(format!(
                "unresolved ProtoIdent for {} (file: {}, package: {})",
                ident.proto_type, ident.proto_file_path, ident.proto_package_name
            )));
        }
        imports.insert(ident.proto_file_path.to_string());
    }

    Ok(())
}
