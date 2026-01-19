use crate::encoding::WireType;
use crate::traits::ProtoExt;

pub struct VarintConst<const N: usize> {
    pub bytes: [u8; N],
    pub len: usize,
}

pub enum ProtoKind {
    Primitive(PrimitiveKind),
    SimpleEnum,
    Message,
    Bytes,
    String,
    Repeated(&'static ProtoKind),
}

pub enum PrimitiveKind {
    Bool,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    Fixed32,
    Fixed64,
    SFixed32,
    SFixed64,
    SInt32,
    SInt64,
}

impl ProtoKind {
    pub const fn dbg_name(&'static self) -> &'static str {
        match self {
            ProtoKind::Primitive(_) => "Primitive",
            ProtoKind::SimpleEnum => "SimpleEnum",
            ProtoKind::Message => "Message",
            ProtoKind::Bytes => "Bytes",
            ProtoKind::String => "String",
            ProtoKind::Repeated(_) => "Repeated",
        }
    }

    #[inline(always)]
    pub const fn for_vec(inner: &'static ProtoKind) -> ProtoKind {
        ProtoKind::Repeated(inner)
    }
    #[inline(always)]
    pub const fn is_packable(&self) -> bool {
        matches!(self, ProtoKind::Primitive(_) | ProtoKind::SimpleEnum)
    }
    #[inline(always)]
    pub const fn wire_type(&self) -> WireType {
        match self {
            ProtoKind::Primitive(p) => match p {
                PrimitiveKind::Bool
                | PrimitiveKind::I8
                | PrimitiveKind::I16
                | PrimitiveKind::I32
                | PrimitiveKind::I64
                | PrimitiveKind::U8
                | PrimitiveKind::U16
                | PrimitiveKind::U32
                | PrimitiveKind::U64
                | PrimitiveKind::SInt32
                | PrimitiveKind::SInt64 => WireType::Varint,

                PrimitiveKind::Fixed32 | PrimitiveKind::SFixed32 | PrimitiveKind::F32 => WireType::ThirtyTwoBit,

                PrimitiveKind::Fixed64 | PrimitiveKind::SFixed64 | PrimitiveKind::F64 => WireType::SixtyFourBit,
            },
            ProtoKind::SimpleEnum => WireType::Varint,
            ProtoKind::Repeated(_) | ProtoKind::Message | ProtoKind::Bytes | ProtoKind::String => WireType::LengthDelimited,
        }
    }
}

#[track_caller]
#[allow(clippy::extra_unused_type_parameters)]
pub const fn const_unreachable<T: ProtoExt>(structure_name: &'static str) -> ! {
    match T::KIND {
        ProtoKind::Primitive(_) | ProtoKind::SimpleEnum | ProtoKind::Message | ProtoKind::Bytes | ProtoKind::String => {
            const_panic::concat_panic!("SHOULD BE SUPPORTED kind: ", T::KIND.dbg_name(), "in", structure_name)
        }
        ProtoKind::Repeated(proto_kind) => {
            const_panic::concat_panic!("unsupported REPEATED kind: ", proto_kind.dbg_name(), "in", structure_name)
        }
    }
}

#[track_caller]
#[allow(clippy::extra_unused_type_parameters)]
pub const fn const_test_validate_with_ext<T: ProtoExt>() -> ! {
    let name = T::KIND.dbg_name();
    const_panic::concat_panic!(name, ": has validator with ext and it should not be used in infallible rpc methods")
}

pub const fn encode_varint_const<const N: usize>(mut value: u64) -> VarintConst<N> {
    let mut out = [0u8; N];
    let mut i = 0usize;

    loop {
        // SAFETY: i < 10 always holds for valid varint encoding of u64
        let byte = (value & 0x7F) as u8;
        value >>= 7;

        if value == 0 {
            out[i] = byte;
            i += 1;
            break;
        }

        out[i] = byte | 0x80;
        i += 1;

        // Varints are never more than 10 bytes for u64.
        if i == N {
            break;
        }
    }

    VarintConst { bytes: out, len: i }
}
