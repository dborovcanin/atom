fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/atom/v1/atom.proto")?;
    Ok(())
}
