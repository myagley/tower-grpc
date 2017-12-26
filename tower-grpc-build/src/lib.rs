extern crate codegen;
extern crate prost_build;

mod client;
mod server;

use std::io;
use std::cell::RefCell;
use std::fmt::Write;
use std::path::Path;
use std::rc::Rc;
use std::ascii::AsciiExt;

/// Code generation configuration
pub struct Config {
    prost: prost_build::Config,
    inner: Rc<RefCell<Inner>>,
}

struct Inner {
    build_client: bool,
    build_server: bool,
}

struct GeneratorState {
    root: codegen::Scope,
    // XXX this is where it gets _really_ ugly.
    last_gen_length: usize,
}

struct ServiceGenerator {
    client: client::ServiceGenerator,
    server: server::ServiceGenerator,
    inner: Rc<RefCell<Inner>>,
    state: RefCell<GeneratorState>,
}

impl Config {
    /// Returns a new `Config` with pre-configured prost.
    ///
    /// You can tweak the configuration how the proto buffers are generated and use this config.
    pub fn from_prost(mut prost: prost_build::Config) -> Self {
        let inner = Rc::new(RefCell::new(Inner {
            // Enable client code gen by default
            build_client: true,

            // Disable server code gen by default
            build_server: false,
        }));

        let state = RefCell::new(GeneratorState {
            root: codegen::Scope::new(),
            // so far we have generated 0 characters of code...
            last_gen_length: 0,
        });

        // Set the service generator
        prost.service_generator(Box::new(ServiceGenerator {
            client: client::ServiceGenerator,
            server: server::ServiceGenerator,
            inner: inner.clone(),
            state,
        }));

        Config {
            prost,
            inner,
        }
    }

    /// Returns a new `Config` with default values.
    pub fn new() -> Self {
        Self::from_prost(prost_build::Config::new())
    }

    /// Enable gRPC client code generation
    pub fn enable_client(&mut self, enable: bool) -> &mut Self {
        self.inner.borrow_mut().build_client = enable;
        self
    }

    /// Enable gRPC server code generation
    pub fn enable_server(&mut self, enable: bool) -> &mut Self {
        self.inner.borrow_mut().build_server = enable;
        self
    }

    /// Generate code
    pub fn build<P>(&self, protos: &[P], includes: &[P]) -> io::Result<()>
    where P: AsRef<Path>,
    {
        self.prost.compile_protos(protos, includes)
    }
}

impl prost_build::ServiceGenerator for ServiceGenerator {
    fn generate(&self, service: prost_build::Service, buf: &mut String) {
        let inner = self.inner.borrow();
        let mut state = self.state.borrow_mut();

        // if a service was already generated, remove its code from the 
        // `prost_build` buffer, because it's already in the `codegen::Scope`, 
        // and we will add to existing modules in the generated code. we can't
        // just wipe out the whole buffer, because the code generated by 
        // `prost` is in there and we need that.
        // XXX this is a terrible hack.
        if state.last_gen_length > 0 {
            let new_len = buf.len() - state.last_gen_length;
            buf.truncate(new_len);
        }

        // Add an extra new line to separate messages
        write!(buf, "\n").unwrap();

        if inner.build_client {
            self.client.generate(&service, &mut state.root);
        }

        if inner.build_server {
            self.server.generate(&service, &mut state.root);
        }

        // XXX terrible hack continues. generate the code for this service into 
        // a _new_ String and then append it to the buffer provided to us by 
        // `prost_build`, so we can record the length of the code we generate
        // so that the next service generated --- if there is one --- can 
        // clobber it to prevent duplication.
        let mut code = String::new();
        {
            let mut fmt = codegen::Formatter::new(&mut code);
            state.root.fmt(&mut fmt).unwrap();
        }
        state.last_gen_length = code.len();
        buf.push_str(&code[..]);

    }
}

// ===== utility fns =====

fn method_path(service: &prost_build::Service, method: &prost_build::Method) -> String {
    format!("\"/{}.{}/{}\"",
            service.package,
            service.proto_name,
            method.proto_name)
}

fn lower_name(name: &str) -> String {
    let mut ret = String::new();

    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() {
            if i != 0 {
                ret.push('_');
            }

            ret.push(ch.to_ascii_lowercase());
        } else {
            ret.push(ch);
        }
    }

    ret
}

fn super_import(ty: &str, level: usize) -> (String, &str) {
    let mut v: Vec<&str> = ty.split("::").collect();

    for _ in 0..level {
        v.insert(0, "super");
    }

    let last = v.pop().unwrap_or(ty);

    (v.join("::"), last)
}

fn unqualified(ty: &str) -> &str {
    ty.rsplit("::").next().unwrap_or(ty)
}
