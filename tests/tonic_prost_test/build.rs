use std::io;

fn main() -> io::Result<()> {
    let mut cfg = tonic_prost_build::configure();

    cfg = cfg.build_client(true).build_server(true);

    cfg.compile_protos(
        &[
            "../../protos/ref_proto/sigma_rpc.proto",
            "../../protos/ref_proto/goon_types.proto",
            "../../protos/ref_proto/rizz_types.proto",
            "../../protos/tests/complex_rpc.proto",
            "../../protos/tests/encoding.proto",
            "../../protos/tests/advanced_features.proto",
        ],
        &["../../protos/ref_proto", "../../protos/tests"],
    )
}
