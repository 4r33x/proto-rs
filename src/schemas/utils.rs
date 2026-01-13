use std::collections::BTreeMap;

use super::ProtoEntry;
use super::ProtoIdent;
use super::ProtoLabel;
use super::ProtoSchema;
use super::ProtoType;

pub(crate) fn derive_package_name(file_path: &str) -> String {
    file_path.trim_end_matches(".proto").replace(['/', '\\', '-', '.'], "_").to_lowercase()
}

pub(crate) fn module_path_segments(package_name: &str) -> Vec<String> {
    package_name.split('.').filter(|segment| !segment.is_empty()).map(sanitize_module_segment).collect()
}

pub(crate) fn module_path_for_package(package_name: &str) -> String {
    module_path_segments(package_name).join("::")
}

pub(crate) fn sanitize_module_segment(segment: &str) -> String {
    let mut out = String::new();
    for ch in segment.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }
    if out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        out.insert(0, '_');
    }
    if out.is_empty() { "_".to_string() } else { out }
}

pub(crate) fn indent_line(output: &mut String, indent: usize) {
    for _ in 0..indent {
        output.push(' ');
    }
}

pub(crate) fn strip_proto_suffix(type_name: &str) -> String {
    type_name.strip_suffix("Proto").unwrap_or(type_name).to_string()
}

pub(crate) fn rust_type_name(ident: ProtoIdent) -> String {
    strip_proto_suffix(ident.name)
}

pub(crate) fn proto_scalar_type(proto_type: &ProtoType) -> Option<&'static str> {
    match proto_type {
        ProtoType::Double => Some("f64"),
        ProtoType::Float => Some("f32"),
        ProtoType::Int32 | ProtoType::Sint32 | ProtoType::Sfixed32 => Some("i32"),
        ProtoType::Int64 | ProtoType::Sint64 | ProtoType::Sfixed64 => Some("i64"),
        ProtoType::Uint32 | ProtoType::Fixed32 => Some("u32"),
        ProtoType::Uint64 | ProtoType::Fixed64 => Some("u64"),
        ProtoType::Bool => Some("bool"),
        ProtoType::String => Some("::proto_rs::alloc::string::String"),
        ProtoType::Bytes => Some("::proto_rs::alloc::vec::Vec<u8>"),
        _ => None,
    }
}

pub(crate) fn proto_map_types(proto_type: &ProtoType) -> Option<(&ProtoType, &ProtoType)> {
    match proto_type {
        ProtoType::Map { key, value } => Some((key, value)),
        _ => None,
    }
}

pub(crate) fn proto_type_name(proto_type: &ProtoType) -> String {
    match proto_type {
        ProtoType::Message(name) => (*name).to_string(),
        ProtoType::Optional(inner) | ProtoType::Repeated(inner) => proto_type_name(inner),
        ProtoType::Double => "double".to_string(),
        ProtoType::Float => "float".to_string(),
        ProtoType::Int32 => "int32".to_string(),
        ProtoType::Int64 => "int64".to_string(),
        ProtoType::Uint32 => "uint32".to_string(),
        ProtoType::Uint64 => "uint64".to_string(),
        ProtoType::Sint32 => "sint32".to_string(),
        ProtoType::Sint64 => "sint64".to_string(),
        ProtoType::Fixed32 => "fixed32".to_string(),
        ProtoType::Fixed64 => "fixed64".to_string(),
        ProtoType::Sfixed32 => "sfixed32".to_string(),
        ProtoType::Sfixed64 => "sfixed64".to_string(),
        ProtoType::Bool => "bool".to_string(),
        ProtoType::Bytes => "bytes".to_string(),
        ProtoType::String => "string".to_string(),
        ProtoType::Enum => "enum".to_string(),
        ProtoType::Map { key, value } => format!("map<{}, {}>", proto_type_name(key), proto_type_name(value)),
        ProtoType::None => "none".to_string(),
    }
}

pub(crate) fn proto_ident_base_type_name(ident: ProtoIdent) -> String {
    match ident.proto_type {
        ProtoType::Enum | ProtoType::None => ident.name.to_string(),
        _ => proto_type_name(&ident.proto_type),
    }
}

pub(crate) fn entry_sort_key(entry: &ProtoSchema) -> (u8, String) {
    let kind = match entry.content {
        ProtoEntry::Import { .. } => 0,
        ProtoEntry::SimpleEnum { .. } => 1,
        ProtoEntry::Struct { .. } => 2,
        ProtoEntry::ComplexEnum { .. } => 3,
        ProtoEntry::Service { .. } => 4,
    };
    (kind, proto_ident_base_type_name(entry.id))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WrapperKind {
    Option,
    Vec,
    VecDeque,
    HashMap,
    BTreeMap,
    HashSet,
    BTreeSet,
    Box,
    Arc,
    Mutex,
    ArcSwap,
    ArcSwapOption,
    CachePadded,
}

pub(crate) fn wrapper_kind(wrapper: Option<ProtoIdent>) -> Option<WrapperKind> {
    let wrapper = wrapper?;
    Some(match wrapper.name {
        "Option" => WrapperKind::Option,
        "Vec" => WrapperKind::Vec,
        "VecDeque" => WrapperKind::VecDeque,
        "HashMap" => WrapperKind::HashMap,
        "BTreeMap" => WrapperKind::BTreeMap,
        "HashSet" => WrapperKind::HashSet,
        "BTreeSet" => WrapperKind::BTreeSet,
        "Box" => WrapperKind::Box,
        "Arc" => WrapperKind::Arc,
        "Mutex" => WrapperKind::Mutex,
        "ArcSwap" => WrapperKind::ArcSwap,
        "ArcSwapOption" => WrapperKind::ArcSwapOption,
        "CachePadded" => WrapperKind::CachePadded,
        _ => return None,
    })
}

pub(crate) fn wrapper_kind_for(wrapper: Option<ProtoIdent>, ident: ProtoIdent) -> Option<WrapperKind> {
    wrapper_kind(wrapper).or_else(|| {
        if ident.proto_file_path.is_empty() && ident.proto_package_name.is_empty() {
            wrapper_kind(Some(ident))
        } else {
            None
        }
    })
}

pub(crate) fn wrapper_label(wrapper: Option<ProtoIdent>, ident: ProtoIdent, current: ProtoLabel) -> ProtoLabel {
    match wrapper_kind_for(wrapper, ident) {
        Some(WrapperKind::Option | WrapperKind::ArcSwapOption) => ProtoLabel::Optional,
        Some(WrapperKind::Vec | WrapperKind::VecDeque | WrapperKind::HashSet | WrapperKind::BTreeSet) => ProtoLabel::Repeated,
        _ => current,
    }
}

pub(crate) fn wrapper_is_map(wrapper: Option<ProtoIdent>, ident: ProtoIdent) -> bool {
    matches!(wrapper_kind_for(wrapper, ident), Some(WrapperKind::HashMap | WrapperKind::BTreeMap))
}

pub(crate) fn resolve_transparent_ident(ident: ProtoIdent, ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>) -> ProtoIdent {
    transparent_inner_ident(&ident, ident_index).unwrap_or(ident)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct WrapperSchemaInfo {
    pub(crate) wrapper: ProtoIdent,
    pub(crate) inner: ProtoIdent,
}

const WRAPPER_SCHEMA_PREFIXES: &[(&str, WrapperKind)] = &[
    ("ArcSwapOption", WrapperKind::ArcSwapOption),
    ("ArcSwap", WrapperKind::ArcSwap),
    ("CachePadded", WrapperKind::CachePadded),
    ("Option", WrapperKind::Option),
    ("VecDeque", WrapperKind::VecDeque),
    ("Vec", WrapperKind::Vec),
    ("HashMap", WrapperKind::HashMap),
    ("BTreeMap", WrapperKind::BTreeMap),
    ("HashSet", WrapperKind::HashSet),
    ("BTreeSet", WrapperKind::BTreeSet),
    ("Box", WrapperKind::Box),
    ("Arc", WrapperKind::Arc),
    ("Mutex", WrapperKind::Mutex),
];

pub(crate) fn wrapper_kind_from_schema_name(name: &str) -> Option<WrapperKind> {
    WRAPPER_SCHEMA_PREFIXES.iter().find_map(|(prefix, kind)| name.starts_with(prefix).then_some(*kind))
}

pub(crate) fn wrapper_prefix_from_schema_name(name: &str) -> Option<&'static str> {
    WRAPPER_SCHEMA_PREFIXES.iter().find_map(|(prefix, _)| name.starts_with(prefix).then_some(*prefix))
}

pub(crate) fn wrapper_schema_info(
    ident: ProtoIdent,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
) -> Option<WrapperSchemaInfo> {
    let schema = ident_index.get(&ident)?;
    wrapper_schema_info_from_entry(schema)
}

pub(crate) fn wrapper_schema_info_from_entry(schema: &ProtoSchema) -> Option<WrapperSchemaInfo> {
    let fields = match schema.content {
        ProtoEntry::Struct { fields } if fields.len() == 1 => fields,
        _ => return None,
    };
    let field = fields[0];
    let wrapper = field.wrapper?;
    if wrapper_kind_for(Some(wrapper), field.proto_ident).is_none() {
        if wrapper.proto_package_name.is_empty()
            && wrapper.proto_file_path.is_empty()
            && field.name == Some("value")
            && wrapper_kind_from_schema_name(schema.id.name).is_some()
        {
            return Some(WrapperSchemaInfo {
                wrapper,
                inner: field.proto_ident,
            });
        }
        return None;
    }
    Some(WrapperSchemaInfo {
        wrapper,
        inner: field.proto_ident,
    })
}

pub(crate) fn is_wrapper_schema(schema: &ProtoSchema) -> bool {
    if wrapper_schema_info_from_entry(schema).is_some() {
        return true;
    }

    match schema.content {
        ProtoEntry::Struct { fields } if fields.len() == 1 => {
            let field = fields[0];
            field.name == Some("value")
                && (field.wrapper.is_some()
                    || matches!(field.proto_label, ProtoLabel::Optional | ProtoLabel::Repeated)
                    || matches!(field.proto_ident.proto_type, ProtoType::Map { .. }))
                && wrapper_kind_from_schema_name(schema.id.name).is_some()
        }
        _ => false,
    }
}

pub(crate) fn resolve_transparent_or_wrapper_inner(
    ident: ProtoIdent,
    ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>,
) -> ProtoIdent {
    if let Some(schema) = ident_index.get(&ident)
        && wrapper_kind_from_schema_name(schema.id.name).is_some()
        && let ProtoEntry::Struct { fields } = schema.content
        && fields.len() == 1
    {
        return fields[0].proto_ident;
    }
    wrapper_schema_info(ident, ident_index).map_or_else(|| resolve_transparent_ident(ident, ident_index), |info| info.inner)
}

fn transparent_inner_ident(ident: &ProtoIdent, ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>) -> Option<ProtoIdent> {
    let schema = ident_index.get(ident)?;
    if !is_transparent_schema(schema) {
        return None;
    }

    match schema.content {
        ProtoEntry::Struct { fields } if fields.len() == 1 => Some(fields[0].proto_ident),
        _ => None,
    }
}

fn is_transparent_schema(schema: &ProtoSchema) -> bool {
    schema.top_level_attributes.iter().any(|attr| attr.path == "proto_message" && attr.tokens.contains("transparent"))
}

pub(crate) fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    let mut prev_is_lower = false;
    let mut prev_is_upper = false;

    while let Some(c) = chars.next() {
        let next_is_upper = chars.peek().is_some_and(|ch| ch.is_uppercase());
        let next_is_lower = chars.peek().is_some_and(|ch| ch.is_lowercase());

        if c.is_uppercase() && !result.is_empty() && (prev_is_lower || prev_is_upper && (next_is_upper || next_is_lower)) {
            result.push('_');
        }

        result.push(c.to_ascii_lowercase());
        prev_is_lower = c.is_lowercase();
        prev_is_upper = c.is_uppercase();
    }

    result
}
