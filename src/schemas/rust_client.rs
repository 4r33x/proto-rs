use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt::Write;
use std::fs;
use std::io;
use std::path::Path;

use super::AttrLevel;
use super::Field;
use super::GenericKind;
use super::MethodReplace;
use super::ProtoEntry;
use super::ProtoIdent;
use super::ProtoLabel;
use super::ProtoSchema;
use super::ServiceMethod;
use super::TypeReplace;
use super::UserAttr;
use super::Variant;
use super::utils::indent_line;
use super::utils::module_path_for_package;
use super::utils::module_path_segments;
use super::utils::parse_map_types;
use super::utils::proto_scalar_type;
use super::utils::resolve_transparent_ident;
use super::utils::rust_type_name;
use super::utils::to_snake_case;

#[derive(Clone, Debug)]
pub(crate) struct ClientImport {
    pub(crate) path: String,
    pub(crate) type_name: String,
    pub(crate) alias: Option<String>,
}

impl ClientImport {
    pub(crate) fn render_use(&self) -> String {
        match &self.alias {
            Some(alias) => format!("{} as {}", self.path, alias),
            None => self.path.clone(),
        }
    }

    pub(crate) fn render_type(&self) -> String {
        self.alias.as_deref().unwrap_or(&self.type_name).to_string()
    }
}

pub(crate) fn write_rust_client_module(
    output_path: &str,
    imports: &[&str],
    client_attrs: &BTreeMap<ProtoIdent, Vec<UserAttr>>,
    module_attrs: &BTreeMap<String, Vec<String>>,
    statements: &BTreeMap<String, Vec<String>>,
    type_replacements: &BTreeMap<ProtoIdent, Vec<TypeReplace>>,
    registry: &BTreeMap<String, Vec<&'static ProtoSchema>>,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
) -> io::Result<()> {
    let client_imports = parse_client_imports(imports);
    let client_imports_by_type = client_imports.iter().map(|import| (import.type_name.clone(), import.clone())).collect::<BTreeMap<_, _>>();
    let mut package_by_ident = BTreeMap::new();
    let mut root = ModuleNode::default();
    let proto_type_index = build_proto_type_index(registry);

    for (file_name, entries) in registry {
        let package_name = package_name_for_entries(file_name, entries);
        let module_segments = module_path_segments(&package_name);
        for entry in entries {
            package_by_ident.insert(entry.id, package_name.clone());
            if client_imports_by_type.contains_key(&rust_type_name(entry.id)) {
                continue;
            }
            if matches!(entry.content, ProtoEntry::Import { .. }) {
                continue;
            }
            insert_module_entry(&mut root, &module_segments, &package_name, entry);
        }
    }

    let mut output = String::new();
    output.push_str("//CODEGEN BELOW - DO NOT TOUCH ME\n");

    if !root.entries.is_empty() {
        output.push_str("#[allow(unused_imports)]\n");
        output.push_str("use proto_rs::{proto_message, proto_rpc};\n");
        render_module_imports(
            &mut output,
            &root.entries,
            root.package_name.as_deref().unwrap_or(""),
            ident_index,
            &package_by_ident,
            &proto_type_index,
            &client_imports_by_type,
            0,
        );
        output.push('\n');
        render_entries(
            &mut output,
            &root.entries,
            root.package_name.as_deref().unwrap_or(""),
            ident_index,
            &package_by_ident,
            &proto_type_index,
            &client_imports_by_type,
            client_attrs,
            type_replacements,
            0,
        );
        output.push('\n');
    }

    for (name, child) in &root.children {
        render_named_module(
            &mut output,
            name,
            child,
            0,
            ident_index,
            &package_by_ident,
            &proto_type_index,
            &client_imports_by_type,
            client_attrs,
            type_replacements,
            module_attrs,
            statements,
        );
    }

    if let Some(parent) = Path::new(output_path).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output_path, output)?;
    Ok(())
}

fn parse_client_imports(imports: &[&str]) -> Vec<ClientImport> {
    imports.iter().filter_map(|import| parse_client_import(import)).collect()
}

fn parse_client_import(import: &str) -> Option<ClientImport> {
    let mut trimmed = import.trim().trim_end_matches(';').trim();
    if let Some(stripped) = trimmed.strip_prefix("use ") {
        trimmed = stripped.trim();
    }
    if trimmed.is_empty() {
        return None;
    }
    let (path, alias) = if let Some((left, right)) = trimmed.split_once(" as ") {
        (left.trim(), Some(right.trim()))
    } else {
        (trimmed, None)
    };
    let type_name = alias.map(str::to_string).or_else(|| path.split("::").last().map(ToString::to_string))?;
    Some(ClientImport {
        path: path.to_string(),
        type_name,
        alias: alias.map(ToString::to_string),
    })
}

#[derive(Default)]
struct ModuleNode {
    package_name: Option<String>,
    entries: Vec<&'static ProtoSchema>,
    children: BTreeMap<String, ModuleNode>,
}

fn insert_module_entry(node: &mut ModuleNode, segments: &[String], package_name: &str, entry: &'static ProtoSchema) {
    if segments.is_empty() {
        node.package_name = Some(package_name.to_string());
        node.entries.push(entry);
        return;
    }
    let child = node.children.entry(segments[0].clone()).or_default();
    insert_module_entry(child, &segments[1..], package_name, entry);
}

fn render_module_attributes(output: &mut String, name: &str, module_attrs: &BTreeMap<String, Vec<String>>, indent: usize) {
    let Some(attrs) = module_attrs.get(name) else {
        return;
    };
    let mut seen = BTreeSet::new();
    for attr in attrs {
        if seen.insert(attr.clone()) {
            indent_line(output, indent);
            output.push_str(attr);
            output.push('\n');
        }
    }
}

fn render_module_statements(output: &mut String, name: &str, statements: &BTreeMap<String, Vec<String>>, indent: usize) {
    let Some(statements) = statements.get(name) else {
        return;
    };
    let mut seen = BTreeSet::new();
    for statement in statements {
        if seen.insert(statement.clone()) {
            indent_line(output, indent);
            output.push_str(statement);
            if !statement.trim_end().ends_with(';') {
                output.push(';');
            }
            output.push('\n');
        }
    }
    output.push('\n');
}

#[allow(clippy::too_many_arguments)]
fn render_named_module(
    output: &mut String,
    name: &str,
    node: &ModuleNode,
    indent: usize,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    package_by_ident: &BTreeMap<ProtoIdent, String>,
    proto_type_index: &BTreeMap<String, Vec<ProtoIdent>>,
    client_imports: &BTreeMap<String, ClientImport>,
    client_attrs: &BTreeMap<ProtoIdent, Vec<UserAttr>>,
    type_replacements: &BTreeMap<ProtoIdent, Vec<TypeReplace>>,
    module_attrs: &BTreeMap<String, Vec<String>>,
    statements: &BTreeMap<String, Vec<String>>,
) {
    render_module_attributes(output, name, module_attrs, indent);
    indent_line(output, indent);
    output.push_str("pub mod ");
    output.push_str(name);
    output.push_str(" {\n");

    let inner_indent = indent + 4;
    if !node.entries.is_empty() {
        indent_line(output, inner_indent);
        output.push_str("#[allow(unused_imports)]\n");
        indent_line(output, inner_indent);
        output.push_str("use proto_rs::{proto_message, proto_rpc};\n");
        render_module_imports(
            output,
            &node.entries,
            node.package_name.as_deref().unwrap_or(""),
            ident_index,
            package_by_ident,
            proto_type_index,
            client_imports,
            inner_indent,
        );
        output.push('\n');
    }

    render_module_statements(output, name, statements, inner_indent);
    render_entries(
        output,
        &node.entries,
        node.package_name.as_deref().unwrap_or(""),
        ident_index,
        package_by_ident,
        proto_type_index,
        client_imports,
        client_attrs,
        type_replacements,
        inner_indent,
    );

    for (child_name, child) in &node.children {
        render_named_module(
            output,
            child_name,
            child,
            inner_indent,
            ident_index,
            package_by_ident,
            proto_type_index,
            client_imports,
            client_attrs,
            type_replacements,
            module_attrs,
            statements,
        );
    }

    indent_line(output, indent);
    output.push_str("}\n");
}

#[allow(clippy::too_many_arguments)]
fn render_module_imports(
    output: &mut String,
    entries: &[&'static ProtoSchema],
    package_name: &str,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    package_by_ident: &BTreeMap<ProtoIdent, String>,
    proto_type_index: &BTreeMap<String, Vec<ProtoIdent>>,
    client_imports: &BTreeMap<String, ClientImport>,
    indent: usize,
) {
    let imports = collect_module_imports(
        entries,
        package_name,
        ident_index,
        package_by_ident,
        proto_type_index,
        client_imports,
    );
    for import in imports {
        indent_line(output, indent);
        output.push_str("use ");
        output.push_str(&import);
        output.push_str(";\n");
    }
}

fn collect_module_imports(
    entries: &[&'static ProtoSchema],
    package_name: &str,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    package_by_ident: &BTreeMap<ProtoIdent, String>,
    proto_type_index: &BTreeMap<String, Vec<ProtoIdent>>,
    client_imports: &BTreeMap<String, ClientImport>,
) -> BTreeSet<String> {
    let mut imports = BTreeSet::new();
    for entry in entries {
        match entry.content {
            ProtoEntry::Struct { fields } => {
                for field in fields {
                    collect_rust_field_imports(
                        field,
                        package_name,
                        ident_index,
                        package_by_ident,
                        proto_type_index,
                        client_imports,
                        &mut imports,
                    );
                }
            }
            ProtoEntry::ComplexEnum { variants } => {
                for variant in variants {
                    for field in variant.fields {
                        collect_rust_field_imports(
                            field,
                            package_name,
                            ident_index,
                            package_by_ident,
                            proto_type_index,
                            client_imports,
                            &mut imports,
                        );
                    }
                }
            }
            ProtoEntry::Service { methods, .. } => {
                for method in methods {
                    let request = resolve_transparent_ident(method.request, ident_index);
                    let response = resolve_transparent_ident(method.response, ident_index);
                    collect_rust_proto_ident_imports(
                        request,
                        package_name,
                        package_by_ident,
                        proto_type_index,
                        client_imports,
                        &mut imports,
                    );
                    collect_rust_proto_ident_imports(
                        response,
                        package_name,
                        package_by_ident,
                        proto_type_index,
                        client_imports,
                        &mut imports,
                    );
                    for arg in method.request_generic_args {
                        collect_rust_proto_ident_imports(
                            resolve_transparent_ident(**arg, ident_index),
                            package_name,
                            package_by_ident,
                            proto_type_index,
                            client_imports,
                            &mut imports,
                        );
                    }
                    for arg in method.response_generic_args {
                        collect_rust_proto_ident_imports(
                            resolve_transparent_ident(**arg, ident_index),
                            package_name,
                            package_by_ident,
                            proto_type_index,
                            client_imports,
                            &mut imports,
                        );
                    }
                }
            }
            ProtoEntry::SimpleEnum { .. } | ProtoEntry::Import { .. } => {}
        }
    }
    imports
}

fn collect_rust_field_imports(
    field: &Field,
    package_name: &str,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    package_by_ident: &BTreeMap<ProtoIdent, String>,
    proto_type_index: &BTreeMap<String, Vec<ProtoIdent>>,
    client_imports: &BTreeMap<String, ClientImport>,
    imports: &mut BTreeSet<String>,
) {
    let ident = resolve_transparent_ident(field.rust_proto_ident, ident_index);
    collect_rust_proto_ident_imports(ident, package_name, package_by_ident, proto_type_index, client_imports, imports);
    for arg in field.generic_args {
        let arg = resolve_transparent_ident(**arg, ident_index);
        collect_rust_proto_ident_imports(arg, package_name, package_by_ident, proto_type_index, client_imports, imports);
    }
}

fn collect_rust_proto_ident_imports(
    ident: ProtoIdent,
    package_name: &str,
    package_by_ident: &BTreeMap<ProtoIdent, String>,
    proto_type_index: &BTreeMap<String, Vec<ProtoIdent>>,
    client_imports: &BTreeMap<String, ClientImport>,
    imports: &mut BTreeSet<String>,
) {
    if ident.proto_type.starts_with("map<") {
        if let Some((key, value)) = parse_map_types(ident.proto_type) {
            collect_rust_proto_name_imports(key, package_name, package_by_ident, proto_type_index, client_imports, imports);
            collect_rust_proto_name_imports(value, package_name, package_by_ident, proto_type_index, client_imports, imports);
        }
        return;
    }

    let type_name = rust_type_name(ident);
    if let Some(import) = client_imports.get(&type_name) {
        imports.insert(import.render_use());
        return;
    }

    let package = package_by_ident.get(&ident).map(String::as_str).or(if ident.proto_package_name.is_empty() {
        None
    } else {
        Some(ident.proto_package_name)
    });

    if let Some(package) = package
        && !package.is_empty()
        && package != package_name
    {
        imports.insert(format!("crate::{}::{}", module_path_for_package(package), type_name));
    }
}

fn collect_rust_proto_name_imports(
    proto_name: &str,
    package_name: &str,
    package_by_ident: &BTreeMap<ProtoIdent, String>,
    proto_type_index: &BTreeMap<String, Vec<ProtoIdent>>,
    client_imports: &BTreeMap<String, ClientImport>,
    imports: &mut BTreeSet<String>,
) {
    if proto_scalar_type(proto_name).is_some() {
        return;
    }
    if proto_name.starts_with("map<") {
        if let Some((key, value)) = parse_map_types(proto_name) {
            collect_rust_proto_name_imports(key, package_name, package_by_ident, proto_type_index, client_imports, imports);
            collect_rust_proto_name_imports(value, package_name, package_by_ident, proto_type_index, client_imports, imports);
        }
        return;
    }
    if let Some(candidates) = proto_type_index.get(proto_name) {
        if let Some(candidate) = candidates.iter().find(|ident| package_by_ident.get(*ident).is_some_and(|pkg| pkg == package_name)) {
            collect_rust_proto_ident_imports(
                *candidate,
                package_name,
                package_by_ident,
                proto_type_index,
                client_imports,
                imports,
            );
            return;
        }
        if let Some(candidate) = candidates.first() {
            collect_rust_proto_ident_imports(
                *candidate,
                package_name,
                package_by_ident,
                proto_type_index,
                client_imports,
                imports,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_entries(
    output: &mut String,
    entries: &[&'static ProtoSchema],
    package_name: &str,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    package_by_ident: &BTreeMap<ProtoIdent, String>,
    proto_type_index: &BTreeMap<String, Vec<ProtoIdent>>,
    client_imports: &BTreeMap<String, ClientImport>,
    client_attrs: &BTreeMap<ProtoIdent, Vec<UserAttr>>,
    type_replacements: &BTreeMap<ProtoIdent, Vec<TypeReplace>>,
    indent: usize,
) {
    if entries.is_empty() {
        return;
    }
    let mut ordered_entries = entries.to_vec();
    ordered_entries.sort_by(|left, right| super::utils::entry_sort_key(left).cmp(&super::utils::entry_sort_key(right)));

    // Group entries by Rust type name to handle generic types with concrete variants
    let mut entries_by_name: BTreeMap<String, Vec<&ProtoSchema>> = BTreeMap::new();
    for entry in ordered_entries {
        let type_name = rust_type_name(entry.id);
        entries_by_name.entry(type_name).or_default().push(entry);
    }

    // For each unique type name (BTreeMap ensures stable alphabetical ordering),
    // prefer the generic version over concrete variants
    for (_type_name, group) in entries_by_name {
        // If there are multiple schemas with the same name, prefer the one with generics
        // (the base generic type) over concrete variants (which have empty generics but
        // different proto_type like "EnvelopeBuildRequest" vs "Envelope")
        //
        // Stable selection: prefer in order:
        // 1. Entry with non-empty generics (base generic type)
        // 2. Entry where proto_type matches name (non-generic or original type)
        // 3. First entry (fallback for consistent ordering)
        let entry = if group.len() > 1 {
            group
                .iter()
                .find(|e| !e.generics.is_empty())
                .or_else(|| group.iter().find(|e| e.id.proto_type == e.id.name))
                .unwrap_or(&group[0])
        } else {
            group[0]
        };

        let user_attrs = build_entry_user_attrs(entry, client_attrs, ident_index);
        let entry_type_replacements = build_entry_type_replacements(entry, type_replacements);
        if let Some(definition) = render_rust_entry(
            entry,
            package_name,
            ident_index,
            package_by_ident,
            proto_type_index,
            client_imports,
            &user_attrs,
            &entry_type_replacements,
            indent,
        ) {
            output.push_str(&definition);
            output.push('\n');
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_rust_entry(
    entry: &ProtoSchema,
    package_name: &str,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    package_by_ident: &BTreeMap<ProtoIdent, String>,
    proto_type_index: &BTreeMap<String, Vec<ProtoIdent>>,
    client_imports: &BTreeMap<String, ClientImport>,
    user_attrs: &EntryUserAttrs,
    type_replacements: &EntryTypeReplacements,
    indent: usize,
) -> Option<String> {
    match entry.content {
        ProtoEntry::Struct { fields } => Some(render_rust_struct(
            entry,
            fields,
            package_name,
            ident_index,
            package_by_ident,
            proto_type_index,
            client_imports,
            user_attrs,
            type_replacements,
            indent,
        )),
        ProtoEntry::SimpleEnum { variants } => Some(render_rust_simple_enum(entry, variants, user_attrs, indent)),
        ProtoEntry::ComplexEnum { variants } => Some(render_rust_complex_enum(
            entry,
            variants,
            package_name,
            ident_index,
            package_by_ident,
            proto_type_index,
            client_imports,
            user_attrs,
            type_replacements,
            indent,
        )),
        ProtoEntry::Import { .. } => None,
        ProtoEntry::Service { methods, rpc_package_name } => Some(render_rust_service(
            entry,
            methods,
            rpc_package_name,
            package_name,
            ident_index,
            package_by_ident,
            proto_type_index,
            client_imports,
            user_attrs,
            type_replacements,
            indent,
        )),
    }
}

#[allow(clippy::too_many_arguments)]
fn render_rust_struct(
    entry: &ProtoSchema,
    fields: &[&Field],
    package_name: &str,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    package_by_ident: &BTreeMap<ProtoIdent, String>,
    proto_type_index: &BTreeMap<String, Vec<ProtoIdent>>,
    client_imports: &BTreeMap<String, ClientImport>,
    user_attrs: &EntryUserAttrs,
    type_replacements: &EntryTypeReplacements,
    indent: usize,
) -> String {
    let mut output = String::new();
    let type_name = rust_type_name(entry.id);
    let generics = render_generics(entry);
    let is_tuple = fields.iter().all(|field| field.name.is_none());

    render_top_level_attributes(&mut output, entry, user_attrs, indent);

    indent_line(&mut output, indent);
    if fields.is_empty() {
        output.write_fmt(format_args!("pub struct {type_name}{generics};\n")).unwrap();
        return output;
    }

    if is_tuple {
        output.write_fmt(format_args!("pub struct {type_name}{generics}(\n")).unwrap();

        for (idx, field) in fields.iter().enumerate() {
            let (field_attrs, field_overrides) = field.name.map_or((Vec::new(), BTreeSet::new()), |name| {
                collect_field_attr_data(user_attrs, None, name)
            });
            render_field_attributes(&mut output, field, idx, &field_attrs, &field_overrides, indent + 4);
            indent_line(&mut output, indent + 4);
            output.push_str("pub ");
            let type_replacement = field.name.and_then(|name| lookup_field_replacement(type_replacements, None, name));
            output.push_str(&render_field_type(
                field,
                package_name,
                ident_index,
                package_by_ident,
                proto_type_index,
                client_imports,
                type_replacement,
            ));
            output.push_str(",\n");
        }
        indent_line(&mut output, indent);
        output.push_str(");\n");
        return output;
    }
    output.write_fmt(format_args!("pub struct {type_name}{generics} {{\n")).unwrap();

    for (idx, field) in fields.iter().enumerate() {
        let (field_attrs, field_overrides) = field.name.map_or((Vec::new(), BTreeSet::new()), |name| {
            collect_field_attr_data(user_attrs, None, name)
        });
        render_field_attributes(&mut output, field, idx, &field_attrs, &field_overrides, indent + 4);
        indent_line(&mut output, indent + 4);
        let name = field.name.unwrap_or("field");
        output.push_str("pub ");
        output.push_str(name);
        output.push_str(": ");
        let type_replacement = field.name.and_then(|name| lookup_field_replacement(type_replacements, None, name));
        output.push_str(&render_field_type(
            field,
            package_name,
            ident_index,
            package_by_ident,
            proto_type_index,
            client_imports,
            type_replacement,
        ));
        output.push_str(",\n");
    }
    indent_line(&mut output, indent);
    output.push_str("}\n");
    output
}

fn render_rust_simple_enum(entry: &ProtoSchema, variants: &[&Variant], user_attrs: &EntryUserAttrs, indent: usize) -> String {
    let mut output = String::new();
    let type_name = rust_type_name(entry.id);
    let generics = render_generics(entry);

    render_top_level_attributes(&mut output, entry, user_attrs, indent);
    indent_line(&mut output, indent);
    output.write_fmt(format_args!("pub enum {type_name}{generics} {{\n")).unwrap();

    for variant in variants {
        indent_line(&mut output, indent + 4);
        output.push_str(variant.name);
        if let Some(discriminant) = variant.discriminant {
            output.write_fmt(format_args!(" = {discriminant}")).unwrap();
        }
        output.push_str(",\n");
    }
    indent_line(&mut output, indent);
    output.push_str("}\n");
    output
}

#[allow(clippy::too_many_arguments)]
fn render_rust_complex_enum(
    entry: &ProtoSchema,
    variants: &[&Variant],
    package_name: &str,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    package_by_ident: &BTreeMap<ProtoIdent, String>,
    proto_type_index: &BTreeMap<String, Vec<ProtoIdent>>,
    client_imports: &BTreeMap<String, ClientImport>,
    user_attrs: &EntryUserAttrs,
    type_replacements: &EntryTypeReplacements,
    indent: usize,
) -> String {
    let mut output = String::new();
    let type_name = rust_type_name(entry.id);
    let generics = render_generics(entry);

    render_top_level_attributes(&mut output, entry, user_attrs, indent);
    indent_line(&mut output, indent);
    output.write_fmt(format_args!("pub enum {type_name}{generics} {{\n")).unwrap();

    for variant in variants {
        indent_line(&mut output, indent + 4);
        output.push_str(variant.name);
        if variant.fields.is_empty() {
            output.push_str(",\n");
            continue;
        }

        let has_named = variant.fields.iter().any(|field| field.name.is_some());
        if has_named {
            output.push_str(" {\n");
            for (idx, field) in variant.fields.iter().enumerate() {
                let (field_attrs, field_overrides) = field.name.map_or((Vec::new(), BTreeSet::new()), |name| {
                    collect_field_attr_data(user_attrs, Some(variant.name), name)
                });
                render_field_attributes(&mut output, field, idx, &field_attrs, &field_overrides, indent + 8);
                indent_line(&mut output, indent + 8);
                let name = field.name.unwrap_or("field");
                output.push_str(name);
                output.push_str(": ");
                let type_replacement = field
                    .name
                    .and_then(|name| lookup_field_replacement(type_replacements, Some(variant.name), name));
                output.push_str(&render_field_type(
                    field,
                    package_name,
                    ident_index,
                    package_by_ident,
                    proto_type_index,
                    client_imports,
                    type_replacement,
                ));
                output.push_str(",\n");
            }
            indent_line(&mut output, indent + 4);
            output.push_str("},\n");
        } else {
            output.push_str("(\n");
            for (idx, field) in variant.fields.iter().enumerate() {
                let (field_attrs, field_overrides) = field.name.map_or((Vec::new(), BTreeSet::new()), |name| {
                    collect_field_attr_data(user_attrs, Some(variant.name), name)
                });
                render_field_attributes(&mut output, field, idx, &field_attrs, &field_overrides, indent + 8);
                indent_line(&mut output, indent + 8);
                let type_replacement = field
                    .name
                    .and_then(|name| lookup_field_replacement(type_replacements, Some(variant.name), name));
                output.push_str(&render_field_type(
                    field,
                    package_name,
                    ident_index,
                    package_by_ident,
                    proto_type_index,
                    client_imports,
                    type_replacement,
                ));
                output.push_str(",\n");
            }
            indent_line(&mut output, indent + 4);
            output.push_str("),\n");
        }
    }
    indent_line(&mut output, indent);
    output.push_str("}\n");
    output
}

#[allow(clippy::too_many_arguments)]
fn render_rust_service(
    entry: &ProtoSchema,
    methods: &[&ServiceMethod],
    rpc_package_name: &str,
    package_name: &str,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    package_by_ident: &BTreeMap<ProtoIdent, String>,
    proto_type_index: &BTreeMap<String, Vec<ProtoIdent>>,
    client_imports: &BTreeMap<String, ClientImport>,
    user_attrs: &EntryUserAttrs,
    type_replacements: &EntryTypeReplacements,
    indent: usize,
) -> String {
    let mut output = String::new();
    let trait_name = rust_type_name(entry.id);
    let generics = render_generics(entry);

    render_service_attributes(&mut output, rpc_package_name, user_attrs, indent);
    indent_line(&mut output, indent);
    writeln!(output, "pub trait {trait_name}{generics} {{").unwrap();

    let mut stream_types = Vec::new();
    for method in methods {
        if method.server_streaming {
            let stream_name = format!("{}Stream", method.name);
            let response_ident = resolve_transparent_ident(method.response, ident_index);
            let item_type = method_type_replacement(type_replacements, method.name, MethodTypeKind::Return)
                .map(str::to_string)
                .unwrap_or_else(|| {
                    render_proto_type_with_generics(
                        response_ident,
                        method.response_generic_args,
                        package_name,
                        package_by_ident,
                        proto_type_index,
                        client_imports,
                    )
                });
            stream_types.push(stream_name.clone());
            indent_line(&mut output, indent + 4);
            writeln!(
                output,
                "type {stream_name}: ::tonic::codegen::tokio_stream::Stream<Item = ::core::result::Result<{item_type}, ::tonic::Status>> + ::core::marker::Send;"
            )
            .unwrap();
        }
    }

    if !stream_types.is_empty() {
        output.push('\n');
    }

    for method in methods {
        let request_ident = resolve_transparent_ident(method.request, ident_index);
        let request_type = method_type_replacement(type_replacements, method.name, MethodTypeKind::Argument)
            .map(str::to_string)
            .unwrap_or_else(|| {
                render_proto_type_with_generics(
                    request_ident,
                    method.request_generic_args,
                    package_name,
                    package_by_ident,
                    proto_type_index,
                    client_imports,
                )
            });
        let response_type = if method.server_streaming {
            format!("Self::{}Stream", method.name)
        } else {
            method_type_replacement(type_replacements, method.name, MethodTypeKind::Return)
                .map(str::to_string)
                .unwrap_or_else(|| {
                    let response_ident = resolve_transparent_ident(method.response, ident_index);
                    render_proto_type_with_generics(
                        response_ident,
                        method.response_generic_args,
                        package_name,
                        package_by_ident,
                        proto_type_index,
                        client_imports,
                    )
                })
        };

        render_method_attributes(&mut output, user_attrs.method_attrs.get(method.name), indent + 4);
        indent_line(&mut output, indent + 4);
        writeln!(output, "async fn {}(", to_snake_case(method.name)).unwrap();
        indent_line(&mut output, indent + 8);
        writeln!(output, "&self,").unwrap();
        indent_line(&mut output, indent + 8);
        writeln!(output, "request: ::tonic::Request<{request_type}>,").unwrap();
        indent_line(&mut output, indent + 4);
        output.push_str(") -> ::core::result::Result<::tonic::Response<");
        output.push_str(&response_type);
        output.push_str(">, ::tonic::Status>;\n\n");
    }

    indent_line(&mut output, indent);
    output.push_str("}\n");
    output
}

fn render_top_level_attributes(output: &mut String, entry: &ProtoSchema, user_attrs: &EntryUserAttrs, indent: usize) {
    let mut seen = BTreeSet::new();
    for attr in &user_attrs.top_level {
        if seen.insert(attr.clone()) {
            indent_line(output, indent);
            output.push_str(attr);
            output.push('\n');
        }
    }

    let mut has_proto_message = false;
    for attr in entry.top_level_attributes {
        if attr.path == "proto_message" {
            if user_attrs.top_level_override_paths.contains("proto_message") {
                continue;
            }
            has_proto_message = true;
            if seen.insert(attr.tokens.to_string()) {
                indent_line(output, indent);
                output.push_str(attr.tokens);
                output.push('\n');
            }
        }
    }
    if !has_proto_message && !user_attrs.top_level_override_paths.contains("proto_message") {
        let default = "#[proto_message]";
        if seen.insert(default.to_string()) {
            indent_line(output, indent);
            output.push_str(default);
            output.push('\n');
        }
    }
}

fn render_field_attributes(
    output: &mut String,
    field: &Field,
    idx: usize,
    user_attrs: &[String],
    override_paths: &BTreeSet<String>,
    indent: usize,
) {
    let mut seen = BTreeSet::new();
    for attr in user_attrs {
        if seen.insert(attr.clone()) {
            indent_line(output, indent);
            output.push_str(attr);
            output.push('\n');
        }
    }

    let expected_tag = idx as u32 + 1;
    let mut emitted = false;
    for attr in field.attributes {
        if attr.path == "proto" {
            if override_paths.contains("proto") {
                continue;
            }
            if is_tag_only_attr(attr.tokens, expected_tag) {
                continue;
            }
            emitted = true;
            if seen.insert(attr.tokens.to_string()) {
                indent_line(output, indent);
                output.push_str(attr.tokens);
                output.push('\n');
            }
        }
    }
    if !emitted && field.tag > 0 && field.tag != expected_tag && !override_paths.contains("proto") {
        indent_line(output, indent);
        output.write_fmt(format_args!("#[proto(tag = {})]\n", field.tag)).unwrap();
    }
}

fn render_service_attributes(output: &mut String, rpc_package_name: &str, user_attrs: &EntryUserAttrs, indent: usize) {
    let mut seen = BTreeSet::new();
    for attr in &user_attrs.top_level {
        if seen.insert(attr.clone()) {
            indent_line(output, indent);
            output.push_str(attr);
            output.push('\n');
        }
    }
    if !user_attrs.top_level_override_paths.contains("proto_rpc") {
        let default = format!("#[proto_rpc(rpc_package = \"{rpc_package_name}\", rpc_server = false, rpc_client = true)]");
        if seen.insert(default.clone()) {
            indent_line(output, indent);
            output.push_str(&default);
            output.push('\n');
        }
    }
}

fn render_method_attributes(output: &mut String, attrs: Option<&Vec<String>>, indent: usize) {
    let mut seen = BTreeSet::new();
    if let Some(attrs) = attrs {
        for attr in attrs {
            if seen.insert(attr.clone()) {
                indent_line(output, indent);
                output.push_str(attr);
                output.push('\n');
            }
        }
    }
}

fn is_tag_only_attr(tokens: &str, expected_tag: u32) -> bool {
    let normalized = tokens.replace(' ', "");
    let inner = normalized.strip_prefix("#[proto(").and_then(|value| value.strip_suffix(")]"));
    let Some(inner) = inner else {
        return false;
    };
    let mut parts = inner.split(',');
    let Some(first) = parts.next() else {
        return false;
    };
    if parts.next().is_some() {
        return false;
    }
    let Some(tag_value) = first.strip_prefix("tag=") else {
        return false;
    };
    tag_value.parse::<u32>().ok().is_some_and(|tag| tag == expected_tag)
}

#[derive(Default)]
struct EntryUserAttrs {
    top_level: Vec<String>,
    top_level_override_paths: BTreeSet<String>,
    field_attrs: BTreeMap<FieldTargetKey, Vec<String>>,
    field_override_paths: BTreeMap<FieldTargetKey, BTreeSet<String>>,
    method_attrs: BTreeMap<String, Vec<String>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct FieldTargetKey {
    variant: Option<String>,
    field_name: String,
}

impl FieldTargetKey {
    fn new(variant: Option<&str>, field_name: &str) -> Self {
        Self {
            variant: variant.map(str::to_string),
            field_name: field_name.to_string(),
        }
    }
}

#[derive(Default)]
struct EntryTypeReplacements {
    field_types: BTreeMap<FieldTargetKey, String>,
    method_arguments: BTreeMap<String, String>,
    method_returns: BTreeMap<String, String>,
}

#[derive(Clone, Copy)]
enum MethodTypeKind {
    Argument,
    Return,
}

fn build_entry_user_attrs(
    entry: &ProtoSchema,
    client_attrs: &BTreeMap<ProtoIdent, Vec<UserAttr>>,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
) -> EntryUserAttrs {
    let mut entry_attrs = EntryUserAttrs::default();
    let Some(attrs) = client_attrs.get(&entry.id) else {
        return entry_attrs;
    };

    for attr in attrs {
        match &attr.level {
            AttrLevel::Top => {
                if let Some(path) = parse_attr_path(&attr.attr) {
                    entry_attrs.top_level_override_paths.insert(path.to_string());
                }
                entry_attrs.top_level.push(attr.attr.clone());
            }
            AttrLevel::Field { field_name, id, variant } => {
                let matches = find_entry_field_matches(entry, field_name, variant.as_deref());
                assert!(
                    !matches.is_empty(),
                    "client attribute targets missing field '{}'{} on type '{}'",
                    field_name,
                    render_variant_suffix(variant.as_deref()),
                    entry.id.name
                );
                for field in &matches {
                    let actual_type = resolve_transparent_ident(field.rust_proto_ident, ident_index);
                    assert!(
                        actual_type == *id,
                        "client attribute targets field '{}'{} on type '{}' with mismatched type",
                        field_name,
                        render_variant_suffix(variant.as_deref()),
                        entry.id.name
                    );
                }
                let field_key = FieldTargetKey::new(variant.as_deref(), field_name);
                if let Some(path) = parse_attr_path(&attr.attr) {
                    entry_attrs.field_override_paths.entry(field_key.clone()).or_default().insert(path.to_string());
                }
                entry_attrs.field_attrs.entry(field_key).or_default().push(attr.attr.clone());
            }
            AttrLevel::Method { method_name } => {
                let Some(methods) = find_entry_methods(entry) else {
                    panic!(
                        "client attribute targets method '{}' on non-service type '{}'",
                        method_name, entry.id.name
                    );
                };
                assert!(
                    methods.iter().any(|method| method.name == method_name),
                    "client attribute targets missing method '{}' on type '{}'",
                    method_name,
                    entry.id.name
                );
                entry_attrs.method_attrs.entry(method_name.clone()).or_default().push(attr.attr.clone());
            }
        }
    }

    entry_attrs
}

fn parse_attr_path(attr: &str) -> Option<&str> {
    let trimmed = attr.trim();
    let stripped = trimmed.strip_prefix("#[")?.trim();
    let end = stripped.find(['(', ']']).unwrap_or(stripped.len());
    let path = stripped[..end].trim();
    if path.is_empty() { None } else { Some(path) }
}

fn render_variant_suffix(variant: Option<&str>) -> String {
    variant.map_or_else(String::new, |name| format!(" in variant '{name}'"))
}

fn find_entry_field_matches<'a>(entry: &'a ProtoSchema, field_name: &str, variant: Option<&str>) -> Vec<&'a Field> {
    match entry.content {
        ProtoEntry::Struct { fields } => {
            assert!(
                variant.is_none(),
                "client attribute targets variant '{}' on non-enum type '{}'",
                variant.unwrap_or_default(),
                entry.id.name
            );
            fields.iter().copied().filter(|field| field.name.is_some_and(|name| name == field_name)).collect()
        }
        ProtoEntry::ComplexEnum { variants } => {
            let selected_variants: Vec<&Variant> = match variant {
                Some(name) => {
                    let Some(target) = variants.iter().find(|variant| variant.name == name) else {
                        panic!(
                            "client attribute targets missing variant '{}' on type '{}'",
                            name, entry.id.name
                        );
                    };
                    vec![target]
                }
                None => variants.to_vec(),
            };
            selected_variants
                .iter()
                .flat_map(|variant| {
                    variant
                        .fields
                        .iter()
                        .copied()
                        .filter(|field| field.name.is_some_and(|name| name == field_name))
                })
                .collect()
        }
        ProtoEntry::SimpleEnum { .. } | ProtoEntry::Import { .. } | ProtoEntry::Service { .. } => Vec::new(),
    }
}

fn find_entry_methods(entry: &ProtoSchema) -> Option<&[&ServiceMethod]> {
    match entry.content {
        ProtoEntry::Service { methods, .. } => Some(methods),
        _ => None,
    }
}

fn build_entry_type_replacements(
    entry: &ProtoSchema,
    type_replacements: &BTreeMap<ProtoIdent, Vec<TypeReplace>>,
) -> EntryTypeReplacements {
    let mut entry_replacements = EntryTypeReplacements::default();
    let Some(replacements) = type_replacements.get(&entry.id) else {
        return entry_replacements;
    };

    for replacement in replacements {
        match replacement {
            TypeReplace::Trait { method, kind, r#type, .. } => {
                let Some(methods) = find_entry_methods(entry) else {
                    panic!(
                        "type replacement targets method '{}' on non-service type '{}'",
                        method, entry.id.name
                    );
                };
                assert!(
                    methods.iter().any(|method_entry| method_entry.name == method.as_str()),
                    "type replacement targets missing method '{}' on type '{}'",
                    method,
                    entry.id.name
                );
                let replacement_type = resolve_method_replace_type(kind, r#type).to_string();
                match kind {
                    MethodReplace::Argument(_) => {
                        entry_replacements.method_arguments.entry(method.clone()).or_insert(replacement_type);
                    }
                    MethodReplace::Return(_) => {
                        entry_replacements.method_returns.entry(method.clone()).or_insert(replacement_type);
                    }
                }
            }
            TypeReplace::Type {
                field,
                variant,
                r#type,
                ..
            } => {
                let matches = find_entry_field_matches(entry, field, variant.as_deref());
                assert!(
                    !matches.is_empty(),
                    "type replacement targets missing field '{}'{} on type '{}'",
                    field,
                    render_variant_suffix(variant.as_deref()),
                    entry.id.name
                );
                let key = FieldTargetKey::new(variant.as_deref(), field);
                entry_replacements.field_types.entry(key).or_insert_with(|| r#type.clone());
            }
        }
    }

    entry_replacements
}

fn resolve_method_replace_type<'a>(kind: &'a MethodReplace, fallback: &'a str) -> &'a str {
    match kind {
        MethodReplace::Argument(replacement) | MethodReplace::Return(replacement) if !replacement.is_empty() => replacement,
        _ => fallback,
    }
}

fn method_type_replacement<'a>(
    replacements: &'a EntryTypeReplacements,
    method_name: &str,
    kind: MethodTypeKind,
) -> Option<&'a str> {
    match kind {
        MethodTypeKind::Argument => replacements.method_arguments.get(method_name).map(String::as_str),
        MethodTypeKind::Return => replacements.method_returns.get(method_name).map(String::as_str),
    }
}

fn lookup_field_replacement<'a>(
    replacements: &'a EntryTypeReplacements,
    variant: Option<&str>,
    field_name: &str,
) -> Option<&'a str> {
    if let Some(variant) = variant {
        if let Some(replacement) = replacements.field_types.get(&FieldTargetKey::new(Some(variant), field_name)) {
            return Some(replacement);
        }
    }
    replacements.field_types.get(&FieldTargetKey::new(None, field_name)).map(String::as_str)
}

fn collect_field_attr_data(user_attrs: &EntryUserAttrs, variant: Option<&str>, field_name: &str) -> (Vec<String>, BTreeSet<String>) {
    let mut attrs = Vec::new();
    let mut overrides = BTreeSet::new();
    let global_key = FieldTargetKey::new(None, field_name);
    if let Some(values) = user_attrs.field_attrs.get(&global_key) {
        attrs.extend(values.clone());
    }
    if let Some(paths) = user_attrs.field_override_paths.get(&global_key) {
        overrides.extend(paths.iter().cloned());
    }
    if let Some(variant) = variant {
        let variant_key = FieldTargetKey::new(Some(variant), field_name);
        if let Some(values) = user_attrs.field_attrs.get(&variant_key) {
            attrs.extend(values.clone());
        }
        if let Some(paths) = user_attrs.field_override_paths.get(&variant_key) {
            overrides.extend(paths.iter().cloned());
        }
    }
    (attrs, overrides)
}

fn render_field_type(
    field: &Field,
    package_name: &str,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
    package_by_ident: &BTreeMap<ProtoIdent, String>,
    proto_type_index: &BTreeMap<String, Vec<ProtoIdent>>,
    client_imports: &BTreeMap<String, ClientImport>,
    type_replacement: Option<&str>,
) -> String {
    if let Some(array_len) = field.array_len {
        let elem_ident = field.array_elem.unwrap_or(field.proto_ident);
        let elem_type = if let Some(type_replacement) = type_replacement {
            type_replacement.to_string()
        } else if field.array_is_bytes {
            "u8".to_string()
        } else {
            render_proto_type(elem_ident, package_name, package_by_ident, proto_type_index, client_imports)
        };
        return format!("[{elem_type}; {array_len}]");
    }

    let base = if let Some(type_replacement) = type_replacement {
        type_replacement.to_string()
    } else {
        let ident = resolve_transparent_ident(field.rust_proto_ident, ident_index);
        render_proto_type_with_generics(
            ident,
            field.generic_args,
            package_name,
            package_by_ident,
            proto_type_index,
            client_imports,
        )
    };
    match field.proto_label {
        ProtoLabel::None => base,
        ProtoLabel::Optional => format!("::core::option::Option<{base}>"),
        ProtoLabel::Repeated => format!("::proto_rs::alloc::vec::Vec<{base}>"),
    }
}

fn render_proto_type(
    ident: ProtoIdent,
    current_package: &str,
    package_by_ident: &BTreeMap<ProtoIdent, String>,
    proto_type_index: &BTreeMap<String, Vec<ProtoIdent>>,
    client_imports: &BTreeMap<String, ClientImport>,
) -> String {
    if ident.proto_type.starts_with("map<") {
        return render_map_type(
            ident.proto_type,
            current_package,
            package_by_ident,
            proto_type_index,
            client_imports,
        );
    }
    if ident.module_path.is_empty()
        && ident.proto_file_path.is_empty()
        && ident.proto_package_name.is_empty()
        && let Some(scalar) = proto_scalar_type(ident.proto_type)
    {
        return scalar.to_string();
    }

    let type_name = rust_type_name(ident);
    if let Some(import) = client_imports.get(&type_name) {
        return import.render_type();
    }
    let package = package_by_ident.get(&ident).map(String::as_str).or(if ident.proto_package_name.is_empty() {
        None
    } else {
        Some(ident.proto_package_name)
    });

    match package {
        Some(package) if package == current_package => type_name,
        Some(package) if !package.is_empty() => type_name,
        _ => type_name,
    }
}

fn render_proto_type_with_generics(
    ident: ProtoIdent,
    generic_args: &[&ProtoIdent],
    current_package: &str,
    package_by_ident: &BTreeMap<ProtoIdent, String>,
    proto_type_index: &BTreeMap<String, Vec<ProtoIdent>>,
    client_imports: &BTreeMap<String, ClientImport>,
) -> String {
    let base = render_proto_type(ident, current_package, package_by_ident, proto_type_index, client_imports);
    if generic_args.is_empty() {
        return base;
    }
    let rendered_args: Vec<String> = generic_args
        .iter()
        .map(|arg| render_proto_type(**arg, current_package, package_by_ident, proto_type_index, client_imports))
        .collect();
    format!("{base}<{}>", rendered_args.join(", "))
}

fn render_map_type(
    proto_type: &str,
    current_package: &str,
    package_by_ident: &BTreeMap<ProtoIdent, String>,
    proto_type_index: &BTreeMap<String, Vec<ProtoIdent>>,
    client_imports: &BTreeMap<String, ClientImport>,
) -> String {
    let Some((key, value)) = parse_map_types(proto_type) else {
        return "::proto_rs::alloc::collections::BTreeMap<::core::primitive::u32, ::core::primitive::u32>".to_string();
    };
    let key_type = proto_name_to_rust_type(key, current_package, package_by_ident, proto_type_index, client_imports);
    let value_type = proto_name_to_rust_type(value, current_package, package_by_ident, proto_type_index, client_imports);
    format!("::proto_rs::alloc::collections::BTreeMap<{key_type}, {value_type}>")
}

fn proto_name_to_rust_type(
    proto_name: &str,
    current_package: &str,
    package_by_ident: &BTreeMap<ProtoIdent, String>,
    proto_type_index: &BTreeMap<String, Vec<ProtoIdent>>,
    client_imports: &BTreeMap<String, ClientImport>,
) -> String {
    if let Some(scalar) = proto_scalar_type(proto_name) {
        return scalar.to_string();
    }
    if proto_name.starts_with("map<") {
        return render_map_type(proto_name, current_package, package_by_ident, proto_type_index, client_imports);
    }

    if let Some(candidates) = proto_type_index.get(proto_name) {
        if let Some(candidate) = candidates.iter().find(|ident| package_by_ident.get(*ident).is_some_and(|pkg| pkg == current_package)) {
            return render_proto_type(*candidate, current_package, package_by_ident, proto_type_index, client_imports);
        }
        if let Some(candidate) = candidates.first() {
            return render_proto_type(*candidate, current_package, package_by_ident, proto_type_index, client_imports);
        }
    }

    proto_name.to_string()
}

fn render_generics(entry: &ProtoSchema) -> String {
    if entry.generics.is_empty() && entry.lifetimes.is_empty() {
        return String::new();
    }

    let mut params = Vec::new();

    for lifetime in entry.lifetimes {
        let mut lifetime_param = format!("'{}", lifetime.name);
        if !lifetime.bounds.is_empty() {
            lifetime_param.push_str(": ");
            lifetime_param.push_str(&lifetime.bounds.join(" + "));
        }
        params.push(lifetime_param);
    }

    for generic in entry.generics {
        match generic.kind {
            GenericKind::Type => {
                let mut param = generic.name.to_string();
                if !generic.constraints.is_empty() {
                    param.push_str(": ");
                    param.push_str(&generic.constraints.join(" + "));
                }
                params.push(param);
            }
            GenericKind::Const => {
                let const_type = generic.const_type.unwrap();
                params.push(format!("const {}: {const_type}", generic.name));
            }
        }
    }

    format!("<{}>", params.join(", "))
}

fn build_proto_type_index(registry: &BTreeMap<String, Vec<&'static ProtoSchema>>) -> BTreeMap<String, Vec<ProtoIdent>> {
    let mut index = BTreeMap::new();
    for entries in registry.values() {
        for entry in entries {
            index.entry(entry.id.proto_type.to_string()).or_insert_with(Vec::new).push(entry.id);
        }
    }
    index
}

fn package_name_for_entries(file_name: &str, entries: &[&ProtoSchema]) -> String {
    let path = Path::new(file_name);
    let file_name_last = path.file_name().and_then(|name| name.to_str()).unwrap_or(file_name);
    entries
        .first()
        .map(|schema| schema.id.proto_package_name)
        .filter(|name| !name.is_empty())
        .map_or_else(|| super::utils::derive_package_name(file_name_last), ToString::to_string)
}
