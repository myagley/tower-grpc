extern crate tower_grpc_build;

fn main() {
    // Build helloworld
    tower_grpc_build::Config::new()
        .enable_server(true)
        .enable_client(true)
        .build(&["proto/helloworld/helloworld.proto"], &["proto/helloworld"])
        .unwrap_or_else(|e| panic!("protobuf compilation failed: {}", e));

    // Build routeguide
    tower_grpc_build::Config::new()
        .enable_server(true)
        .enable_client(true)
        .build(&["proto/routeguide/route_guide.proto"], &["proto/routeguide"])
        .unwrap_or_else(|e| panic!("protobuf compilation failed: {}", e));

    // Build grpc-interop
    tower_grpc_build::Config::new()
        .enable_server(true)
        .enable_client(true)
        .build(&["proto/grpc/testing/test.proto"], &["proto/grpc/testing"])
        .unwrap_or_else(|e| panic!("protobuf compilation failed: {}", e));
}