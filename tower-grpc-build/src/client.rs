use codegen;
use prost_build;
use std::fmt;

/// Generates service code
pub struct ServiceGenerator;

// ===== impl ServiceGenerator =====

impl ServiceGenerator {
    pub fn generate(&self,
                    service: &prost_build::Service,
                    mut scope: codegen::Scope) 
                    -> codegen::Scope {
        self.define(service, &mut scope);
        scope
    }

    fn define(&self, 
              service: &prost_build::Service,
              scope: &mut codegen::Scope) {
        // Create scope that contains the generated client code.
        let scope = scope.new_module("client")
            .vis("pub")
            .import("::tower_grpc::codegen::client", "*")
            .scope()
            ;

        self.import_message_types(service, scope);
        self.define_client_struct(service, scope);
        self.define_client_impl(service, scope);
    }

    fn import_message_types(&self, service: &prost_build::Service, scope: &mut codegen::Scope) {
        for method in &service.methods {
            let (input_path, input_type) = ::super_import(&method.input_type, 1);
            let (output_path, output_type) = ::super_import(&method.output_type, 1);

            scope.import(&input_path, input_type);
            scope.import(&output_path, output_type);
        }
    }

    fn define_client_struct(&self, service: &prost_build::Service, scope: &mut codegen::Scope) {
        scope.new_struct(&service.name)
            .vis("pub")
            .generic("T")
            .derive("Debug")
            .field("inner", "grpc::Grpc<T>")
            ;
    }

    fn define_client_impl(&self, service: &prost_build::Service, scope: &mut codegen::Scope) {
        let imp = scope.new_impl(&service.name)
            .generic("T")
            .target_generic("T")
            .bound("T", "tower_h2::HttpService")
            ;

        imp.new_fn("new")
            .vis("pub")
            .arg("inner", "T")
            .arg("uri", "http::Uri")
            .ret("Result<Self, grpc::BuilderError>")
            .line("let inner = grpc::Builder::new()")
            .line("    .uri(uri)")
            .line("    .build(inner)?;")
            .line("")
            .line("Ok(Self { inner })")
            ;

        imp.new_fn("poll_ready")
            .vis("pub")
            .arg_mut_self()
            .ret("futures::Poll<(), grpc::Error<T::Error>>")
            .line("self.inner.poll_ready()")
            ;

        for method in &service.methods {
            let name = ::lower_name(&method.proto_name);
            let path = ::method_path(service, method);
            let input_type = ::unqualified(&method.input_type);
            let output_type = ::unqualified(&method.output_type);

            let func = imp.new_fn(&name)
                .vis("pub")
                .arg_mut_self()
                .line(format!("let path = http::PathAndQuery::from_static({});", path))
                ;

            let mut request = codegen::Type::new("grpc::Request");

            let req_body = match (method.client_streaming, method.server_streaming) {
                (false, false) => {
                    let ret = format!(
                        "grpc::unary::ResponseFuture<{}, T::Future, T::ResponseBody>",
                        output_type);

                    request.generic(input_type);

                    func.ret(ret)
                        .line("self.inner.unary(request, path)")
                        ;

                    format!("grpc::unary::Once<{}>", input_type)
                }
                (false, true) => {
                    let ret = format!(
                        "grpc::server_streaming::ResponseFuture<{}, T::Future>",
                        output_type);

                    request.generic(input_type);

                    func.generic("B")
                        .ret(ret)
                        .line("self.inner.server_streaming(request, path)")
                        ;

                    format!("grpc::unary::Once<{}>", input_type)
                }
                (true, false) => {
                    let ret = format!(
                        "grpc::client_streaming::ResponseFuture<{}, T::Future, T::ResponseBody>",
                        output_type);

                    request.generic("B");

                    func.generic("B")
                        .ret(ret)
                        .line("self.inner.client_streaming(request, path)")
                        ;

                    "B".to_string()
                }
                (true, true) => {
                    let ret = format!(
                        "grpc::streaming::ResponseFuture<{}, T::Future>",
                        output_type);

                    request.generic("B");

                    func.generic("B")
                        .ret(ret)
                        .line("self.inner.streaming(request, path)")
                        ;

                    "B".to_string()
                }
            };

            func.arg("request", request)
                .bound(&req_body, "grpc::Encodable<T::RequestBody>");
        }
    }
}
