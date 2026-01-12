use std::collections::BTreeMap;

use super::ProtoEntry;
use super::ProtoIdent;
use super::ProtoLabel;
use super::ProtoSchema;

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

pub(crate) fn proto_scalar_type(proto_type: &str) -> Option<&'static str> {
    match proto_type {
        "double" => Some("f64"),
        "float" => Some("f32"),
        "int32" | "sint32" | "sfixed32" => Some("i32"),
        "int64" | "sint64" | "sfixed64" => Some("i64"),
        "uint32" | "fixed32" => Some("u32"),
        "uint64" | "fixed64" => Some("u64"),
        "bool" => Some("bool"),
        "string" => Some("::proto_rs::alloc::string::String"),
        "bytes" => Some("::proto_rs::alloc::vec::Vec<u8>"),
        _ => None,
    }
}

pub(crate) fn parse_map_types(proto_type: &str) -> Option<(&str, &str)> {
    let inner = proto_type.strip_prefix("map<")?.strip_suffix('>')?;
    let mut parts = inner.splitn(2, ',');
    let key = parts.next()?.trim();
    let value = parts.next()?.trim();
    Some((key, value))
}

pub(crate) fn entry_sort_key(entry: &ProtoSchema) -> (u8, &'static str) {
    let kind = match entry.content {
        ProtoEntry::Import { .. } => 0,
        ProtoEntry::SimpleEnum { .. } => 1,
        ProtoEntry::Struct { .. } => 2,
        ProtoEntry::ComplexEnum { .. } => 3,
        ProtoEntry::Service { .. } => 4,
    };
    (kind, entry.id.proto_type)
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
        Some(
            WrapperKind::Vec
            | WrapperKind::VecDeque
            | WrapperKind::HashSet
            | WrapperKind::BTreeSet
        ) => ProtoLabel::Repeated,
        _ => current,
    }
}

pub(crate) fn wrapper_is_map(wrapper: Option<ProtoIdent>, ident: ProtoIdent) -> bool {
    matches!(
        wrapper_kind_for(wrapper, ident),
        Some(WrapperKind::HashMap | WrapperKind::BTreeMap)
    )
}

pub(crate) fn resolve_transparent_ident(ident: ProtoIdent, ident_index: &BTreeMap<ProtoIdent, &'static ProtoSchema>) -> ProtoIdent {
    transparent_inner_ident(&ident, ident_index).unwrap_or(ident)
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
