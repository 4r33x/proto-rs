mod signed;
mod unsigned;

#[allow(dead_code)]
pub trait DecimalProtoExt: Sized {
    type Proto;

    fn to_proto(&self) -> Self::Proto;

    fn from_proto(proto: Self::Proto) -> Result<Self, crate::DecodeError>;
}
