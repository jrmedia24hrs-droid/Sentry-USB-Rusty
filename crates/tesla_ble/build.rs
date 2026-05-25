//! Compile Tesla's vendored .proto files into Rust types via prost.

fn main() {
    let protos = [
        "proto/common.proto",
        "proto/errors.proto",
        "proto/keys.proto",
        "proto/signatures.proto",
        "proto/universal_message.proto",
        "proto/vcsec.proto",
    ];
    for p in &protos {
        println!("cargo:rerun-if-changed={}", p);
    }
    let protoc = protoc_bin_vendored::protoc_bin_path()
        .expect("locating vendored protoc binary");
    unsafe {
        std::env::set_var("PROTOC", protoc);
    }
    prost_build::Config::new()
        .compile_protos(&protos, &["proto/"])
        .expect("compiling tesla .proto files");
}
