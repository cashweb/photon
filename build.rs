fn main() -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Repeating like this is a workaround due to a known bug in prost build
    // https://github.com/hyperium/tonic/issues/38
    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile(&["proto/database.proto"], &["proto"])?;
    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile(&["proto/header.proto"], &["proto"])?;
    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile(&["proto/utility.proto"], &["proto"])?;
    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile(&["proto/transaction.proto"], &["proto"])?;

    Ok(())
}
