//! String manipulation utilities

/// Convert identifier to `UPPER_SNAKE_CASE` for proto enums
pub fn to_upper_snake_case(s: &str) -> String {
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

        result.push(c.to_ascii_uppercase());
        prev_is_lower = c.is_lowercase();
        prev_is_upper = c.is_uppercase();
    }

    result
}

/// Convert identifier to `snake_case`
pub fn to_snake_case(s: &str) -> String {
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

/// Convert identifier to `PascalCase`
pub fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .filter(|w| !w.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect()
}

/// Strip "Proto" suffix from type name
pub fn strip_proto_suffix(type_name: &str) -> String {
    type_name.strip_suffix("Proto").unwrap_or(type_name).to_string()
}

/// Derive package name from file path
pub fn derive_package_name(file_path: &str) -> String {
    file_path.trim_end_matches(".proto").replace(['/', '\\', '-', '.'], "_").to_lowercase()
}

/// Format import statement
pub fn format_import(import_path: &str) -> String {
    format!("import \"{import_path}.proto\";\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_upper_snake_case() {
        assert_eq!(to_upper_snake_case("MyEnum"), "MY_ENUM");
        assert_eq!(to_upper_snake_case("HTTPStatus"), "H_T_T_P_STATUS");
        assert_eq!(to_upper_snake_case("simple"), "SIMPLE");
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("MyStruct"), "my_struct");
        assert_eq!(to_snake_case("HTTPClient"), "h_t_t_p_client");
        assert_eq!(to_snake_case("already_snake"), "already_snake");
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("my_function"), "MyFunction");
        assert_eq!(to_pascal_case("http_client"), "HttpClient");
        assert_eq!(to_pascal_case("PascalCase"), "PascalCase");
    }

    #[test]
    fn test_strip_proto_suffix() {
        assert_eq!(strip_proto_suffix("MyStructProto"), "MyStruct");
        assert_eq!(strip_proto_suffix("NoSuffix"), "NoSuffix");
    }

    #[test]
    fn test_derive_package_name() {
        assert_eq!(derive_package_name("path/to/file.proto"), "path_to_file");
        assert_eq!(derive_package_name("my-service.proto"), "my_service");
    }
}
