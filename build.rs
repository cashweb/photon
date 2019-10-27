fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure().build_server(false).compile(
        &[
            "proto/fee_estimation.proto",
            "proto/header.proto",
            "proto/script_hash.proto",
            "proto/server.proto",
            "proto/transaction.proto",
        ],
        &["proto"],
    )?;
    Ok(())
}
