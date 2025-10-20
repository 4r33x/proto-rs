// Recursive expansion of proto_message macro
// ===========================================
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ComplexRoot {
    pub id: String,
    pub payload: Bytes,
    pub leaves: Vec<NestedLeaf>,
    pub deep_list: Vec<DeepMessage>,
    pub leaf_lookup: HashMap<String, NestedLeaf>,
    pub deep_lookup: HashMap<String, DeepMessage>,
    pub status: ComplexEnum,
    pub status_history: Vec<ComplexEnum>,
    pub status_lookup: HashMap<String, ComplexEnum>,
    pub codes: Vec<SimpleEnum>,
    pub code_lookup: HashMap<String, SimpleEnum>,
    pub attachments: Vec<Bytes>,
    pub tags: Vec<String>,
    pub count: i64,
    pub ratio: f64,
    pub active: bool,
    pub big_numbers: Vec<u64>,
    pub audit_log: HashMap<String, DeepMessage>,
    pub primary_focus: Option<Box<NestedLeaf>>,
    pub secondary_focus: Option<Box<DeepMessage>>,
}
impl ::proto_rs::ProtoShadow for ComplexRoot {
    type Sun<'a> = &'a Self;
    type OwnedSun = Self;
    type View<'a> = &'a Self;
    fn to_sun(self) -> Result<Self::OwnedSun, ::proto_rs::DecodeError> {
        Ok(self)
    }
    fn from_sun(value: Self::Sun<'_>) -> Self::View<'_> {
        value
    }
}
impl ::proto_rs::ProtoExt for ComplexRoot {
    type Shadow<'a> = ComplexRoot;
    #[inline]
    fn proto_default<'a>() -> Self::Shadow<'a> {
        ComplexRoot {
            id: <String as ::proto_rs::ProtoExt>::proto_default(),
            payload: <Bytes as ::proto_rs::ProtoExt>::proto_default(),
            leaves: Vec::new(),
            deep_list: Vec::new(),
            leaf_lookup: ::core::default::Default::default(),
            deep_lookup: ::core::default::Default::default(),
            status: <ComplexEnum as ::proto_rs::ProtoExt>::proto_default(),
            status_history: Vec::new(),
            status_lookup: ::core::default::Default::default(),
            codes: Vec::new(),
            code_lookup: ::core::default::Default::default(),
            attachments: Vec::new(),
            tags: Vec::new(),
            count: <i64 as ::proto_rs::ProtoExt>::proto_default(),
            ratio: <f64 as ::proto_rs::ProtoExt>::proto_default(),
            active: <bool as ::proto_rs::ProtoExt>::proto_default(),
            big_numbers: Vec::new(),
            audit_log: ::core::default::Default::default(),
            primary_focus: None,
            secondary_focus: None,
        }
    }
    fn encoded_len(value: &::proto_rs::ViewOf<'_, Self>) -> usize {
        let value: &Self = *value;
        0 + <String as ::proto_rs::SingularField>::encoded_len_singular_field(1u32, &&(value.id))
            + <Bytes as ::proto_rs::SingularField>::encoded_len_singular_field(2u32, &&(value.payload))
            + {
                let __proto_rs_views = (value.leaves)
                    .iter()
                    .map(|value| <<NestedLeaf as ::proto_rs::ProtoExt>::Shadow<'_> as ::proto_rs::ProtoShadow>::from_sun(value));
                <NestedLeaf as ::proto_rs::RepeatedField>::encoded_len_repeated_field(3u32, __proto_rs_views)
            }
            + {
                let __proto_rs_views = (value.deep_list)
                    .iter()
                    .map(|value| <<DeepMessage as ::proto_rs::ProtoExt>::Shadow<'_> as ::proto_rs::ProtoShadow>::from_sun(value));
                <DeepMessage as ::proto_rs::RepeatedField>::encoded_len_repeated_field(4u32, __proto_rs_views)
            }
            + ::proto_rs::encoding::hash_map::encoded_len(
                |tag, key| <String as ::proto_rs::SingularField>::encoded_len_singular_field(tag, &key),
                |tag, value| <NestedLeaf as ::proto_rs::SingularField>::encoded_len_singular_field(tag, &value),
                5u32,
                &(value.leaf_lookup),
            )
            + ::proto_rs::encoding::hash_map::encoded_len(
                |tag, key| <String as ::proto_rs::SingularField>::encoded_len_singular_field(tag, &key),
                |tag, value| <DeepMessage as ::proto_rs::SingularField>::encoded_len_singular_field(tag, &value),
                6u32,
                &(value.deep_lookup),
            )
            + <ComplexEnum as ::proto_rs::SingularField>::encoded_len_singular_field(7u32, &&(value.status))
            + {
                let __proto_rs_views = (value.status_history)
                    .iter()
                    .map(|value| <<ComplexEnum as ::proto_rs::ProtoExt>::Shadow<'_> as ::proto_rs::ProtoShadow>::from_sun(value));
                <ComplexEnum as ::proto_rs::RepeatedField>::encoded_len_repeated_field(8u32, __proto_rs_views)
            }
            + ::proto_rs::encoding::hash_map::encoded_len(
                |tag, key| <String as ::proto_rs::SingularField>::encoded_len_singular_field(tag, &key),
                |tag, value| <ComplexEnum as ::proto_rs::SingularField>::encoded_len_singular_field(tag, &value),
                9u32,
                &(value.status_lookup),
            )
            + {
                let __proto_rs_views = (value.codes)
                    .iter()
                    .map(|value| <<SimpleEnum as ::proto_rs::ProtoExt>::Shadow<'_> as ::proto_rs::ProtoShadow>::from_sun(value));
                <SimpleEnum as ::proto_rs::RepeatedField>::encoded_len_repeated_field(10u32, __proto_rs_views)
            }
            + ::proto_rs::encoding::hash_map::encoded_len(
                |tag, key| <String as ::proto_rs::SingularField>::encoded_len_singular_field(tag, &key),
                |tag, value| <SimpleEnum as ::proto_rs::SingularField>::encoded_len_singular_field(tag, &value),
                11u32,
                &(value.code_lookup),
            )
            + {
                let __proto_rs_views = (value.attachments)
                    .iter()
                    .map(|value| <<Bytes as ::proto_rs::ProtoExt>::Shadow<'_> as ::proto_rs::ProtoShadow>::from_sun(value));
                <Bytes as ::proto_rs::RepeatedField>::encoded_len_repeated_field(12u32, __proto_rs_views)
            }
            + {
                let __proto_rs_views = (value.tags)
                    .iter()
                    .map(|value| <<String as ::proto_rs::ProtoExt>::Shadow<'_> as ::proto_rs::ProtoShadow>::from_sun(value));
                <String as ::proto_rs::RepeatedField>::encoded_len_repeated_field(13u32, __proto_rs_views)
            }
            + <i64 as ::proto_rs::SingularField>::encoded_len_singular_field(14u32, &&(value.count))
            + <f64 as ::proto_rs::SingularField>::encoded_len_singular_field(15u32, &&(value.ratio))
            + <bool as ::proto_rs::SingularField>::encoded_len_singular_field(16u32, &&(value.active))
            + {
                if (value.big_numbers).is_empty() {
                    0
                } else {
                    let mut __proto_rs_body_len = 0usize;
                    for __proto_rs_value in (value.big_numbers).iter() {
                        let __proto_rs_converted: u64 = *__proto_rs_value;
                        __proto_rs_body_len += (::proto_rs::encoding::uint64::encoded_len(1u32, &__proto_rs_converted) - ::proto_rs::encoding::key_len(1u32));
                    }
                    ::proto_rs::encoding::key_len(17u32) + ::proto_rs::encoding::encoded_len_varint(__proto_rs_body_len as u64) + __proto_rs_body_len
                }
            }
            + ::proto_rs::encoding::hash_map::encoded_len(
                |tag, key| <String as ::proto_rs::SingularField>::encoded_len_singular_field(tag, &key),
                |tag, value| <DeepMessage as ::proto_rs::SingularField>::encoded_len_singular_field(tag, &value),
                18u32,
                &(value.audit_log),
            )
            + {
                let __proto_rs_value = (value.primary_focus)
                    .as_ref()
                    .map(|value| <<Box<NestedLeaf> as ::proto_rs::ProtoExt>::Shadow<'_> as ::proto_rs::ProtoShadow>::from_sun(value));
                <Box<NestedLeaf> as ::proto_rs::SingularField>::encoded_len_option_field(19u32, __proto_rs_value)
            }
            + {
                let __proto_rs_value = (value.secondary_focus)
                    .as_ref()
                    .map(|value| <<Box<DeepMessage> as ::proto_rs::ProtoExt>::Shadow<'_> as ::proto_rs::ProtoShadow>::from_sun(value));
                <Box<DeepMessage> as ::proto_rs::SingularField>::encoded_len_option_field(20u32, __proto_rs_value)
            }
    }
    fn encode_raw(value: ::proto_rs::ViewOf<'_, Self>, buf: &mut impl ::proto_rs::bytes::BufMut) {
        let value: &Self = value;
        <String as ::proto_rs::SingularField>::encode_singular_field(1u32, &(value.id), buf);
        <Bytes as ::proto_rs::SingularField>::encode_singular_field(2u32, &(value.payload), buf);
        {
            let __proto_rs_views = (value.leaves)
                .iter()
                .map(|value| <<NestedLeaf as ::proto_rs::ProtoExt>::Shadow<'_> as ::proto_rs::ProtoShadow>::from_sun(value));
            <NestedLeaf as ::proto_rs::RepeatedField>::encode_repeated_field(3u32, __proto_rs_views, buf);
        }
        {
            let __proto_rs_views = (value.deep_list)
                .iter()
                .map(|value| <<DeepMessage as ::proto_rs::ProtoExt>::Shadow<'_> as ::proto_rs::ProtoShadow>::from_sun(value));
            <DeepMessage as ::proto_rs::RepeatedField>::encode_repeated_field(4u32, __proto_rs_views, buf);
        }
        if !(value.leaf_lookup).is_empty() {
            ::proto_rs::encoding::hash_map::encode(
                |tag, key, buf| <String as ::proto_rs::SingularField>::encode_singular_field(tag, key, buf),
                |tag, key| <String as ::proto_rs::SingularField>::encoded_len_singular_field(tag, &key),
                |tag, value, buf| <NestedLeaf as ::proto_rs::SingularField>::encode_singular_field(tag, value, buf),
                |tag, value| <NestedLeaf as ::proto_rs::SingularField>::encoded_len_singular_field(tag, &value),
                5u32,
                &(value.leaf_lookup),
                buf,
            );
        }
        if !(value.deep_lookup).is_empty() {
            ::proto_rs::encoding::hash_map::encode(
                |tag, key, buf| <String as ::proto_rs::SingularField>::encode_singular_field(tag, key, buf),
                |tag, key| <String as ::proto_rs::SingularField>::encoded_len_singular_field(tag, &key),
                |tag, value, buf| <DeepMessage as ::proto_rs::SingularField>::encode_singular_field(tag, value, buf),
                |tag, value| <DeepMessage as ::proto_rs::SingularField>::encoded_len_singular_field(tag, &value),
                6u32,
                &(value.deep_lookup),
                buf,
            );
        }
        <ComplexEnum as ::proto_rs::SingularField>::encode_singular_field(7u32, &(value.status), buf);
        {
            let __proto_rs_views = (value.status_history)
                .iter()
                .map(|value| <<ComplexEnum as ::proto_rs::ProtoExt>::Shadow<'_> as ::proto_rs::ProtoShadow>::from_sun(value));
            <ComplexEnum as ::proto_rs::RepeatedField>::encode_repeated_field(8u32, __proto_rs_views, buf);
        }
        if !(value.status_lookup).is_empty() {
            ::proto_rs::encoding::hash_map::encode(
                |tag, key, buf| <String as ::proto_rs::SingularField>::encode_singular_field(tag, key, buf),
                |tag, key| <String as ::proto_rs::SingularField>::encoded_len_singular_field(tag, &key),
                |tag, value, buf| <ComplexEnum as ::proto_rs::SingularField>::encode_singular_field(tag, value, buf),
                |tag, value| <ComplexEnum as ::proto_rs::SingularField>::encoded_len_singular_field(tag, &value),
                9u32,
                &(value.status_lookup),
                buf,
            );
        }
        {
            let __proto_rs_views = (value.codes)
                .iter()
                .map(|value| <<SimpleEnum as ::proto_rs::ProtoExt>::Shadow<'_> as ::proto_rs::ProtoShadow>::from_sun(value));
            <SimpleEnum as ::proto_rs::RepeatedField>::encode_repeated_field(10u32, __proto_rs_views, buf);
        }
        if !(value.code_lookup).is_empty() {
            ::proto_rs::encoding::hash_map::encode(
                |tag, key, buf| <String as ::proto_rs::SingularField>::encode_singular_field(tag, key, buf),
                |tag, key| <String as ::proto_rs::SingularField>::encoded_len_singular_field(tag, &key),
                |tag, value, buf| <SimpleEnum as ::proto_rs::SingularField>::encode_singular_field(tag, value, buf),
                |tag, value| <SimpleEnum as ::proto_rs::SingularField>::encoded_len_singular_field(tag, &value),
                11u32,
                &(value.code_lookup),
                buf,
            );
        }
        {
            let __proto_rs_views = (value.attachments)
                .iter()
                .map(|value| <<Bytes as ::proto_rs::ProtoExt>::Shadow<'_> as ::proto_rs::ProtoShadow>::from_sun(value));
            <Bytes as ::proto_rs::RepeatedField>::encode_repeated_field(12u32, __proto_rs_views, buf);
        }
        {
            let __proto_rs_views = (value.tags)
                .iter()
                .map(|value| <<String as ::proto_rs::ProtoExt>::Shadow<'_> as ::proto_rs::ProtoShadow>::from_sun(value));
            <String as ::proto_rs::RepeatedField>::encode_repeated_field(13u32, __proto_rs_views, buf);
        }
        <i64 as ::proto_rs::SingularField>::encode_singular_field(14u32, &(value.count), buf);
        <f64 as ::proto_rs::SingularField>::encode_singular_field(15u32, &(value.ratio), buf);
        <bool as ::proto_rs::SingularField>::encode_singular_field(16u32, &(value.active), buf);
        {
            if !(value.big_numbers).is_empty() {
                let __proto_rs_body_len = {
                    let mut __proto_rs_body_len = 0usize;
                    for __proto_rs_value in (value.big_numbers).iter() {
                        let __proto_rs_converted: u64 = *__proto_rs_value;
                        __proto_rs_body_len += (::proto_rs::encoding::uint64::encoded_len(1u32, &__proto_rs_converted) - ::proto_rs::encoding::key_len(1u32));
                    }
                    __proto_rs_body_len
                };
                ::proto_rs::encoding::encode_key(17u32, ::proto_rs::encoding::WireType::LengthDelimited, buf);
                ::proto_rs::encoding::encode_varint(__proto_rs_body_len as u64, buf);
                for __proto_rs_value in (value.big_numbers).iter() {
                    let __proto_rs_converted: u64 = *__proto_rs_value;
                    ::proto_rs::encoding::encode_varint(__proto_rs_converted as u64, buf);
                }
            }
        }
        if !(value.audit_log).is_empty() {
            ::proto_rs::encoding::hash_map::encode(
                |tag, key, buf| <String as ::proto_rs::SingularField>::encode_singular_field(tag, key, buf),
                |tag, key| <String as ::proto_rs::SingularField>::encoded_len_singular_field(tag, &key),
                |tag, value, buf| <DeepMessage as ::proto_rs::SingularField>::encode_singular_field(tag, value, buf),
                |tag, value| <DeepMessage as ::proto_rs::SingularField>::encoded_len_singular_field(tag, &value),
                18u32,
                &(value.audit_log),
                buf,
            );
        }
        {
            let __proto_rs_value = (value.primary_focus)
                .as_ref()
                .map(|value| <<Box<NestedLeaf> as ::proto_rs::ProtoExt>::Shadow<'_> as ::proto_rs::ProtoShadow>::from_sun(value));
            <Box<NestedLeaf> as ::proto_rs::SingularField>::encode_option_field(19u32, __proto_rs_value, buf);
        }
        {
            let __proto_rs_value = (value.secondary_focus)
                .as_ref()
                .map(|value| <<Box<DeepMessage> as ::proto_rs::ProtoExt>::Shadow<'_> as ::proto_rs::ProtoShadow>::from_sun(value));
            <Box<DeepMessage> as ::proto_rs::SingularField>::encode_option_field(20u32, __proto_rs_value, buf);
        }
    }
    fn merge_field(
        shadow: &mut Self::Shadow<'_>,
        tag: u32,
        wire_type: ::proto_rs::encoding::WireType,
        buf: &mut impl ::proto_rs::bytes::Buf,
        ctx: ::proto_rs::encoding::DecodeContext,
    ) -> Result<(), ::proto_rs::DecodeError> {
        use ::proto_rs::bytes::Buf;
        match tag {
            1u32 => {
                <String as ::proto_rs::SingularField>::merge_singular_field(wire_type, &mut (shadow.id), buf, ctx.clone())?;
                Ok(())
            }
            2u32 => {
                <Bytes as ::proto_rs::SingularField>::merge_singular_field(wire_type, &mut (shadow.payload), buf, ctx.clone())?;
                Ok(())
            }
            3u32 => {
                <NestedLeaf as ::proto_rs::RepeatedField>::merge_repeated_field(wire_type, &mut (shadow.leaves), buf, ctx.clone())?;
                Ok(())
            }
            4u32 => {
                <DeepMessage as ::proto_rs::RepeatedField>::merge_repeated_field(wire_type, &mut (shadow.deep_list), buf, ctx.clone())?;
                Ok(())
            }
            5u32 => {
                ::proto_rs::encoding::hash_map::merge(
                    |wire_type, key, buf, ctx| <String as ::proto_rs::SingularField>::merge_singular_field(wire_type, key, buf, ctx),
                    |wire_type, value, buf, ctx| <NestedLeaf as ::proto_rs::SingularField>::merge_singular_field(wire_type, value, buf, ctx),
                    &mut (shadow.leaf_lookup),
                    buf,
                    ctx.clone(),
                )?;
                Ok(())
            }
            6u32 => {
                ::proto_rs::encoding::hash_map::merge(
                    |wire_type, key, buf, ctx| <String as ::proto_rs::SingularField>::merge_singular_field(wire_type, key, buf, ctx),
                    |wire_type, value, buf, ctx| <DeepMessage as ::proto_rs::SingularField>::merge_singular_field(wire_type, value, buf, ctx),
                    &mut (shadow.deep_lookup),
                    buf,
                    ctx.clone(),
                )?;
                Ok(())
            }
            7u32 => {
                <ComplexEnum as ::proto_rs::SingularField>::merge_singular_field(wire_type, &mut (shadow.status), buf, ctx.clone())?;
                Ok(())
            }
            8u32 => {
                <ComplexEnum as ::proto_rs::RepeatedField>::merge_repeated_field(wire_type, &mut (shadow.status_history), buf, ctx.clone())?;
                Ok(())
            }
            9u32 => {
                ::proto_rs::encoding::hash_map::merge(
                    |wire_type, key, buf, ctx| <String as ::proto_rs::SingularField>::merge_singular_field(wire_type, key, buf, ctx),
                    |wire_type, value, buf, ctx| <ComplexEnum as ::proto_rs::SingularField>::merge_singular_field(wire_type, value, buf, ctx),
                    &mut (shadow.status_lookup),
                    buf,
                    ctx.clone(),
                )?;
                Ok(())
            }
            10u32 => {
                <SimpleEnum as ::proto_rs::RepeatedField>::merge_repeated_field(wire_type, &mut (shadow.codes), buf, ctx.clone())?;
                Ok(())
            }
            11u32 => {
                ::proto_rs::encoding::hash_map::merge(
                    |wire_type, key, buf, ctx| <String as ::proto_rs::SingularField>::merge_singular_field(wire_type, key, buf, ctx),
                    |wire_type, value, buf, ctx| <SimpleEnum as ::proto_rs::SingularField>::merge_singular_field(wire_type, value, buf, ctx),
                    &mut (shadow.code_lookup),
                    buf,
                    ctx.clone(),
                )?;
                Ok(())
            }
            12u32 => {
                <Bytes as ::proto_rs::RepeatedField>::merge_repeated_field(wire_type, &mut (shadow.attachments), buf, ctx.clone())?;
                Ok(())
            }
            13u32 => {
                <String as ::proto_rs::RepeatedField>::merge_repeated_field(wire_type, &mut (shadow.tags), buf, ctx.clone())?;
                Ok(())
            }
            14u32 => {
                <i64 as ::proto_rs::SingularField>::merge_singular_field(wire_type, &mut (shadow.count), buf, ctx.clone())?;
                Ok(())
            }
            15u32 => {
                <f64 as ::proto_rs::SingularField>::merge_singular_field(wire_type, &mut (shadow.ratio), buf, ctx.clone())?;
                Ok(())
            }
            16u32 => {
                <bool as ::proto_rs::SingularField>::merge_singular_field(wire_type, &mut (shadow.active), buf, ctx.clone())?;
                Ok(())
            }
            17u32 => {
                <u64 as ::proto_rs::RepeatedField>::merge_repeated_field(wire_type, &mut (shadow.big_numbers), buf, ctx.clone())?;
                Ok(())
            }
            18u32 => {
                ::proto_rs::encoding::hash_map::merge(
                    |wire_type, key, buf, ctx| <String as ::proto_rs::SingularField>::merge_singular_field(wire_type, key, buf, ctx),
                    |wire_type, value, buf, ctx| <DeepMessage as ::proto_rs::SingularField>::merge_singular_field(wire_type, value, buf, ctx),
                    &mut (shadow.audit_log),
                    buf,
                    ctx.clone(),
                )?;
                Ok(())
            }
            19u32 => {
                if let Some(__proto_rs_existing) = (shadow.primary_focus).as_mut() {
                    <NestedLeaf as ::proto_rs::SingularField>::merge_singular_field(wire_type, __proto_rs_existing.as_mut(), buf, ctx.clone())?;
                } else {
                    let mut __proto_rs_tmp: <::proto_rs::alloc::boxed::Box<NestedLeaf> as ::proto_rs::ProtoExt>::Shadow<'_> =
                        <::proto_rs::alloc::boxed::Box<NestedLeaf> as ::proto_rs::ProtoExt>::proto_default();
                    <::proto_rs::alloc::boxed::Box<NestedLeaf> as ::proto_rs::SingularField>::merge_singular_field(wire_type, &mut __proto_rs_tmp, buf, ctx.clone())?;
                    let __proto_rs_owned = <::proto_rs::alloc::boxed::Box<NestedLeaf> as ::proto_rs::ProtoExt>::post_decode(__proto_rs_tmp)?;
                    (shadow.primary_focus) = Some(__proto_rs_owned);
                }
                Ok(())
            }
            20u32 => {
                if let Some(__proto_rs_existing) = (shadow.secondary_focus).as_mut() {
                    <DeepMessage as ::proto_rs::SingularField>::merge_singular_field(wire_type, __proto_rs_existing.as_mut(), buf, ctx.clone())?;
                } else {
                    let mut __proto_rs_tmp: <::proto_rs::alloc::boxed::Box<DeepMessage> as ::proto_rs::ProtoExt>::Shadow<'_> =
                        <::proto_rs::alloc::boxed::Box<DeepMessage> as ::proto_rs::ProtoExt>::proto_default();
                    <::proto_rs::alloc::boxed::Box<DeepMessage> as ::proto_rs::SingularField>::merge_singular_field(wire_type, &mut __proto_rs_tmp, buf, ctx.clone())?;
                    let __proto_rs_owned = <::proto_rs::alloc::boxed::Box<DeepMessage> as ::proto_rs::ProtoExt>::post_decode(__proto_rs_tmp)?;
                    (shadow.secondary_focus) = Some(__proto_rs_owned);
                }
                Ok(())
            }
            _ => ::proto_rs::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }
    fn clear(&mut self) {
        self.id = <String as ::proto_rs::ProtoExt>::proto_default();
        self.payload = <Bytes as ::proto_rs::ProtoExt>::proto_default();
        self.leaves.clear();
        self.deep_list.clear();
        self.leaf_lookup.clear();
        self.deep_lookup.clear();
        self.status = <ComplexEnum as ::proto_rs::ProtoExt>::proto_default();
        self.status_history.clear();
        self.status_lookup.clear();
        self.codes.clear();
        self.code_lookup.clear();
        self.attachments.clear();
        self.tags.clear();
        self.count = <i64 as ::proto_rs::ProtoExt>::proto_default();
        self.ratio = <f64 as ::proto_rs::ProtoExt>::proto_default();
        self.active = <bool as ::proto_rs::ProtoExt>::proto_default();
        self.big_numbers.clear();
        self.audit_log.clear();
        self.primary_focus = None;
        self.secondary_focus = None;
    }
}
impl ::proto_rs::MessageField for ComplexRoot {}
