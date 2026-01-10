use std::io;

fn main() -> io::Result<()> {
    let mut cfg = tonic_prost_build::configure();

    cfg = cfg.build_client(true).build_server(true);

    cfg.compile_protos(
        &[
            "../../protos_ref/sigma_rpc_simple.proto",
            "../../protos_ref/goon_types.proto",
            "../../protos_ref/extra_types.proto",
            "../../protos_ref/rizz_types.proto",
            "../../protos_ref/solana.proto",
            "../../protos_ref/fastnum.proto",
            "../../protos/tests/complex_rpc.proto",
            "../../protos/tests/encoding.proto",
            "../../protos/tests/advanced_features.proto",
        ],
        &["../../protos_ref", "../../protos/tests"],
    )
}
