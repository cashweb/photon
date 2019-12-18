fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile(&["proto/database.proto", "proto/header.proto", "proto/utility.proto", "proto/transaction.proto", "proto/script_hash.proto"], &["proto"])?;
    Ok(())
}
