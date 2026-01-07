#![cfg_attr(not(feature = "stable"), feature(impl_trait_in_assoc_type))]
#![allow(clippy::missing_errors_doc)]

use proto_rs::proto_message;
use proto_rs::proto_rpc;

#[proto_message(proto_path = "protos/gen_proto/types.proto")]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct UserType;

#[proto_rpc(
    rpc_package = "import_all_from_rpc",
    rpc_server = true,
    rpc_client = false,
    proto_path = "protos/gen_proto/import_all_from_rpc.proto",
    proto_import_all_from(types)
)]
pub trait ImportAllFromRpc {
    async fn get_user(&self, request: tonic::Request<UserType>) -> Result<UserType, tonic::Status>;
}

#[cfg(test)]
mod tests {
    #[test]
    fn proto_import_all_from_qualifies_user_types() {
        let proto = include_str!("../protos/gen_proto/import_all_from_rpc.proto");

        assert!(proto.contains("import \"types.proto\";"));
        assert!(proto.contains("rpc GetUser(types.UserType) returns (types.UserType) {}"));
    }
}
