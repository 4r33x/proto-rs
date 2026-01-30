use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::Field;
use super::ProtoEntry;
use super::ProtoIdent;
use super::ProtoLabel;
use super::ProtoSchema;
use super::ProtoType;
use super::ServiceMethod;
use super::Variant;
use super::utils::WrapperKind;
use super::utils::entry_sort_key;
use super::utils::is_wrapper_schema;
use super::utils::proto_ident_base_type_name;
use super::utils::proto_map_types;
use super::utils::proto_type_name;
use super::utils::resolve_transparent_ident;
use super::utils::to_snake_case;
use super::utils::wrapper_kind_for;
use super::utils::wrapper_kind_from_schema_name;
use super::utils::wrapper_prefix_from_schema_name;

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
    ordered_entries.sort_by_key(|left| entry_sort_key(left));

    let wrapper_definitions = collect_wrapper_definitions(&ordered_entries, package_name, ident_index, specializations);

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
    rendered.extend(wrapper_definitions);
    let mut seen_proto_types = std::collections::BTreeSet::new();

    for entry in ordered_entries {
        if matches!(entry.content, ProtoEntry::Import { .. }) {
            continue;
        }

        if is_wrapper_schema(entry)
            || (matches!(entry.content, ProtoEntry::Struct { .. }) && wrapper_kind_from_schema_name(entry.id.name).is_some())
        {
            continue;
        }

        // Skip duplicate proto_types (ensures stable ordering - first occurrence wins)
        let entry_proto_type = proto_ident_base_type_name(entry.id);
        if !seen_proto_types.insert(entry_proto_type.clone()) {
            continue;
        }

        // Skip concrete variant schemas that will be rendered via specializations
        // These are identifiable by having a proto_type that differs from their name
        // and matches a specialized type
        if entry_proto_type != entry.id.name && specialized_types.contains(entry_proto_type.as_str()) {
            continue;
        }

        let specs = specializations.get(&entry.id);
        rendered.extend(render_entry(entry, package_name, ident_index, specs));
    }
    rendered
}

#[derive(Clone, Copy)]
enum WrapperInner {
    Single(ProtoIdent),
    Map { key: ProtoIdent, value: ProtoIdent },
}

fn collect_wrapper_definitions(
    entries: &[&ProtoSchema],
    package_name: &str,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    specializations: &BTreeMap<ProtoIdent, Vec<GenericSpecialization>>,
) -> Vec<String> {
    let mut definitions: BTreeMap<String, String> = BTreeMap::new();
    let existing_names: BTreeSet<String> = entries
        .iter()
        .filter_map(|entry| {
            if matches!(entry.content, ProtoEntry::Import { .. }) {
                return None;
            }
            let name = wrapper_schema_message_name(entry).unwrap_or_else(|| proto_ident_base_type_name(entry.id));
            Some(name)
        })
        .collect();

    for entry in entries {
        if matches!(entry.content, ProtoEntry::Import { .. }) {
            continue;
        }
        if is_wrapper_schema(entry)
            || (matches!(entry.content, ProtoEntry::Struct { .. }) && wrapper_kind_from_schema_name(entry.id.name).is_some())
        {
            continue;
        }
        let type_generics: Vec<&str> =
            entry.generics.iter().filter(|generic| matches!(generic.kind, super::GenericKind::Type)).map(|generic| generic.name).collect();
        let has_type_generics = !type_generics.is_empty();
        if has_type_generics {
            if let Some(specs) = specializations.get(&entry.id) {
                for spec in specs {
                    let substitution = build_substitution(&type_generics, &spec.args);
                    collect_wrapper_definitions_for_entry(
                        entry,
                        package_name,
                        ident_index,
                        Some(&substitution),
                        &existing_names,
                        &mut definitions,
                    );
                }
            }
        } else {
            collect_wrapper_definitions_for_entry(entry, package_name, ident_index, None, &existing_names, &mut definitions);
        }
    }

    definitions.into_values().collect()
}

fn collect_wrapper_definitions_for_entry(
    entry: &ProtoSchema,
    package_name: &str,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    substitution: Option<&BTreeMap<&str, ProtoIdent>>,
    existing_names: &BTreeSet<String>,
    definitions: &mut BTreeMap<String, String>,
) {
    match entry.content {
        ProtoEntry::Struct { fields } => {
            for field in fields {
                collect_wrapper_definition_for_field(field, package_name, ident_index, substitution, existing_names, definitions);
            }
        }
        ProtoEntry::ComplexEnum { variants } => {
            for variant in variants {
                for field in variant.fields {
                    collect_wrapper_definition_for_field(field, package_name, ident_index, substitution, existing_names, definitions);
                }
            }
        }
        ProtoEntry::Service { methods, .. } => {
            for method in methods {
                collect_wrapper_definition_for_method(method, package_name, ident_index, substitution, existing_names, definitions);
            }
        }
        ProtoEntry::SimpleEnum { .. } | ProtoEntry::Import { .. } => {}
    }
}

fn collect_wrapper_definition_for_field(
    field: &Field,
    package_name: &str,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    substitution: Option<&BTreeMap<&str, ProtoIdent>>,
    existing_names: &BTreeSet<String>,
    definitions: &mut BTreeMap<String, String>,
) {
    let Some(kind) = wrapper_kind_for(field.wrapper, field.proto_ident) else {
        return;
    };
    if wrapper_kind_inline_for_field(field, kind) {
        return;
    }
    let inner = wrapper_inner_for_field(field, kind, substitution);
    register_wrapper_definition(kind, inner, package_name, ident_index, existing_names, definitions);
}

fn collect_wrapper_definition_for_method(
    method: &ServiceMethod,
    package_name: &str,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    substitution: Option<&BTreeMap<&str, ProtoIdent>>,
    existing_names: &BTreeSet<String>,
    definitions: &mut BTreeMap<String, String>,
) {
    if let Some(kind) =
        wrapper_kind_for(method.request_wrapper, method.request).or_else(|| wrapper_kind_from_schema_name(method.request.name))
    {
        let inner = wrapper_inner_for_method(
            method.request,
            method.request_generic_args,
            method.request_wrapper,
            kind,
            substitution,
        );
        register_wrapper_definition(kind, inner, package_name, ident_index, existing_names, definitions);
    }
    if let Some(kind) =
        wrapper_kind_for(method.response_wrapper, method.response).or_else(|| wrapper_kind_from_schema_name(method.response.name))
    {
        let inner = wrapper_inner_for_method(
            method.response,
            method.response_generic_args,
            method.response_wrapper,
            kind,
            substitution,
        );
        register_wrapper_definition(kind, inner, package_name, ident_index, existing_names, definitions);
    }
}

fn wrapper_inner_for_field(field: &Field, kind: WrapperKind, substitution: Option<&BTreeMap<&str, ProtoIdent>>) -> Option<WrapperInner> {
    match kind {
        WrapperKind::HashMap | WrapperKind::BTreeMap => {
            let (key, value) = wrapper_map_args(field.wrapper, field.generic_args)?;
            let key = apply_substitution(key, substitution);
            let value = apply_substitution(value, substitution);
            Some(WrapperInner::Map { key, value })
        }
        _ => {
            let ident = wrapper_first_generic(field.wrapper, field.generic_args).unwrap_or(field.proto_ident);
            let ident = apply_substitution(ident, substitution);
            Some(WrapperInner::Single(ident))
        }
    }
}

fn wrapper_inner_for_method(
    ident: ProtoIdent,
    generic_args: &[&ProtoIdent],
    wrapper: Option<ProtoIdent>,
    kind: WrapperKind,
    substitution: Option<&BTreeMap<&str, ProtoIdent>>,
) -> Option<WrapperInner> {
    match kind {
        WrapperKind::HashMap | WrapperKind::BTreeMap => {
            let (key, value) = wrapper_map_args(wrapper, generic_args)?;
            let key = apply_substitution(key, substitution);
            let value = apply_substitution(value, substitution);
            Some(WrapperInner::Map { key, value })
        }
        _ => {
            let inner = wrapper_first_generic(wrapper, generic_args).unwrap_or(ident);
            let inner = apply_substitution(inner, substitution);
            Some(WrapperInner::Single(inner))
        }
    }
}

fn register_wrapper_definition(
    kind: WrapperKind,
    inner: Option<WrapperInner>,
    package_name: &str,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    existing_names: &BTreeSet<String>,
    definitions: &mut BTreeMap<String, String>,
) {
    let Some(inner) = inner else {
        return;
    };
    let name = wrapper_message_name(kind, inner, ident_index);
    if existing_names.contains(&name) || definitions.contains_key(&name) {
        return;
    }
    let definition = render_wrapper_message(&name, kind, inner, package_name, ident_index);
    definitions.insert(name, definition);
}

fn wrapper_message_name(kind: WrapperKind, inner: WrapperInner, ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>) -> String {
    let prefix = wrapper_prefix_for_kind(kind);
    match inner {
        WrapperInner::Single(ident) => {
            let segment = wrapper_type_segment(ident, ident_index);
            format!("{prefix}{segment}")
        }
        WrapperInner::Map { key, value } => {
            let key_segment = wrapper_type_segment(key, ident_index);
            let value_segment = wrapper_type_segment(value, ident_index);
            format!("{prefix}{key_segment}{value_segment}")
        }
    }
}

fn wrapper_type_segment(ident: ProtoIdent, ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>) -> String {
    let ident = resolve_transparent_ident(ident, ident_index);
    proto_type_segment(&ident.proto_type)
}

const fn wrapper_prefix_for_kind(kind: WrapperKind) -> &'static str {
    match kind {
        WrapperKind::Option => "Option",
        WrapperKind::Vec => "Vec",
        WrapperKind::VecDeque => "VecDeque",
        WrapperKind::HashMap => "HashMap",
        WrapperKind::BTreeMap => "BTreeMap",
        WrapperKind::HashSet => "HashSet",
        WrapperKind::BTreeSet => "BTreeSet",
        WrapperKind::Box => "Box",
        WrapperKind::Arc => "Arc",
        WrapperKind::Mutex => "Mutex",
        WrapperKind::ArcSwap => "ArcSwap",
        WrapperKind::ArcSwapOption => "ArcSwapOption",
        WrapperKind::CachePadded => "CachePadded",
    }
}

fn render_wrapper_message(
    name: &str,
    kind: WrapperKind,
    inner: WrapperInner,
    package_name: &str,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
) -> String {
    let (label, field_type) = match inner {
        WrapperInner::Single(ident) => {
            let ident = resolve_transparent_ident(ident, ident_index);
            let field_type = proto_ident_type_name(ident, package_name, ident_index);
            let label = match kind {
                WrapperKind::Option | WrapperKind::ArcSwapOption => "optional ",
                WrapperKind::Vec | WrapperKind::VecDeque | WrapperKind::HashSet | WrapperKind::BTreeSet => "repeated ",
                _ => "",
            };
            (label, field_type)
        }
        WrapperInner::Map { key, value } => {
            let key = resolve_transparent_ident(key, ident_index);
            let value = resolve_transparent_ident(value, ident_index);
            let key_type = proto_ident_type_name(key, package_name, ident_index);
            let value_type = proto_ident_type_name(value, package_name, ident_index);
            ("", format!("map<{key_type}, {value_type}>"))
        }
    };
    format!("message {name} {{\n  {label}{field_type} value = 1;\n}}\n")
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

    let entry_name = wrapper_schema_message_name(entry).unwrap_or_else(|| proto_ident_base_type_name(entry.id));
    let definition = match entry.content {
        ProtoEntry::Struct { fields } => render_struct(&entry_name, fields, package_name, ident_index, None),
        ProtoEntry::SimpleEnum { variants } => render_simple_enum(&entry_name, variants),
        ProtoEntry::ComplexEnum { variants } => render_complex_enum(&entry_name, variants, package_name, ident_index, None),
        ProtoEntry::Import { .. } => return Vec::new(),
        ProtoEntry::Service { methods, .. } => render_service(&entry_name, methods, package_name, ident_index, None),
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
    let label = match proto_label_for_field(field) {
        ProtoLabel::None => "",
        ProtoLabel::Optional => "optional ",
        ProtoLabel::Repeated => "repeated ",
    };
    let proto_type = field_type_name(field, package_name, ident_index, substitution);
    format!("  {label}{proto_type} {name} = {};", field.tag)
}

const fn proto_label_for_field(field: &Field) -> ProtoLabel {
    field.proto_label
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
    if let Some(wrapper_type) = wrapper_message_type_name_for_field(field, ident_index, substitution) {
        return wrapper_type;
    }

    let ident = resolve_transparent_ident(field.proto_ident, ident_index);
    if proto_map_types(&ident.proto_type).is_some() {
        return proto_type_name(&ident.proto_type);
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
    if let Some(wrapper_type) = wrapper_message_type_name_for_method(ident, generic_args, wrapper, ident_index, substitution) {
        return wrapper_type;
    }
    if let Some(wrapper_name) = method_wrapper_schema_type_name(ident, package_name, ident_index) {
        return wrapper_name;
    }

    proto_ident_type_name_with_generics(ident, generic_args, package_name, ident_index, substitution)
}

fn wrapper_message_type_name_for_field(
    field: &Field,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    substitution: Option<&BTreeMap<&str, ProtoIdent>>,
) -> Option<String> {
    let kind = wrapper_kind_for(field.wrapper, field.proto_ident)?;
    if wrapper_kind_inline_for_field(field, kind) {
        return None;
    }
    let inner = wrapper_inner_for_field(field, kind, substitution)?;
    Some(wrapper_message_name(kind, inner, ident_index))
}

fn wrapper_message_type_name_for_method(
    ident: ProtoIdent,
    generic_args: &[&ProtoIdent],
    wrapper: Option<ProtoIdent>,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    substitution: Option<&BTreeMap<&str, ProtoIdent>>,
) -> Option<String> {
    let kind = wrapper_kind_for(wrapper, ident).or_else(|| wrapper_kind_from_schema_name(ident.name))?;
    let inner = wrapper_inner_for_method(ident, generic_args, wrapper, kind, substitution)?;
    Some(wrapper_message_name(kind, inner, ident_index))
}

fn method_wrapper_schema_type_name(
    ident: ProtoIdent,
    package_name: &str,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
) -> Option<String> {
    let schema = ident_index.get(&ident)?;
    let name = wrapper_schema_message_name(schema)?;
    if ident.proto_package_name.is_empty() || ident.proto_package_name == package_name {
        Some(name)
    } else {
        Some(format!("{}.{}", ident.proto_package_name, name))
    }
}

fn wrapper_first_generic(wrapper: Option<ProtoIdent>, generic_args: &[&ProtoIdent]) -> Option<ProtoIdent> {
    wrapper.and_then(|ident| ident.generics.first().copied()).or_else(|| generic_args.first().copied().copied())
}

fn wrapper_map_args(wrapper: Option<ProtoIdent>, generic_args: &[&ProtoIdent]) -> Option<(ProtoIdent, ProtoIdent)> {
    wrapper
        .and_then(|ident| match ident.generics {
            [key, value, ..] => Some((*key, *value)),
            _ => None,
        })
        .or_else(|| {
            let key = generic_args.first().copied().copied()?;
            let value = generic_args.get(1).copied().copied()?;
            Some((key, value))
        })
}

fn wrapper_schema_message_name(schema: &ProtoSchema) -> Option<String> {
    let kind = wrapper_kind_from_schema_name(schema.id.name)?;
    let fields = match schema.content {
        ProtoEntry::Struct { fields } if fields.len() == 1 => fields,
        _ => return None,
    };
    let field = fields[0];
    let prefix = wrapper_prefix_from_schema_name(schema.id.name)?;

    match kind {
        WrapperKind::HashMap | WrapperKind::BTreeMap => {
            let (key, value) = proto_map_types(&field.proto_ident.proto_type)?;
            Some(format!("{prefix}{}{}", proto_type_segment(key), proto_type_segment(value)))
        }
        WrapperKind::HashSet | WrapperKind::BTreeSet => {
            let elem = match field.proto_ident.proto_type {
                ProtoType::Optional(inner) | ProtoType::Repeated(inner) => inner,
                _ => &field.proto_ident.proto_type,
            };
            Some(format!("{prefix}{}", proto_type_segment(elem)))
        }
        WrapperKind::Option
        | WrapperKind::Vec
        | WrapperKind::VecDeque
        | WrapperKind::Box
        | WrapperKind::Arc
        | WrapperKind::Mutex
        | WrapperKind::ArcSwap
        | WrapperKind::ArcSwapOption
        | WrapperKind::CachePadded => {
            let segment = proto_type_segment(&field.proto_ident.proto_type);
            Some(format!("{prefix}{segment}"))
        }
    }
}

fn wrapper_kind_inline_for_field(field: &Field, kind: WrapperKind) -> bool {
    match kind {
        WrapperKind::Option | WrapperKind::ArcSwapOption => matches!(field.proto_label, ProtoLabel::Optional),
        WrapperKind::Vec | WrapperKind::VecDeque | WrapperKind::HashSet | WrapperKind::BTreeSet => {
            matches!(field.proto_label, ProtoLabel::Repeated)
        }
        WrapperKind::HashMap | WrapperKind::BTreeMap => proto_map_types(&field.proto_ident.proto_type).is_some(),
        WrapperKind::Box | WrapperKind::Arc | WrapperKind::Mutex | WrapperKind::ArcSwap | WrapperKind::CachePadded => true,
    }
}

fn proto_type_segment(proto_type: &ProtoType) -> String {
    match proto_type {
        ProtoType::Message(name) => (*name).to_string(),
        ProtoType::Enum => "Enum".to_string(),
        ProtoType::Optional(inner) | ProtoType::Repeated(inner) => proto_type_segment(inner),
        ProtoType::Double => "F64".to_string(),
        ProtoType::Float => "F32".to_string(),
        ProtoType::Int32 | ProtoType::Sint32 | ProtoType::Sfixed32 => "I32".to_string(),
        ProtoType::Int64 | ProtoType::Sint64 | ProtoType::Sfixed64 => "I64".to_string(),
        ProtoType::Uint32 | ProtoType::Fixed32 => "U32".to_string(),
        ProtoType::Uint64 | ProtoType::Fixed64 => "U64".to_string(),
        ProtoType::Bool => "Bool".to_string(),
        ProtoType::Bytes => "Bytes".to_string(),
        ProtoType::String => "String".to_string(),
        ProtoType::Map { key, value } => format!("Map{}{}", proto_type_segment(key), proto_type_segment(value)),
        ProtoType::None => "None".to_string(),
    }
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
    if proto_ident_base_type_name(ident) != ident.name {
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
    let mut name = proto_ident_base_type_name(base);
    for arg in args {
        name.push_str(&proto_ident_base_type_name(*arg));
    }
    name
}

fn proto_ident_type_name(ident: ProtoIdent, package_name: &str, ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>) -> String {
    let ident = resolve_transparent_ident(ident, ident_index);
    if ident.proto_package_name.is_empty() || ident.proto_package_name == package_name {
        proto_ident_base_type_name(ident)
    } else {
        format!("{}.{}", ident.proto_package_name, proto_ident_base_type_name(ident))
    }
}

fn apply_substitution(ident: ProtoIdent, substitution: Option<&BTreeMap<&str, ProtoIdent>>) -> ProtoIdent {
    let Some(substitution) = substitution else {
        return ident;
    };
    substitution.get(proto_ident_base_type_name(ident).as_str()).copied().unwrap_or(ident)
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
                proto_ident_base_type_name(*ident),
                ident.proto_file_path,
                ident.proto_package_name
            )));
        }
        imports.insert(ident.proto_file_path.to_string());
    }

    Ok(())
}
