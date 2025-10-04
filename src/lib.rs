pub use prosto_derive::inject_proto_import;
pub use prosto_derive::proto_dump;
pub use prosto_derive::proto_message;
pub use prosto_derive::proto_rpc;

pub trait HasProto {
    type Proto: Clone + prost::Message + PartialEq;
    fn to_proto(&self) -> Self::Proto;
    fn from_proto(proto: Self::Proto) -> Result<Self, Box<dyn std::error::Error>>
    where
        Self: Sized;
}
