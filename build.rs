fn main() -> Result<(), Box<dyn std::error::Error>> {
    prost_build::compile_protos(&["src/tss/tss.proto"], &["src/"]).unwrap();
    Ok(())
}
