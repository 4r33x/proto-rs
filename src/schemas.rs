use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt::Write;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::LazyLock;

mod proto_output;
mod rust_client;
mod utils;

/// Represents a proto schema collected at compile time
#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash)]
pub struct ProtoSchema {
    pub id: ProtoIdent,
    pub generics: &'static [Generic],
    pub lifetimes: &'static [Lifetime],
    pub top_level_attributes: &'static [Attribute],
    pub content: ProtoEntry,
}

pub struct RustClientCtx<'a> {
    pub output_path: Option<&'a str>,
    pub imports: &'a [&'a str],
    pub client_attrs: BTreeMap<ProtoIdent, Vec<UserAttr>>,
    pub module_attrs: BTreeMap<String, Vec<String>>,
    pub statements: BTreeMap<String, Vec<String>>,
    pub type_replacements: BTreeMap<ProtoIdent, Vec<TypeReplace>>,
}

impl<'a> RustClientCtx<'a> {
    pub const fn disabled() -> Self {
        Self {
            output_path: None,
            imports: &[],
            client_attrs: BTreeMap::new(),
            module_attrs: BTreeMap::new(),
            statements: BTreeMap::new(),
            type_replacements: BTreeMap::new(),
        }
    }

    pub const fn enabled(output_path: &'a str) -> Self {
        Self {
            output_path: Some(output_path),
            imports: &[],
            client_attrs: BTreeMap::new(),
            module_attrs: BTreeMap::new(),
            statements: BTreeMap::new(),
            type_replacements: BTreeMap::new(),
        }
    }
    #[must_use]
    pub const fn with_imports(mut self, imports: &'a [&'a str]) -> Self {
        self.imports = imports;
        self
    }

    #[must_use]
    pub fn with_statements(mut self, statements: &[(&str, &str)]) -> Self {
        for (module_name, statement) in statements {
            if statement.trim().is_empty() {
                continue;
            }
            self.statements.entry((*module_name).to_string()).or_default().push((*statement).to_string());
        }
        self
    }

    #[must_use]
    #[allow(clippy::needless_pass_by_value)]
    pub fn add_client_attrs(mut self, target: ClientAttrTarget<'a>, attr: UserAttr) -> Self {
        match target {
            ClientAttrTarget::Ident(ident) => {
                self.client_attrs.entry(ident).or_default().push(attr);
            }
            ClientAttrTarget::Module(module_name) => {
                assert!(
                    matches!(attr.level, AttrLevel::Top),
                    "module-level client attributes must use AttrLevel::Top"
                );
                self.module_attrs.entry(module_name.to_string()).or_default().push(attr.attr);
            }
        }
        self
    }

    #[must_use]
    pub fn replace_type(mut self, replacements: &[TypeReplace]) -> Self {
        for replacement in replacements {
            let entry = replacement.target_ident();
            let entry_replacements = self.type_replacements.entry(entry).or_default();
            if !entry_replacements.contains(replacement) {
                entry_replacements.push(replacement.clone());
            }
        }
        self
    }
}

pub enum ClientAttrTarget<'a> {
    Module(&'a str),
    Ident(ProtoIdent),
}

#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ProtoIdent {
    pub module_path: &'static str,
    pub name: &'static str,
    pub proto_package_name: &'static str,
    pub proto_file_path: &'static str,
    pub proto_type: ProtoType,
    pub generics: &'static [ProtoIdent],
}

#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ProtoType {
    Message(&'static str),
    Optional(&'static ProtoType),
    Repeated(&'static ProtoType),
    Double,
    Float,
    Int32,
    Int64,
    Uint32,
    Uint64,
    Sint32,
    Sint64,
    Fixed32,
    Fixed64,
    Sfixed32,
    Sfixed64,
    Bool,
    Bytes,
    String,
    Enum,
    Map {
        key: &'static ProtoType,
        value: &'static ProtoType,
    },
    None,
}
impl ProtoType {
    const fn is_allowed_as_key(&self) -> bool {
        matches!(
            self,
            ProtoType::Int32
                | ProtoType::Int64
                | ProtoType::Uint32
                | ProtoType::Uint64
                | ProtoType::Sint32
                | ProtoType::Sint64
                | ProtoType::Fixed32
                | ProtoType::Fixed64
                | ProtoType::Sfixed32
                | ProtoType::Sfixed64
                | ProtoType::Bool
                | ProtoType::String
        )
    }
    const fn proto_type_validation(&self, mut ctx: TypeValidatorCtx) -> Result<(), &'static str> {
        match self {
            ProtoType::Optional(t) => {
                if ctx.repeated {
                    return Err("repeated optional is invalid");
                }
                if ctx.optional {
                    return Err("optional optional is invalid");
                }
                if ctx.is_map() {
                    return Err("optional map key/value is invalid");
                }
                ctx.optional = true;
                t.proto_type_validation(ctx)
            }
            ProtoType::Repeated(t) => {
                if ctx.repeated {
                    return Err("repeated repeated is invalid");
                }
                if ctx.optional {
                    return Err("optional repeated is invalid");
                }
                if ctx.map_key {
                    return Err("repeated map key is invalid");
                }
                if ctx.map_value {
                    return Err("repeated map value is invalid");
                }
                ctx.repeated = true;
                t.proto_type_validation(ctx)
            }
            ProtoType::Map { key, value } => {
                if ctx.repeated {
                    return Err("repeated map is invalid");
                }
                if ctx.optional {
                    return Err("optional map is invalid");
                }
                if ctx.is_map() {
                    return Err("map in map is not allowed");
                }
                if !key.is_allowed_as_key() {
                    return Err("type is not allowed as map key");
                }

                let mut ctx_key = ctx;
                ctx_key.map_key = true;

                if let Err(e) = key.proto_type_validation(ctx_key) {
                    return Err(e);
                }

                let mut ctx_val = ctx;
                ctx_val.map_value = true;
                value.proto_type_validation(ctx_val)
            }
            ProtoType::Message(_) => {
                if ctx.map_key {
                    return Err("message as map key is invalid");
                }
                Ok(())
            }
            ProtoType::Double
            | ProtoType::Float
            | ProtoType::Int32
            | ProtoType::Int64
            | ProtoType::Uint32
            | ProtoType::Uint64
            | ProtoType::Sint32
            | ProtoType::Sint64
            | ProtoType::Fixed32
            | ProtoType::Fixed64
            | ProtoType::Sfixed32
            | ProtoType::Sfixed64
            | ProtoType::Bool
            | ProtoType::Bytes
            | ProtoType::String
            | ProtoType::None
            | ProtoType::Enum => Ok(()),
        }
    }
}

pub trait ProtoIdentifiable: Sized {
    const PROTO_IDENT: ProtoIdent;
    const PROTO_TYPE: ProtoType;
    const _VALIDATOR: () = {
        if let Err(e) = Self::PROTO_TYPE.proto_type_validation(TypeValidatorCtx::new()) {
            proto_type_validation_fail::<Self>(e);
        }
    };
}

#[track_caller]
#[allow(clippy::extra_unused_type_parameters)]
pub const fn proto_type_validation_fail<T: ProtoIdentifiable>(e: &'static str) -> ! {
    const_panic::concat_panic!("Error in validation ", T::PROTO_IDENT.name, ": ", e)
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy)]
struct TypeValidatorCtx {
    optional: bool,
    repeated: bool,
    map_key: bool,
    map_value: bool,
}
impl TypeValidatorCtx {
    pub const fn new() -> Self {
        Self {
            optional: false,
            repeated: false,
            map_key: false,
            map_value: false,
        }
    }
    pub const fn is_map(self) -> bool {
        self.map_key || self.map_value
    }
}

macro_rules! impl_proto_ident_primitive {
    ($ty:ty, $proto_type:expr) => {
        #[cfg(feature = "build-schemas")]
        impl ProtoIdentifiable for $ty {
            const PROTO_IDENT: ProtoIdent = ProtoIdent {
                module_path: module_path!(),
                name: stringify!($ty),
                proto_package_name: "",
                proto_file_path: "",
                proto_type: $proto_type,
                generics: &[],
            };
            const PROTO_TYPE: ProtoType = $proto_type;
        }
        #[cfg(feature = "build-schemas")]
        const _: () = <$ty as ProtoIdentifiable>::_VALIDATOR;
    };
}

impl_proto_ident_primitive!(bool, ProtoType::Bool);
impl_proto_ident_primitive!(u8, ProtoType::Uint32);
impl_proto_ident_primitive!(u16, ProtoType::Uint32);
impl_proto_ident_primitive!(u32, ProtoType::Uint32);
impl_proto_ident_primitive!(u64, ProtoType::Uint64);
impl_proto_ident_primitive!(usize, ProtoType::Uint64);
impl_proto_ident_primitive!(i8, ProtoType::Int32);
impl_proto_ident_primitive!(i16, ProtoType::Int32);
impl_proto_ident_primitive!(i32, ProtoType::Int32);
impl_proto_ident_primitive!(i64, ProtoType::Int64);
impl_proto_ident_primitive!(isize, ProtoType::Int64);
impl_proto_ident_primitive!(f32, ProtoType::Float);
impl_proto_ident_primitive!(f64, ProtoType::Double);
impl_proto_ident_primitive!(crate::bytes::Bytes, ProtoType::Bytes);
impl_proto_ident_primitive!(::std::string::String, ProtoType::String);
impl_proto_ident_primitive!(::core::sync::atomic::AtomicBool, ProtoType::Bool);
impl_proto_ident_primitive!(::core::sync::atomic::AtomicU8, ProtoType::Uint32);
impl_proto_ident_primitive!(::core::sync::atomic::AtomicU16, ProtoType::Uint32);
impl_proto_ident_primitive!(::core::sync::atomic::AtomicU32, ProtoType::Uint32);
impl_proto_ident_primitive!(::core::sync::atomic::AtomicU64, ProtoType::Uint64);
impl_proto_ident_primitive!(::core::sync::atomic::AtomicUsize, ProtoType::Uint64);
impl_proto_ident_primitive!(::core::sync::atomic::AtomicI8, ProtoType::Int32);
impl_proto_ident_primitive!(::core::sync::atomic::AtomicI16, ProtoType::Int32);
impl_proto_ident_primitive!(::core::sync::atomic::AtomicI32, ProtoType::Int32);
impl_proto_ident_primitive!(::core::sync::atomic::AtomicI64, ProtoType::Int64);
impl_proto_ident_primitive!(::core::sync::atomic::AtomicIsize, ProtoType::Int64);

#[cfg(feature = "build-schemas")]
impl<T: ProtoIdentifiable, const N: usize> ProtoIdentifiable for [T; N] {
    const PROTO_IDENT: ProtoIdent = T::PROTO_IDENT;
    const PROTO_TYPE: ProtoType = T::PROTO_TYPE;
}

#[cfg(feature = "build-schemas")]
impl<T: ProtoIdentifiable> ProtoIdentifiable for ::core::option::Option<T> {
    const PROTO_IDENT: ProtoIdent = ProtoIdent {
        module_path: module_path!(),
        name: "Option",
        proto_package_name: "",
        proto_file_path: "",
        proto_type: Self::PROTO_TYPE,
        generics: &[T::PROTO_IDENT],
    };
    const PROTO_TYPE: ProtoType = ProtoType::Optional(&T::PROTO_TYPE);
}

#[cfg(feature = "build-schemas")]
impl<T: ProtoIdentifiable> ProtoIdentifiable for ::std::boxed::Box<T> {
    const PROTO_IDENT: ProtoIdent = ProtoIdent {
        module_path: module_path!(),
        name: "Box",
        proto_package_name: "",
        proto_file_path: "",
        proto_type: Self::PROTO_TYPE,
        generics: &[T::PROTO_IDENT],
    };
    const PROTO_TYPE: ProtoType = T::PROTO_TYPE;
}

#[cfg(feature = "build-schemas")]
impl<T: ProtoIdentifiable> ProtoIdentifiable for ::std::sync::Arc<T> {
    const PROTO_IDENT: ProtoIdent = ProtoIdent {
        module_path: module_path!(),
        name: "Arc",
        proto_package_name: "",
        proto_file_path: "",
        proto_type: Self::PROTO_TYPE,
        generics: &[T::PROTO_IDENT],
    };
    const PROTO_TYPE: ProtoType = T::PROTO_TYPE;
}

#[cfg(feature = "build-schemas")]
impl<T: ProtoIdentifiable> ProtoIdentifiable for ::std::sync::Mutex<T> {
    const PROTO_IDENT: ProtoIdent = ProtoIdent {
        module_path: module_path!(),
        name: "Mutex",
        proto_package_name: "",
        proto_file_path: "",
        proto_type: Self::PROTO_TYPE,
        generics: &[T::PROTO_IDENT],
    };
    const PROTO_TYPE: ProtoType = T::PROTO_TYPE;
}

#[cfg(feature = "build-schemas")]
impl<T: ProtoIdentifiable> ProtoIdentifiable for ::std::vec::Vec<T> {
    const PROTO_IDENT: ProtoIdent = ProtoIdent {
        module_path: module_path!(),
        name: "Vec",
        proto_package_name: "",
        proto_file_path: "",
        proto_type: Self::PROTO_TYPE,
        generics: &[T::PROTO_IDENT],
    };
    const PROTO_TYPE: ProtoType = ProtoType::Repeated(&T::PROTO_TYPE);
}

#[cfg(feature = "build-schemas")]
impl<T: ProtoIdentifiable> ProtoIdentifiable for ::std::collections::VecDeque<T> {
    const PROTO_IDENT: ProtoIdent = ProtoIdent {
        module_path: module_path!(),
        name: "VecDeque",
        proto_package_name: "",
        proto_file_path: "",
        proto_type: Self::PROTO_TYPE,
        generics: &[T::PROTO_IDENT],
    };
    const PROTO_TYPE: ProtoType = ProtoType::Repeated(&T::PROTO_TYPE);
}

#[cfg(feature = "build-schemas")]
impl<K: ProtoIdentifiable, V: ProtoIdentifiable, S> ProtoIdentifiable for ::std::collections::HashMap<K, V, S> {
    const PROTO_IDENT: ProtoIdent = ProtoIdent {
        module_path: module_path!(),
        name: "HashMap",
        proto_package_name: "",
        proto_file_path: "",
        proto_type: Self::PROTO_TYPE,
        generics: &[K::PROTO_IDENT, V::PROTO_IDENT],
    };
    const PROTO_TYPE: ProtoType = ProtoType::Map {
        key: &K::PROTO_TYPE,
        value: &V::PROTO_TYPE,
    };
}

#[cfg(feature = "build-schemas")]
impl<K: ProtoIdentifiable, V: ProtoIdentifiable> ProtoIdentifiable for ::std::collections::BTreeMap<K, V> {
    const PROTO_IDENT: ProtoIdent = ProtoIdent {
        module_path: module_path!(),
        name: "BTreeMap",
        proto_package_name: "",
        proto_file_path: "",
        proto_type: Self::PROTO_TYPE,
        generics: &[K::PROTO_IDENT, V::PROTO_IDENT],
    };
    const PROTO_TYPE: ProtoType = ProtoType::Map {
        key: &K::PROTO_TYPE,
        value: &V::PROTO_TYPE,
    };
}

#[cfg(feature = "build-schemas")]
impl<T: ProtoIdentifiable, S> ProtoIdentifiable for ::std::collections::HashSet<T, S> {
    const PROTO_IDENT: ProtoIdent = ProtoIdent {
        module_path: module_path!(),
        name: "HashSet",
        proto_package_name: "",
        proto_file_path: "",
        proto_type: Self::PROTO_TYPE,
        generics: &[T::PROTO_IDENT],
    };
    const PROTO_TYPE: ProtoType = ProtoType::Repeated(&T::PROTO_TYPE);
}

#[cfg(feature = "build-schemas")]
impl<T: ProtoIdentifiable> ProtoIdentifiable for ::std::collections::BTreeSet<T> {
    const PROTO_IDENT: ProtoIdent = ProtoIdent {
        module_path: module_path!(),
        name: "BTreeSet",
        proto_package_name: "",
        proto_file_path: "",
        proto_type: Self::PROTO_TYPE,
        generics: &[T::PROTO_IDENT],
    };
    const PROTO_TYPE: ProtoType = ProtoType::Repeated(&T::PROTO_TYPE);
}

#[cfg(all(feature = "build-schemas", feature = "arc_swap"))]
impl<T: ProtoIdentifiable> ProtoIdentifiable for arc_swap::ArcSwap<T> {
    const PROTO_IDENT: ProtoIdent = ProtoIdent {
        module_path: module_path!(),
        name: "ArcSwap",
        proto_package_name: "",
        proto_file_path: "",
        proto_type: Self::PROTO_TYPE,
        generics: &[T::PROTO_IDENT],
    };
    const PROTO_TYPE: ProtoType = T::PROTO_TYPE;
}

#[cfg(all(feature = "build-schemas", feature = "arc_swap"))]
impl<T: ProtoIdentifiable> ProtoIdentifiable for arc_swap::ArcSwapOption<T> {
    const PROTO_IDENT: ProtoIdent = ProtoIdent {
        module_path: module_path!(),
        name: "ArcSwapOption",
        proto_package_name: "",
        proto_file_path: "",
        proto_type: Self::PROTO_TYPE,
        generics: &[T::PROTO_IDENT],
    };
    const PROTO_TYPE: ProtoType = ProtoType::Optional(&T::PROTO_TYPE);
}

#[cfg(all(feature = "build-schemas", feature = "cache_padded"))]
impl<T: ProtoIdentifiable> ProtoIdentifiable for crossbeam_utils::CachePadded<T> {
    const PROTO_IDENT: ProtoIdent = ProtoIdent {
        module_path: module_path!(),
        name: "CachePadded",
        proto_package_name: "",
        proto_file_path: "",
        proto_type: Self::PROTO_TYPE,
        generics: &[T::PROTO_IDENT],
    };
    const PROTO_TYPE: ProtoType = T::PROTO_TYPE;
}

#[cfg(all(feature = "build-schemas", feature = "parking_lot"))]
impl<T: ProtoIdentifiable> ProtoIdentifiable for parking_lot::Mutex<T> {
    const PROTO_IDENT: ProtoIdent = ProtoIdent {
        module_path: module_path!(),
        name: "Mutex",
        proto_package_name: "",
        proto_file_path: "",
        proto_type: Self::PROTO_TYPE,
        generics: &[T::PROTO_IDENT],
    };
    const PROTO_TYPE: ProtoType = T::PROTO_TYPE;
}

#[cfg(all(feature = "build-schemas", feature = "papaya"))]
impl<K: ProtoIdentifiable, V: ProtoIdentifiable, S> ProtoIdentifiable for papaya::HashMap<K, V, S> {
    const PROTO_IDENT: ProtoIdent = ProtoIdent {
        module_path: module_path!(),
        name: "HashMap",
        proto_package_name: "",
        proto_file_path: "",
        proto_type: Self::PROTO_TYPE,
        generics: &[K::PROTO_IDENT, V::PROTO_IDENT],
    };
    const PROTO_TYPE: ProtoType = ProtoType::Map {
        key: &K::PROTO_TYPE,
        value: &V::PROTO_TYPE,
    };
}

#[cfg(all(feature = "build-schemas", feature = "papaya"))]
impl<T: ProtoIdentifiable, S> ProtoIdentifiable for papaya::HashSet<T, S> {
    const PROTO_IDENT: ProtoIdent = ProtoIdent {
        module_path: module_path!(),
        name: "HashSet",
        proto_package_name: "",
        proto_file_path: "",
        proto_type: Self::PROTO_TYPE,
        generics: &[T::PROTO_IDENT],
    };
    const PROTO_TYPE: ProtoType = ProtoType::Repeated(&T::PROTO_TYPE);
}

#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Attribute {
    pub path: &'static str,
    pub tokens: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserAttr {
    pub level: AttrLevel,
    pub attr: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AttrLevel {
    Top,
    Field {
        field_name: String,
        id: ProtoIdent,
        variant: Option<String>,
    },
    Method {
        method_name: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum MethodReplace {
    Argument(String),
    Return(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum TypeReplace {
    Trait {
        id: ProtoIdent,
        method: String,
        kind: MethodReplace,
        type_name: String,
    },
    Type {
        id: ProtoIdent,
        variant: Option<String>,
        field: String,
        type_name: String,
    },
}

impl TypeReplace {
    pub const fn target_ident(&self) -> ProtoIdent {
        match self {
            TypeReplace::Trait { id, .. } | TypeReplace::Type { id, .. } => *id,
        }
    }
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash)]
pub struct Generic {
    pub name: &'static str,
    pub kind: GenericKind,
    pub constraints: &'static [&'static str],
    pub const_type: Option<&'static str>,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash)]
pub enum GenericKind {
    Type,
    Const,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash)]
pub struct Lifetime {
    pub name: &'static str,
    pub bounds: &'static [&'static str],
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash)]
pub enum ProtoEntry {
    SimpleEnum {
        variants: &'static [&'static Variant],
    },
    Struct {
        fields: &'static [&'static Field],
    },
    ComplexEnum {
        variants: &'static [&'static Variant],
    },
    Import {
        paths: &'static [&'static str],
    },
    Service {
        methods: &'static [&'static ServiceMethod],
        rpc_package_name: &'static str,
    },
}

#[derive(Clone, Debug, Copy, Eq, PartialEq, Hash)]
pub struct Variant {
    pub name: &'static str,
    pub fields: &'static [&'static Field],
    pub discriminant: Option<i32>,
}

#[derive(Clone, Debug, Copy, Eq, PartialEq, Hash)]
pub struct Field {
    pub name: Option<&'static str>,
    pub proto_ident: ProtoIdent,
    pub rust_proto_ident: ProtoIdent,
    pub wrapper: Option<ProtoIdent>,
    pub generic_args: &'static [&'static ProtoIdent],
    pub proto_label: ProtoLabel,
    pub tag: u32,
    pub attributes: &'static [Attribute],
    pub array_len: Option<&'static str>,
    pub array_is_bytes: bool,
    pub array_elem: Option<ProtoIdent>,
}

#[derive(Clone, Debug, Copy, Eq, PartialEq, Hash)]
pub struct ServiceMethod {
    pub name: &'static str,
    pub request: ProtoIdent,
    pub request_generic_args: &'static [&'static ProtoIdent],
    pub request_wrapper: Option<ProtoIdent>,
    pub response: ProtoIdent,
    pub response_generic_args: &'static [&'static ProtoIdent],
    pub response_wrapper: Option<ProtoIdent>,
    pub client_streaming: bool,
    pub server_streaming: bool,
}

#[derive(Clone, Debug, Copy, Eq, PartialEq, Hash)]
pub enum ProtoLabel {
    None,
    Optional,
    Repeated,
}

// Auto-collect all schemas via inventory
inventory::collect!(ProtoSchema);

static REGISTRY: LazyLock<BTreeMap<String, Vec<&'static ProtoSchema>>> = LazyLock::new(|| build_registry().0);

/// Get an iterator over all registered proto schemas
///
/// Schemas are automatically collected from all crates that use
/// proto_dump, proto_message, or proto_rpc macros when compiled
/// with the "build-schemas" feature.
pub fn all() -> impl Iterator<Item = &'static ProtoSchema> {
    inventory::iter::<ProtoSchema>.into_iter()
}

/// Write all registered proto schemas to a directory
///
/// # Arguments
/// * `output_dir` - The directory to write .proto files to
/// * `rust_client_output` - Controls whether a Rust client module is generated
///
/// # Returns
/// The number of proto files written
///
/// # Example
/// ```no_run
/// // In main.rs or build.rs (all protos should be declared in other_crates)
/// fn your_main() {
///     if std::env::var("GENERATE_PROTOS").is_ok() {
///         let count = proto_rs::schemas::write_all("protos", &proto_rs::schemas::RustClientCtx::disabled())
///             .expect("Failed to write proto files");
///         println!("Generated {} proto files", count);
///     }
/// }
/// ```
/// Write all registered proto schemas to a directory
/// # Errors
///
/// Will return `Err` if fs throws error
pub fn write_all(output_dir: &str, rust_client_output: &RustClientCtx<'_>) -> io::Result<usize> {
    match fs::remove_dir_all(output_dir) {
        Ok(()) => {}
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(err) => return Err(err),
    }
    fs::create_dir_all(output_dir)?;
    let mut count = 0;
    let (registry, ident_index) = build_registry();
    let all_entries: Vec<&ProtoSchema> = registry.values().flat_map(|entries| entries.iter().copied()).collect();
    let specializations = proto_output::collect_generic_specializations(&all_entries, &ident_index);

    for (file_name, entries) in &registry {
        let output_path = format!("{output_dir}/{file_name}");

        if let Some(parent) = Path::new(&output_path).parent() {
            fs::create_dir_all(parent)?;
        }

        let path = Path::new(file_name.as_str());
        let file_name_last = path.file_name().unwrap().to_str().unwrap();
        let package_name = entries
            .first()
            .map(|schema| schema.id.proto_package_name)
            .filter(|name| !name.is_empty())
            .map_or(utils::derive_package_name(file_name_last), ToString::to_string);
        let mut output = String::new();

        output.push_str("//CODEGEN BELOW - DO NOT TOUCH ME\n");
        output.push_str("syntax = \"proto3\";\n");
        writeln!(output, "package {package_name};").unwrap();

        output.push('\n');

        let imports = proto_output::collect_imports(entries.as_slice(), &ident_index, file_name, &package_name)?;
        if !imports.is_empty() {
            let mut import_stems = BTreeSet::new();
            for import in &imports {
                let import_path = Path::new(import);
                let import_file = import_path.file_name().and_then(|name| name.to_str()).unwrap_or(import);
                let import_stem = import_file.strip_suffix(".proto").unwrap_or(import_file);
                import_stems.insert(import_stem.to_string());
            }
            for import_stem in import_stems {
                writeln!(output, "import \"{import_stem}.proto\";").unwrap();
            }
            output.push('\n');
        }

        let definitions = proto_output::render_entries(entries, &package_name, &ident_index, &specializations);
        for definition in definitions {
            output.push_str(&definition);
            output.push('\n');
        }

        fs::write(&output_path, output)?;
        count += 1;
    }

    if let Some(output_path) = rust_client_output.output_path {
        rust_client::write_rust_client_module(
            output_path,
            rust_client_output.imports,
            &rust_client_output.client_attrs,
            &rust_client_output.module_attrs,
            &rust_client_output.statements,
            &rust_client_output.type_replacements,
            &registry,
            &ident_index,
        )?;
    }

    Ok(count)
}

/// Get the total number of registered files
pub fn count() -> usize {
    REGISTRY.len()
}

/// Get all filenames in the registry
pub fn file_names() -> Vec<String> {
    REGISTRY.keys().cloned().collect()
}

fn build_registry() -> (
    BTreeMap<String, Vec<&'static ProtoSchema>>,
    BTreeMap<ProtoIdent, &'static ProtoSchema>,
) {
    let mut registry = BTreeMap::new();
    let mut ident_index = BTreeMap::new();

    for schema in inventory::iter::<ProtoSchema>() {
        if ident_index.insert(schema.id, schema).is_some() {
            continue;
        }
        if schema.id.proto_file_path.is_empty() {
            continue;
        }
        registry.entry(schema.id.proto_file_path.to_string()).or_insert_with(Vec::new).push(schema);
    }

    (registry, ident_index)
}
