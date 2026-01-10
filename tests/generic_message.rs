use std::collections::VecDeque;

use bytes::Bytes;
use proto_rs::ProtoExt;
use proto_rs::proto_message;

#[proto_message]
#[derive(Debug, PartialEq)]
struct Pair<K, V> {
    key: K,
    value: V,
}

mod tst {
    #[derive(Debug, PartialEq)]
    struct Pair<K, V> {
        key: K,
        value: V,
    }
    impl<K, V> ::proto_rs::ProtoShadow<Self> for Pair<K, V>
    where
        for<'a> K: 'a,
        for<'a> V: 'a,
        for<'a> K: ::proto_rs::EncodeInputFromRef<'a>,
        for<'a> V: ::proto_rs::EncodeInputFromRef<'a>,
        for<'a> K: ::proto_rs::EncodeInputFromRef<'a>,
        for<'a> V: ::proto_rs::EncodeInputFromRef<'a>,
    {
        type Sun<'a> = &'a Self;
        type OwnedSun = Self;
        type View<'a> = &'a Self;
        #[inline(always)]
        fn to_sun(self) -> Result<Self::OwnedSun, ::proto_rs::DecodeError> {
            Ok(self)
        }
        #[inline(always)]
        fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
            value
        }
    }
    impl<K, V> ::proto_rs::ProtoExt for Pair<K, V>
    where
        for<'a> K: 'a,
        for<'a> V: 'a,
        for<'a> K: ::proto_rs::EncodeInputFromRef<'a>,
        for<'a> V: ::proto_rs::EncodeInputFromRef<'a>,
        for<'a> K: ::proto_rs::EncodeInputFromRef<'a>,
        for<'a> V: ::proto_rs::EncodeInputFromRef<'a>,
    {
        type Shadow<'b> = Pair<K, V>;
        #[inline(always)]
        fn merge_field(
            value: &mut Self::Shadow<'_>,
            tag: u32,
            wire_type: ::proto_rs::encoding::WireType,
            buf: &mut impl ::proto_rs::bytes::Buf,
            ctx: ::proto_rs::encoding::DecodeContext,
        ) -> Result<(), ::proto_rs::DecodeError> {
            match tag {
                1u32 => {
                    <K as ::proto_rs::ProtoWire>::decode_into(wire_type, &mut value.key, buf, ctx)?;
                    Ok(())
                }
                2u32 => {
                    <V as ::proto_rs::ProtoWire>::decode_into(wire_type, &mut value.value, buf, ctx)?;
                    Ok(())
                }
                _ => ::proto_rs::encoding::skip_field(wire_type, tag, buf, ctx),
            }
        }
    }
    impl<K, V> ::proto_rs::ProtoWire for Pair<K, V>
    where
        for<'a> K: 'a,
        for<'a> V: 'a,
        for<'a> K: ::proto_rs::EncodeInputFromRef<'a>,
        for<'a> V: ::proto_rs::EncodeInputFromRef<'a>,
        for<'a> K: ::proto_rs::EncodeInputFromRef<'a>,
        for<'a> V: ::proto_rs::EncodeInputFromRef<'a>,
    {
        type EncodeInput<'b> = <Self as ::proto_rs::ProtoShadow<Self>>::View<'b>;
        const KIND: ::proto_rs::ProtoKind = ::proto_rs::ProtoKind::Message;
        #[inline(always)]
        fn proto_default() -> Self {
            Self {
                key: <K as ::proto_rs::ProtoWire>::proto_default(),
                value: <V as ::proto_rs::ProtoWire>::proto_default(),
            }
        }
        #[inline(always)]
        fn clear(&mut self) {
            <K as ::proto_rs::ProtoWire>::clear(&mut self.key);
            <V as ::proto_rs::ProtoWire>::clear(&mut self.value);
        }
        #[inline(always)]
        fn is_default_impl(value: &Self::EncodeInput<'_>) -> bool {
            {
                if !<K as ::proto_rs::ProtoWire>::is_default_impl(&<K as ::proto_rs::EncodeInputFromRef<'_>>::encode_input_from_ref(&(value.key))) {
                    return false;
                }
            };
            {
                if !<V as ::proto_rs::ProtoWire>::is_default_impl(&<V as ::proto_rs::EncodeInputFromRef<'_>>::encode_input_from_ref(&(value.value))) {
                    return false;
                }
            };
            true
        }
        #[inline(always)]
        unsafe fn encoded_len_impl_raw(value: &Self::EncodeInput<'_>) -> usize {
            0 + { <K as ::proto_rs::ProtoWire>::encoded_len_tagged_impl(&<K as ::proto_rs::EncodeInputFromRef<'_>>::encode_input_from_ref(&(value.key)), 1u32) } + {
                <V as ::proto_rs::ProtoWire>::encoded_len_tagged_impl(&<V as ::proto_rs::EncodeInputFromRef<'_>>::encode_input_from_ref(&(value.value)), 2u32)
            }
        }
        #[inline(always)]
        fn encode_raw_unchecked(value: Self::EncodeInput<'_>, buf: &mut impl ::proto_rs::bytes::BufMut) {
            {
                <K as ::proto_rs::ProtoWire>::encode_with_tag(1u32, <K as ::proto_rs::EncodeInputFromRef<'_>>::encode_input_from_ref(&(value.key)), buf)
            }
            { <V as ::proto_rs::ProtoWire>::encode_with_tag(2u32, <V as ::proto_rs::EncodeInputFromRef<'_>>::encode_input_from_ref(&(value.value)), buf) }
        }
        #[inline(always)]
        fn decode_into(
            wire_type: ::proto_rs::encoding::WireType,
            value: &mut Self,
            buf: &mut impl ::proto_rs::bytes::Buf,
            ctx: ::proto_rs::encoding::DecodeContext,
        ) -> Result<(), ::proto_rs::DecodeError> {
            ::proto_rs::encoding::check_wire_type(::proto_rs::encoding::WireType::LengthDelimited, wire_type)?;
            ctx.limit_reached()?;
            ::proto_rs::encoding::merge_loop(value, buf, ctx.enter_recursion(), |msg: &mut Self, buf, ctx| {
                let (tag, wire_type) = ::proto_rs::encoding::decode_key(buf)?;
                match tag {
                    1u32 => {
                        <K as ::proto_rs::ProtoWire>::decode_into(wire_type, &mut msg.key, buf, ctx)?;
                        Ok(())
                    }
                    2u32 => {
                        <V as ::proto_rs::ProtoWire>::decode_into(wire_type, &mut msg.value, buf, ctx)?;
                        Ok(())
                    }
                    _ => ::proto_rs::encoding::skip_field(wire_type, tag, buf, ctx),
                }
            })
        }
    }
    impl<K, V> Pair<K, V> {
        #[cfg(feature = "build-schemas")]
        const PROTO_SCHEMA_GENERIC_BOUNDS_PAIR_PAIR_0: &[&str] = &[];
        #[cfg(feature = "build-schemas")]
        const PROTO_SCHEMA_GENERIC_PAIR_PAIR_0: ::proto_rs::schemas::Generic = ::proto_rs::schemas::Generic {
            name: "K",
            kind: ::proto_rs::schemas::GenericKind::Type,
            constraints: Self::PROTO_SCHEMA_GENERIC_BOUNDS_PAIR_PAIR_0,
            const_type: ::core::option::Option::None,
        };
        #[cfg(feature = "build-schemas")]
        const PROTO_SCHEMA_GENERIC_BOUNDS_PAIR_PAIR_1: &[&str] = &[];
        #[cfg(feature = "build-schemas")]
        const PROTO_SCHEMA_GENERIC_PAIR_PAIR_1: ::proto_rs::schemas::Generic = ::proto_rs::schemas::Generic {
            name: "V",
            kind: ::proto_rs::schemas::GenericKind::Type,
            constraints: Self::PROTO_SCHEMA_GENERIC_BOUNDS_PAIR_PAIR_1,
            const_type: ::core::option::Option::None,
        };
        #[cfg(feature = "build-schemas")]
        const PROTO_SCHEMA_ATTR_PAIR_PAIR_0: ::proto_rs::schemas::Attribute = ::proto_rs::schemas::Attribute {
            path: "derive",
            tokens: "# [derive (Debug , PartialEq)]",
        };
        #[cfg(feature = "build-schemas")]
        const PROTO_SCHEMA_FIELD_PAIR_PAIR_0: ::proto_rs::schemas::Field = ::proto_rs::schemas::Field {
            name: ::core::option::Option::Some("key"),
            proto_ident: ::proto_rs::schemas::ProtoIdent {
                module_path: "",
                name: "K",
                proto_package_name: "",
                proto_file_path: "",
                proto_type: "K",
            },
            rust_proto_ident: ::proto_rs::schemas::ProtoIdent {
                module_path: "",
                name: "K",
                proto_package_name: "",
                proto_file_path: "",
                proto_type: "K",
            },
            generic_args: &[],
            proto_label: ::proto_rs::schemas::ProtoLabel::None,
            tag: 1u32,
            attributes: &[],
            array_len: ::core::option::Option::None,
            array_is_bytes: false,
            array_elem: ::core::option::Option::None,
        };
        #[cfg(feature = "build-schemas")]
        const PROTO_SCHEMA_FIELD_PAIR_PAIR_1: ::proto_rs::schemas::Field = ::proto_rs::schemas::Field {
            name: ::core::option::Option::Some("value"),
            proto_ident: ::proto_rs::schemas::ProtoIdent {
                module_path: "",
                name: "V",
                proto_package_name: "",
                proto_file_path: "",
                proto_type: "V",
            },
            rust_proto_ident: ::proto_rs::schemas::ProtoIdent {
                module_path: "",
                name: "V",
                proto_package_name: "",
                proto_file_path: "",
                proto_type: "V",
            },
            generic_args: &[],
            proto_label: ::proto_rs::schemas::ProtoLabel::None,
            tag: 2u32,
            attributes: &[],
            array_len: ::core::option::Option::None,
            array_is_bytes: false,
            array_elem: ::core::option::Option::None,
        };
        #[cfg(feature = "build-schemas")]
        pub const PROTO_SCHEMA_PAIR_PAIR: ::proto_rs::schemas::ProtoSchema = ::proto_rs::schemas::ProtoSchema {
            id: ::proto_rs::schemas::ProtoIdent {
                module_path: ::core::module_path!(),
                name: "Pair",
                proto_package_name: "",
                proto_file_path: "",
                proto_type: "Pair",
            },
            generics: &[Self::PROTO_SCHEMA_GENERIC_PAIR_PAIR_0, Self::PROTO_SCHEMA_GENERIC_PAIR_PAIR_1],
            lifetimes: &[],
            top_level_attributes: &[Self::PROTO_SCHEMA_ATTR_PAIR_PAIR_0],
            content: ::proto_rs::schemas::ProtoEntry::Struct {
                fields: &[&Self::PROTO_SCHEMA_FIELD_PAIR_PAIR_0, &Self::PROTO_SCHEMA_FIELD_PAIR_PAIR_1],
            },
        };
        #[cfg(feature = "build-schemas")]
        const _REGISTRY_PROTO_SCHEMA_PAIR_PAIR: () = {
            #[allow(non_upper_case_globals)]
            const _: () = {
                static __INVENTORY: inventory::Node = inventory::Node {
                    value: &{ Pair::<K, V>::PROTO_SCHEMA_PAIR_PAIR },
                    next: inventory::core::cell::UnsafeCell::new(inventory::core::option::Option::None),
                    #[cfg(target_family = "wasm")]
                    initialized: inventory::core::sync::atomic::AtomicBool::new(false),
                };
                #[cfg_attr(any(target_os = "linux", target_os = "android"), unsafe(link_section = ".text.startup"))]
                unsafe extern "C" fn __ctor() {
                    unsafe { inventory::ErasedNode::submit(__INVENTORY.value, &__INVENTORY) }
                }
                #[used]
                #[cfg_attr(
                    all(
                        not(target_family = "wasm"),
                        any(
                            target_os = "linux",
                            target_os = "android",
                            target_os = "dragonfly",
                            target_os = "freebsd",
                            target_os = "haiku",
                            target_os = "illumos",
                            target_os = "netbsd",
                            target_os = "openbsd",
                            target_os = "none",
                        )
                    ),
                    unsafe(link_section = ".init_array")
                )]
                #[cfg_attr(target_family = "wasm",inventory::__private::attr(any(all(stable,since(1.85)),since(2024-12-18)),link_section = ".init_array",),)]
                #[cfg_attr(any(target_os = "macos", target_os = "ios"), link_section = "__DATA,__mod_init_func,mod_init_funcs")]
                #[cfg_attr(windows, link_section = ".CRT$XCU")]
                static __CTOR: unsafe extern "C" fn() = __ctor;
            };
        };
    }
    #[cfg(feature = "build-schemas")]
    impl<K, V> ::proto_rs::schemas::ProtoIdentifiable for Pair<K, V> {
        const PROTO_IDENT: ::proto_rs::schemas::ProtoIdent = ::proto_rs::schemas::ProtoIdent {
            module_path: ::core::module_path!(),
            name: "Pair",
            proto_package_name: "",
            proto_file_path: "",
            proto_type: "Pair",
        };
    }
}

#[proto_message]
#[derive(Debug, PartialEq)]
struct Lru<K, V, const CAP: usize> {
    items: VecDeque<Pair<K, V>>,
}

#[test]
fn generic_const_message_roundtrip() {
    let mut items = VecDeque::new();
    items.push_back(Pair { key: 10u32, value: 20u64 });
    items.push_back(Pair { key: 11u32, value: 21u64 });

    let lru = Lru::<u32, u64, 8> { items };
    let encoded = <Lru<u32, u64, 8> as ProtoExt>::encode_to_vec(&lru);
    let decoded = <Lru<u32, u64, 8> as ProtoExt>::decode(Bytes::from(encoded)).expect("decode");

    assert_eq!(lru, decoded);
}
