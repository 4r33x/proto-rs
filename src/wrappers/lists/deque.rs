enum ArchivedVecDeque<'a, T: ProtoArchive + ProtoExt> {
    Bytes { a: &'a [u8], b: &'a [u8] },
    Owned(ArchivedRepeated<'a, T>),
}

impl<T: ProtoExt> ProtoExt for VecDeque<T> {
    const KIND: ProtoKind = match T::KIND {
        ProtoKind::Primitive(PrimitiveKind::U8) => ProtoKind::Bytes,
        _ => ProtoKind::Repeated(&T::KIND),
    };
    const _REPEATED_SUPPORT: Option<&'static str> = match T::KIND {
        ProtoKind::Primitive(PrimitiveKind::U8) => None,
        _ => Some("VecDeque"),
    };
}

impl<T: ProtoDecoder + ProtoExt> ProtoDecoder for VecDeque<T> {
    #[inline(always)]
    fn proto_default() -> Self {
        VecDeque::new()
    }

    #[inline(always)]
    fn clear(&mut self) {
        VecDeque::clear(self);
    }

    #[inline(always)]
    fn merge_field(value: &mut Self, tag: u32, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if tag == 1 {
            Self::merge(value, wire_type, buf, ctx)
        } else {
            skip_field(wire_type, tag, buf, ctx)
        }
    }

    #[inline(always)]
    fn merge(&mut self, wire_type: WireType, buf: &mut impl Buf, ctx: DecodeContext) -> Result<(), DecodeError> {
        if is_bytes_kind::<T>() {
            // SAFETY: only exercised for VecDeque<u8> which implements BytesAdapterDecode.
            let bytes = unsafe { &mut *(self as *mut VecDeque<T> as *mut VecDeque<u8>) };
            return bytes_encoding::merge(wire_type, bytes, buf, ctx);
        }
        match T::KIND {
            ProtoKind::Primitive(_) | ProtoKind::SimpleEnum => {
                if wire_type == WireType::LengthDelimited {
                    let len = decode_varint(buf)? as usize;
                    let mut slice = buf.take(len);
                    while slice.has_remaining() {
                        let mut v = T::proto_default();
                        T::merge(&mut v, T::WIRE_TYPE, &mut slice, ctx)?;
                        self.push_back(v);
                    }
                    debug_assert!(!slice.has_remaining());
                } else {
                    let mut v = T::proto_default();
                    T::merge(&mut v, wire_type, buf, ctx)?;
                    self.push_back(v);
                }
                Ok(())
            }
            ProtoKind::String | ProtoKind::Bytes | ProtoKind::Message => {
                let mut v = T::proto_default();
                T::merge(&mut v, wire_type, buf, ctx)?;
                self.push_back(v);
                Ok(())
            }
            ProtoKind::Repeated(_) => unreachable!(),
        }
    }
}

impl<T: ProtoDecode> ProtoDecode for VecDeque<T>
where
    T::ShadowDecoded: ProtoDecoder + ProtoExt,
    VecDeque<T::ShadowDecoded>: ProtoDecoder + ProtoExt,
{
    type ShadowDecoded = VecDeque<T::ShadowDecoded>;
}

impl<T, U> ProtoShadowDecode<VecDeque<U>> for VecDeque<T>
where
    T: ProtoShadowDecode<U>,
{
    #[inline]
    fn to_sun(self) -> Result<VecDeque<U>, DecodeError> {
        self.into_iter().map(T::to_sun).collect()
    }
}

impl<'a, T> ProtoArchive for VecDeque<T>
where
    T: ProtoArchive + ProtoExt,
{
    type Archived<'x> = ArchivedVecDeque<'x, T>;

    #[inline(always)]
    fn is_default(&self) -> bool {
        self.is_empty()
    }

    #[inline(always)]
    fn len(archived: &Self::Archived<'_>) -> usize {
        match archived {
            ArchivedVecDeque::Bytes(bytes) => bytes.len(),
            ArchivedVecDeque::Repeated(repeated) => repeated.len,
        }
    }

    #[inline(always)]
    unsafe fn encode(archived: Self::Archived<'_>, buf: &mut impl BufMut) {
        match archived {
            ArchivedVecDeque::Bytes(bytes) => bytes_encoding::encode(&bytes.as_slice(), buf),
            ArchivedVecDeque::Repeated(repeated) => {
                for item in repeated.items {
                    encode_repeated_value::<T>(item, buf);
                }
            }
        }
    }

    #[inline(always)]
    fn archive(&self) -> Self::Archived<'_> {
        if is_bytes_kind::<T>() {
            // SAFETY: only exercised for VecDeque<u8>.
            let bytes = unsafe { &*(self as *const VecDeque<T> as *const VecDeque<u8>) };
            let (left, right) = bytes.as_slices();
            let mut output = Vec::with_capacity(bytes.len());
            output.extend_from_slice(left);
            output.extend_from_slice(right);
            return ArchivedVecDeque::Bytes(output);
        }

        let mut items = Vec::with_capacity(self.len());
        let mut len = 0;
        for item in self {
            let archived = item.archive();
            len += repeated_payload_len::<T>(&archived);
            items.push(archived);
        }
        ArchivedVecDeque::Repeated(ArchivedRepeated { items, len })
    }
}

impl<T: ProtoEncode> ProtoEncode for VecDeque<T>
where
    for<'a> T::Shadow<'a>: ProtoArchive + ProtoExt,
    for<'a> VecDeque<T::Shadow<'a>>: ProtoArchive + ProtoExt,
{
    type Shadow<'a> = VecDeque<T::Shadow<'a>>;
}

impl<'a, T, S> ProtoShadowEncode<'a, VecDeque<T>> for VecDeque<S>
where
    S: ProtoShadowEncode<'a, T>,
    T: ProtoEncode,
{
    #[inline]
    fn from_sun(value: &'a VecDeque<T>) -> Self {
        value.iter().map(S::from_sun).collect()
    }
}
