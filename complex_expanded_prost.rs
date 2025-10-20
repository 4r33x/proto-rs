// Recursive expansion of prost::Message macro
// ============================================

impl ::prost::Message for ComplexRootProst {
    #[allow(unused_variables)]
    fn encode_raw(&self, buf: &mut impl ::prost::bytes::BufMut) {
        if self.id != "" {
            ::prost::encoding::string::encode(1u32, &self.id, buf);
        }
        if self.payload != b"" as &[u8] {
            ::prost::encoding::bytes::encode(2u32, &self.payload, buf);
        }
        for msg in &self.leaves {
            ::prost::encoding::message::encode(3u32, msg, buf);
        }
        for msg in &self.deep_list {
            ::prost::encoding::message::encode(4u32, msg, buf);
        }
        ::prost::encoding::hash_map::encode(
            ::prost::encoding::string::encode,
            ::prost::encoding::string::encoded_len,
            ::prost::encoding::message::encode,
            ::prost::encoding::message::encoded_len,
            5u32,
            &self.leaf_lookup,
            buf,
        );
        ::prost::encoding::hash_map::encode(
            ::prost::encoding::string::encode,
            ::prost::encoding::string::encoded_len,
            ::prost::encoding::message::encode,
            ::prost::encoding::message::encoded_len,
            6u32,
            &self.deep_lookup,
            buf,
        );
        if let Some(ref msg) = self.status {
            ::prost::encoding::message::encode(7u32, msg, buf);
        }
        for msg in &self.status_history {
            ::prost::encoding::message::encode(8u32, msg, buf);
        }
        ::prost::encoding::hash_map::encode(
            ::prost::encoding::string::encode,
            ::prost::encoding::string::encoded_len,
            ::prost::encoding::message::encode,
            ::prost::encoding::message::encoded_len,
            9u32,
            &self.status_lookup,
            buf,
        );
        ::prost::encoding::int32::encode_packed(10u32, &self.codes, buf);
        ::prost::encoding::hash_map::encode_with_default(
            ::prost::encoding::string::encode,
            ::prost::encoding::string::encoded_len,
            ::prost::encoding::int32::encode,
            ::prost::encoding::int32::encoded_len,
            &(SimpleEnumProst::default() as i32),
            11u32,
            &self.code_lookup,
            buf,
        );
        ::prost::encoding::bytes::encode_repeated(12u32, &self.attachments, buf);
        ::prost::encoding::string::encode_repeated(13u32, &self.tags, buf);
        if self.count != 0i64 {
            ::prost::encoding::int64::encode(14u32, &self.count, buf);
        }
        if self.ratio != 0f64 {
            ::prost::encoding::double::encode(15u32, &self.ratio, buf);
        }
        if self.active != false {
            ::prost::encoding::bool::encode(16u32, &self.active, buf);
        }
        ::prost::encoding::uint64::encode_packed(17u32, &self.big_numbers, buf);
        ::prost::encoding::hash_map::encode(
            ::prost::encoding::string::encode,
            ::prost::encoding::string::encoded_len,
            ::prost::encoding::message::encode,
            ::prost::encoding::message::encoded_len,
            18u32,
            &self.audit_log,
            buf,
        );
        if let Some(ref msg) = self.primary_focus {
            ::prost::encoding::message::encode(19u32, msg, buf);
        }
        if let Some(ref msg) = self.secondary_focus {
            ::prost::encoding::message::encode(20u32, msg, buf);
        }
    }
    #[allow(unused_variables)]
    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: ::prost::encoding::wire_type::WireType,
        buf: &mut impl ::prost::bytes::Buf,
        ctx: ::prost::encoding::DecodeContext,
    ) -> ::core::result::Result<(), ::prost::DecodeError> {
        const STRUCT_NAME: &'static str = stringify!(ComplexRootProst);
        match tag {
            1u32 => {
                let mut value = &mut self.id;
                ::prost::encoding::string::merge(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, stringify!(id));
                    error
                })
            }
            2u32 => {
                let mut value = &mut self.payload;
                ::prost::encoding::bytes::merge(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, stringify!(payload));
                    error
                })
            }
            3u32 => {
                let mut value = &mut self.leaves;
                ::prost::encoding::message::merge_repeated(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, stringify!(leaves));
                    error
                })
            }
            4u32 => {
                let mut value = &mut self.deep_list;
                ::prost::encoding::message::merge_repeated(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, stringify!(deep_list));
                    error
                })
            }
            5u32 => {
                let mut value = &mut self.leaf_lookup;
                ::prost::encoding::hash_map::merge(::prost::encoding::string::merge, ::prost::encoding::message::merge, &mut value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, stringify!(leaf_lookup));
                    error
                })
            }
            6u32 => {
                let mut value = &mut self.deep_lookup;
                ::prost::encoding::hash_map::merge(::prost::encoding::string::merge, ::prost::encoding::message::merge, &mut value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, stringify!(deep_lookup));
                    error
                })
            }
            7u32 => {
                let mut value = &mut self.status;
                ::prost::encoding::message::merge(wire_type, value.get_or_insert_with(::core::default::Default::default), buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, stringify!(status));
                    error
                })
            }
            8u32 => {
                let mut value = &mut self.status_history;
                ::prost::encoding::message::merge_repeated(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, stringify!(status_history));
                    error
                })
            }
            9u32 => {
                let mut value = &mut self.status_lookup;
                ::prost::encoding::hash_map::merge(::prost::encoding::string::merge, ::prost::encoding::message::merge, &mut value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, stringify!(status_lookup));
                    error
                })
            }
            10u32 => {
                let mut value = &mut self.codes;
                ::prost::encoding::int32::merge_repeated(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, stringify!(codes));
                    error
                })
            }
            11u32 => {
                let mut value = &mut self.code_lookup;
                ::prost::encoding::hash_map::merge_with_default(
                    ::prost::encoding::string::merge,
                    ::prost::encoding::int32::merge,
                    SimpleEnumProst::default() as i32,
                    &mut value,
                    buf,
                    ctx,
                )
                .map_err(|mut error| {
                    error.push(STRUCT_NAME, stringify!(code_lookup));
                    error
                })
            }
            12u32 => {
                let mut value = &mut self.attachments;
                ::prost::encoding::bytes::merge_repeated(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, stringify!(attachments));
                    error
                })
            }
            13u32 => {
                let mut value = &mut self.tags;
                ::prost::encoding::string::merge_repeated(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, stringify!(tags));
                    error
                })
            }
            14u32 => {
                let mut value = &mut self.count;
                ::prost::encoding::int64::merge(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, stringify!(count));
                    error
                })
            }
            15u32 => {
                let mut value = &mut self.ratio;
                ::prost::encoding::double::merge(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, stringify!(ratio));
                    error
                })
            }
            16u32 => {
                let mut value = &mut self.active;
                ::prost::encoding::bool::merge(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, stringify!(active));
                    error
                })
            }
            17u32 => {
                let mut value = &mut self.big_numbers;
                ::prost::encoding::uint64::merge_repeated(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, stringify!(big_numbers));
                    error
                })
            }
            18u32 => {
                let mut value = &mut self.audit_log;
                ::prost::encoding::hash_map::merge(::prost::encoding::string::merge, ::prost::encoding::message::merge, &mut value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, stringify!(audit_log));
                    error
                })
            }
            19u32 => {
                let mut value = &mut self.primary_focus;
                ::prost::encoding::message::merge(wire_type, value.get_or_insert_with(::core::default::Default::default), buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, stringify!(primary_focus));
                    error
                })
            }
            20u32 => {
                let mut value = &mut self.secondary_focus;
                ::prost::encoding::message::merge(wire_type, value.get_or_insert_with(::core::default::Default::default), buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, stringify!(secondary_focus));
                    error
                })
            }
            _ => ::prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }
    #[inline]
    fn encoded_len(&self) -> usize {
        0 + if self.id != "" { ::prost::encoding::string::encoded_len(1u32, &self.id) } else { 0 }
            + if self.payload != b"" as &[u8] {
                ::prost::encoding::bytes::encoded_len(2u32, &self.payload)
            } else {
                0
            }
            + ::prost::encoding::message::encoded_len_repeated(3u32, &self.leaves)
            + ::prost::encoding::message::encoded_len_repeated(4u32, &self.deep_list)
            + ::prost::encoding::hash_map::encoded_len(::prost::encoding::string::encoded_len, ::prost::encoding::message::encoded_len, 5u32, &self.leaf_lookup)
            + ::prost::encoding::hash_map::encoded_len(::prost::encoding::string::encoded_len, ::prost::encoding::message::encoded_len, 6u32, &self.deep_lookup)
            + self.status.as_ref().map_or(0, |msg| ::prost::encoding::message::encoded_len(7u32, msg))
            + ::prost::encoding::message::encoded_len_repeated(8u32, &self.status_history)
            + ::prost::encoding::hash_map::encoded_len(::prost::encoding::string::encoded_len, ::prost::encoding::message::encoded_len, 9u32, &self.status_lookup)
            + ::prost::encoding::int32::encoded_len_packed(10u32, &self.codes)
            + ::prost::encoding::hash_map::encoded_len_with_default(
                ::prost::encoding::string::encoded_len,
                ::prost::encoding::int32::encoded_len,
                &(SimpleEnumProst::default() as i32),
                11u32,
                &self.code_lookup,
            )
            + ::prost::encoding::bytes::encoded_len_repeated(12u32, &self.attachments)
            + ::prost::encoding::string::encoded_len_repeated(13u32, &self.tags)
            + if self.count != 0i64 { ::prost::encoding::int64::encoded_len(14u32, &self.count) } else { 0 }
            + if self.ratio != 0f64 { ::prost::encoding::double::encoded_len(15u32, &self.ratio) } else { 0 }
            + if self.active != false { ::prost::encoding::bool::encoded_len(16u32, &self.active) } else { 0 }
            + ::prost::encoding::uint64::encoded_len_packed(17u32, &self.big_numbers)
            + ::prost::encoding::hash_map::encoded_len(::prost::encoding::string::encoded_len, ::prost::encoding::message::encoded_len, 18u32, &self.audit_log)
            + self.primary_focus.as_ref().map_or(0, |msg| ::prost::encoding::message::encoded_len(19u32, msg))
            + self.secondary_focus.as_ref().map_or(0, |msg| ::prost::encoding::message::encoded_len(20u32, msg))
    }
    fn clear(&mut self) {
        self.id.clear();
        self.payload.clear();
        self.leaves.clear();
        self.deep_list.clear();
        self.leaf_lookup.clear();
        self.deep_lookup.clear();
        self.status = ::core::option::Option::None;
        self.status_history.clear();
        self.status_lookup.clear();
        self.codes.clear();
        self.code_lookup.clear();
        self.attachments.clear();
        self.tags.clear();
        self.count = 0i64;
        self.ratio = 0f64;
        self.active = false;
        self.big_numbers.clear();
        self.audit_log.clear();
        self.primary_focus = ::core::option::Option::None;
        self.secondary_focus = ::core::option::Option::None;
    }
}
impl ::core::default::Default for ComplexRootProst {
    fn default() -> Self {
        ComplexRootProst {
            id: ::prost::alloc::string::String::new(),
            payload: ::core::default::Default::default(),
            leaves: ::core::default::Default::default(),
            deep_list: ::core::default::Default::default(),
            leaf_lookup: ::core::default::Default::default(),
            deep_lookup: ::core::default::Default::default(),
            status: ::core::default::Default::default(),
            status_history: ::core::default::Default::default(),
            status_lookup: ::core::default::Default::default(),
            codes: ::prost::alloc::vec::Vec::new(),
            code_lookup: ::core::default::Default::default(),
            attachments: ::prost::alloc::vec::Vec::new(),
            tags: ::prost::alloc::vec::Vec::new(),
            count: 0i64,
            ratio: 0f64,
            active: false,
            big_numbers: ::prost::alloc::vec::Vec::new(),
            audit_log: ::core::default::Default::default(),
            primary_focus: ::core::default::Default::default(),
            secondary_focus: ::core::default::Default::default(),
        }
    }
}
impl ::core::fmt::Debug for ComplexRootProst {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        let mut builder = f.debug_struct(stringify!(ComplexRootProst));
        let builder = {
            let wrapper = {
                #[allow(non_snake_case)]
                fn ScalarWrapper<T>(v: T) -> T {
                    v
                }
                ScalarWrapper(&self.id)
            };
            builder.field(stringify!(id), &wrapper)
        };
        let builder = {
            let wrapper = {
                #[allow(non_snake_case)]
                fn ScalarWrapper<T>(v: T) -> T {
                    v
                }
                ScalarWrapper(&self.payload)
            };
            builder.field(stringify!(payload), &wrapper)
        };
        let builder = {
            let wrapper = &self.leaves;
            builder.field(stringify!(leaves), &wrapper)
        };
        let builder = {
            let wrapper = &self.deep_list;
            builder.field(stringify!(deep_list), &wrapper)
        };
        let builder = {
            let wrapper = {
                struct MapWrapper<'a, V: 'a>(&'a ::std::collections::HashMap<::prost::alloc::string::String, V>);

                impl<'a, V> ::core::fmt::Debug for MapWrapper<'a, V>
                where
                    V: ::core::fmt::Debug + 'a,
                {
                    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                        #[allow(non_snake_case)]
                        fn KeyWrapper<T>(v: T) -> T {
                            v
                        }
                        fn ValueWrapper<T>(v: T) -> T {
                            v
                        }
                        let mut builder = f.debug_map();
                        for (k, v) in self.0 {
                            builder.entry(&KeyWrapper(k), &ValueWrapper(v));
                        }
                        builder.finish()
                    }
                }
                MapWrapper(&self.leaf_lookup)
            };
            builder.field(stringify!(leaf_lookup), &wrapper)
        };
        let builder = {
            let wrapper = {
                struct MapWrapper<'a, V: 'a>(&'a ::std::collections::HashMap<::prost::alloc::string::String, V>);

                impl<'a, V> ::core::fmt::Debug for MapWrapper<'a, V>
                where
                    V: ::core::fmt::Debug + 'a,
                {
                    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                        #[allow(non_snake_case)]
                        fn KeyWrapper<T>(v: T) -> T {
                            v
                        }
                        fn ValueWrapper<T>(v: T) -> T {
                            v
                        }
                        let mut builder = f.debug_map();
                        for (k, v) in self.0 {
                            builder.entry(&KeyWrapper(k), &ValueWrapper(v));
                        }
                        builder.finish()
                    }
                }
                MapWrapper(&self.deep_lookup)
            };
            builder.field(stringify!(deep_lookup), &wrapper)
        };
        let builder = {
            let wrapper = &self.status;
            builder.field(stringify!(status), &wrapper)
        };
        let builder = {
            let wrapper = &self.status_history;
            builder.field(stringify!(status_history), &wrapper)
        };
        let builder = {
            let wrapper = {
                struct MapWrapper<'a, V: 'a>(&'a ::std::collections::HashMap<::prost::alloc::string::String, V>);

                impl<'a, V> ::core::fmt::Debug for MapWrapper<'a, V>
                where
                    V: ::core::fmt::Debug + 'a,
                {
                    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                        #[allow(non_snake_case)]
                        fn KeyWrapper<T>(v: T) -> T {
                            v
                        }
                        fn ValueWrapper<T>(v: T) -> T {
                            v
                        }
                        let mut builder = f.debug_map();
                        for (k, v) in self.0 {
                            builder.entry(&KeyWrapper(k), &ValueWrapper(v));
                        }
                        builder.finish()
                    }
                }
                MapWrapper(&self.status_lookup)
            };
            builder.field(stringify!(status_lookup), &wrapper)
        };
        let builder = {
            let wrapper = {
                struct ScalarWrapper<'a>(&'a ::prost::alloc::vec::Vec<i32>);

                impl<'a> ::core::fmt::Debug for ScalarWrapper<'a> {
                    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                        let mut vec_builder = f.debug_list();
                        for v in self.0 {
                            struct Inner<'a>(&'a i32);

                            impl<'a> ::core::fmt::Debug for Inner<'a> {
                                fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                                    let res: ::core::result::Result<SimpleEnumProst, _> = ::core::convert::TryFrom::try_from(*self.0);
                                    match res {
                                        Err(_) => ::core::fmt::Debug::fmt(&self.0, f),
                                        Ok(en) => ::core::fmt::Debug::fmt(&en, f),
                                    }
                                }
                            }
                            vec_builder.entry(&Inner(v));
                        }
                        vec_builder.finish()
                    }
                }
                ScalarWrapper(&self.codes)
            };
            builder.field(stringify!(codes), &wrapper)
        };
        let builder = {
            let wrapper = {
                struct MapWrapper<'a>(&'a ::std::collections::HashMap<::prost::alloc::string::String, i32>);

                impl<'a> ::core::fmt::Debug for MapWrapper<'a> {
                    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                        #[allow(non_snake_case)]
                        fn KeyWrapper<T>(v: T) -> T {
                            v
                        }
                        struct ValueWrapper<'a>(&'a i32);

                        impl<'a> ::core::fmt::Debug for ValueWrapper<'a> {
                            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                                let res: ::core::result::Result<SimpleEnumProst, _> = ::core::convert::TryFrom::try_from(*self.0);
                                match res {
                                    Err(_) => ::core::fmt::Debug::fmt(&self.0, f),
                                    Ok(en) => ::core::fmt::Debug::fmt(&en, f),
                                }
                            }
                        }
                        let mut builder = f.debug_map();
                        for (k, v) in self.0 {
                            builder.entry(&KeyWrapper(k), &ValueWrapper(v));
                        }
                        builder.finish()
                    }
                }
                MapWrapper(&self.code_lookup)
            };
            builder.field(stringify!(code_lookup), &wrapper)
        };
        let builder = {
            let wrapper = {
                struct ScalarWrapper<'a>(&'a ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>);

                impl<'a> ::core::fmt::Debug for ScalarWrapper<'a> {
                    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                        let mut vec_builder = f.debug_list();
                        for v in self.0 {
                            #[allow(non_snake_case)]
                            fn Inner<T>(v: T) -> T {
                                v
                            }
                            vec_builder.entry(&Inner(v));
                        }
                        vec_builder.finish()
                    }
                }
                ScalarWrapper(&self.attachments)
            };
            builder.field(stringify!(attachments), &wrapper)
        };
        let builder = {
            let wrapper = {
                struct ScalarWrapper<'a>(&'a ::prost::alloc::vec::Vec<::prost::alloc::string::String>);

                impl<'a> ::core::fmt::Debug for ScalarWrapper<'a> {
                    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                        let mut vec_builder = f.debug_list();
                        for v in self.0 {
                            #[allow(non_snake_case)]
                            fn Inner<T>(v: T) -> T {
                                v
                            }
                            vec_builder.entry(&Inner(v));
                        }
                        vec_builder.finish()
                    }
                }
                ScalarWrapper(&self.tags)
            };
            builder.field(stringify!(tags), &wrapper)
        };
        let builder = {
            let wrapper = {
                #[allow(non_snake_case)]
                fn ScalarWrapper<T>(v: T) -> T {
                    v
                }
                ScalarWrapper(&self.count)
            };
            builder.field(stringify!(count), &wrapper)
        };
        let builder = {
            let wrapper = {
                #[allow(non_snake_case)]
                fn ScalarWrapper<T>(v: T) -> T {
                    v
                }
                ScalarWrapper(&self.ratio)
            };
            builder.field(stringify!(ratio), &wrapper)
        };
        let builder = {
            let wrapper = {
                #[allow(non_snake_case)]
                fn ScalarWrapper<T>(v: T) -> T {
                    v
                }
                ScalarWrapper(&self.active)
            };
            builder.field(stringify!(active), &wrapper)
        };
        let builder = {
            let wrapper = {
                struct ScalarWrapper<'a>(&'a ::prost::alloc::vec::Vec<u64>);

                impl<'a> ::core::fmt::Debug for ScalarWrapper<'a> {
                    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                        let mut vec_builder = f.debug_list();
                        for v in self.0 {
                            #[allow(non_snake_case)]
                            fn Inner<T>(v: T) -> T {
                                v
                            }
                            vec_builder.entry(&Inner(v));
                        }
                        vec_builder.finish()
                    }
                }
                ScalarWrapper(&self.big_numbers)
            };
            builder.field(stringify!(big_numbers), &wrapper)
        };
        let builder = {
            let wrapper = {
                struct MapWrapper<'a, V: 'a>(&'a ::std::collections::HashMap<::prost::alloc::string::String, V>);

                impl<'a, V> ::core::fmt::Debug for MapWrapper<'a, V>
                where
                    V: ::core::fmt::Debug + 'a,
                {
                    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                        #[allow(non_snake_case)]
                        fn KeyWrapper<T>(v: T) -> T {
                            v
                        }
                        fn ValueWrapper<T>(v: T) -> T {
                            v
                        }
                        let mut builder = f.debug_map();
                        for (k, v) in self.0 {
                            builder.entry(&KeyWrapper(k), &ValueWrapper(v));
                        }
                        builder.finish()
                    }
                }
                MapWrapper(&self.audit_log)
            };
            builder.field(stringify!(audit_log), &wrapper)
        };
        let builder = {
            let wrapper = &self.primary_focus;
            builder.field(stringify!(primary_focus), &wrapper)
        };
        let builder = {
            let wrapper = &self.secondary_focus;
            builder.field(stringify!(secondary_focus), &wrapper)
        };
        builder.finish()
    }
}
#[allow(dead_code)]
impl ComplexRootProst {
    #[doc = "Returns an iterator which yields the valid enum values contained in `codes`."]
    pub fn codes(&self) -> ::core::iter::FilterMap<::core::iter::Cloned<::core::slice::Iter<i32>>, fn(i32) -> ::core::option::Option<SimpleEnumProst>> {
        self.codes.iter().cloned().filter_map(|x| {
            let result: ::core::result::Result<SimpleEnumProst, _> = ::core::convert::TryFrom::try_from(x);
            result.ok()
        })
    }
    #[doc = "Appends the provided enum value to `codes`."]
    pub fn push_codes(&mut self, value: SimpleEnumProst) {
        self.codes.push(value as i32);
    }
    #[doc = "Returns the enum value for the corresponding key in `code_lookup`, or `None` if the entry does not exist or it is not a valid enum value."]
    pub fn get_code_lookup(&self, key: &str) -> ::core::option::Option<SimpleEnumProst> {
        self.code_lookup.get(key).cloned().and_then(|x| {
            let result: ::core::result::Result<SimpleEnumProst, _> = ::core::convert::TryFrom::try_from(x);
            result.ok()
        })
    }
    #[doc = "Inserts a key value pair into `code_lookup`."]
    pub fn insert_code_lookup(&mut self, key: ::prost::alloc::string::String, value: SimpleEnumProst) -> ::core::option::Option<SimpleEnumProst> {
        self.code_lookup.insert(key, value as i32).and_then(|x| {
            let result: ::core::result::Result<SimpleEnumProst, _> = ::core::convert::TryFrom::try_from(x);
            result.ok()
        })
    }
}
