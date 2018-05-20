// Hello World
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate diesel;
extern crate actix;
extern crate actix_web;
extern crate env_logger;
extern crate futures;
extern crate r2d2;
extern crate v8;

use actix::prelude::{Addr,Syn,SyncArbiter};
use actix_web::{http, server, Path, App, AsyncResponder, FutureResponse,
                HttpResponse, HttpRequest};
use actix_web::middleware::Logger;
use diesel::pg::PgConnection;
use diesel::r2d2::ConnectionManager;
use futures::Future;
use actix_web::HttpMessage;

mod db;
mod models;
mod schema;

use db::{GetLambda, CreateLambda, DbExecutor};

use std::env;

struct AppState {
    db: Addr<Syn, DbExecutor>,
}

#[derive(Deserialize)]
struct LambdaPath {
    path: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct CreateRequest {
    javascript: String,
}

fn hello(info: v8::value::FunctionCallbackInfo) -> Result<v8::value::Value, v8::value::Value> {
    let test = v8::value::String::from_str(&info.isolate, "World!");
    Ok(v8::value::Value::from(test))
}

fn request_to_js_obj(req: &HttpRequest<AppState>, isolate: &v8::isolate::Isolate, context: &v8::context::Context) -> v8::value::Object {
    let hostname = req.headers().get("host").unwrap().to_str().unwrap();
    let path = req.uri().path();
    let request = v8::value::Object::new(&isolate, &context);
    request.set(&context, &v8::value::String::from_str(&isolate, "hostname"),
        &v8::value::String::from_str(&isolate, hostname));
    request.set(&context, &v8::value::String::from_str(&isolate, "path"),
        &v8::value::String::from_str(&isolate, path));
    request
}

fn create_lambda(name: Path<LambdaPath>, req: HttpRequest<AppState>) -> FutureResponse<HttpResponse> {
    req.clone().json()
        .from_err()
        .and_then(move |val: CreateRequest| {
            let hostname = req.headers().get("host").unwrap().to_str().unwrap();
            let path = format!("/{}", name.path);
            req.clone()
                .state()
                .db
                .send(CreateLambda {
                    path: path,
                    hostname: hostname.to_string(),
                    code: val.javascript,
                })
                .from_err()
                .and_then(move |res| match res {
                    Ok(lambda) => Ok(HttpResponse::Ok().json(lambda)),
                    Err(_) => Ok(HttpResponse::InternalServerError().into()),
                })
        })
        .responder()
}

fn exec_lambda(_name: Path<LambdaPath>, req: HttpRequest<AppState>) -> FutureResponse<HttpResponse> {
    let tmp = req.clone();
    let hostname = tmp.headers().get("host").unwrap().to_str().unwrap();
    let path = tmp.uri().path();

    req.clone()
        .state()
        .db
        .send(GetLambda {
            path: path.to_string(),
            hostname: hostname.to_string(),
        })
        .from_err()
        .and_then(move |res| match res {
            Ok(lambda) => {
                let isolate = v8::Isolate::new();
                let context = v8::Context::new(&isolate);

                // Load the source code that we want to evaluate
                let source = v8::value::String::from_str(&isolate, &lambda.code);

                // Compile the source code.  `unwrap()` panics if the code is invalid,
                // e.g. if there is a syntax  error.
                let script = v8::Script::compile(&isolate, &context, &source).unwrap();
                match script.run(&context) {
                    Ok(_) => {
                        let global = context.global();
                        // helper functions
                        let test = v8::value::Function::new(&isolate, &context, 0, Box::new(hello));
                        global.set(&context, &v8::value::String::from_str(&isolate, "hello"), &test);

                        // request object
                        let request = request_to_js_obj(&req, &isolate, &context);

                        // endpoint    
                        let value = global.get(&context, &v8::value::String::from_str(&isolate, "handler"));
                        let fun = value.into_function().unwrap();
                        match fun.call(&context, &[&request]) {
                            Ok(res) => {
                                // Convert the result to a value::String.
                                let result_str = res.to_string(&context);
                                Ok(HttpResponse::Ok().body(result_str.value()))
                            },
                            Err(e) => {
                                println!("ERR! {:?}", e);
                                Ok(HttpResponse::InternalServerError().body("Internal Error"))
                            },
                        }
                    },
                    Err(e) => {
                        println!("ERR! {:?}", e);
                        Ok(HttpResponse::InternalServerError().body("Internal Error"))
                    },
                }
            },
            Err(e) => {
                println!("ERR! {:?}", e);
                Ok(HttpResponse::from_error(e))
            },
        })
        .responder()
}

fn main() {
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();
    let sys = actix::System::new("serverless");

    let database_url =
        match env::var("DATABASE_URL") {
            Ok(v) => v,
            Err(_) => "postgres://postgres:@localhost/serverless".to_string(),
        };
    let listen_addr =
        match env::var("LISTEN_ADDR") {
            Ok(v) => v,
            Err(_) => "127.0.0.1:8088".to_string(),
        };

    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");

    let addr = SyncArbiter::start(3, move || DbExecutor(pool.clone()));

    server::new(move
        || App::with_state(AppState{db: addr.clone()})
            .middleware(Logger::default())
            .resource("/v1/lambda/{path}", |r| r.method(http::Method::POST).with2(create_lambda))
            .resource("/{path}", |r| r.route().with2(exec_lambda)))
        .bind(listen_addr)
        .unwrap()
        .start();

    let _ = sys.run();
}
