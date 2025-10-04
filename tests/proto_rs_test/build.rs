use std::io;

fn main() -> io::Result<()> {
    let mut cfg = tonic_prost_build::configure();

    cfg = cfg.build_client(true).build_server(true);

    cfg.compile_protos(
        &[
            "../../protos/gen_proto/sigma_rpc.proto",
            "../../protos/gen_proto/goon_types.proto",
            "../../protos/gen_proto/rizz_types.proto",
        ],
        &["../../protos/gen_proto"],
    )
}
