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

use actix::prelude::{SyncArbiter};
use actix_web::{http, server, Path, App, AsyncResponder, FutureResponse,
                HttpResponse, HttpRequest};
use http::{StatusCode};
use actix_web::middleware::Logger;
use diesel::pg::PgConnection;
use diesel::r2d2::ConnectionManager;
use futures::Future;

mod db;
mod models;
mod schema;
mod functions;
mod request;
mod response;

use db::{GetLambda, CreateLambda, DbExecutor, AppState};
use request::{Request};
use response::{Response};

use std::env;

#[derive(Debug, Deserialize, Serialize)]
struct CreateRequest {
    javascript: String,
}

fn create_lambda(body: String, name: Path<request::LambdaPath>, req: HttpRequest<AppState>) -> FutureResponse<HttpResponse> {
    let request = Request::map(&name, &req); 
    req.clone()
        .state()
        .db
        .send(CreateLambda {
            path: request.path,
            hostname: request.host,
            code: body,
        })
        .from_err()
        .and_then(move |res| match res {
            Ok(lambda) => Ok(HttpResponse::Ok().json(lambda)),
            Err(_) => Ok(HttpResponse::InternalServerError().into()),
        })
        .responder()
}

fn exec_lambda(body: String, name: Path<request::LambdaPath>, req: HttpRequest<AppState>) -> FutureResponse<HttpResponse> {
    req.clone()
        .state()
        .db
        .send(GetLambda {
            request: Request::map(&name, &req),
        })
        .from_err()
        .and_then(move |res| match res {
            Ok(lambda) => {
                let isolate = v8::Isolate::new();
                let context = v8::Context::new(&isolate);

                // request object passed to handler
                let js_request = Request::map(&name, &req).js(&isolate, &context);
                // initial response object
                let js_response = Response::new().js(&isolate, &context);

                // Load the source code that we want to evaluate
                let source = v8::value::String::from_str(&isolate, &lambda.code);

                // Compile the source code.  `unwrap()` panics if the code is invalid,
                // e.g. if there is a syntax  error.
                let script = v8::Script::compile(&isolate, &context, &source).unwrap();
                match script.run(&context) {
                    Ok(_) => {
                        let global = context.global();
                        // helper functions
                        let test = v8::value::Function::new(&isolate, &context, 0, Box::new(functions::hello));
                        global.set(&context, &v8::value::String::from_str(&isolate, "hello"), &test);
                        let http = v8::value::Object::new(&isolate, &context);
                        let http_request = v8::value::Function::new(&isolate, &context, 1, Box::new(functions::make_request));
                        http.set(&context, &v8::value::String::from_str(&isolate, "request"), &http_request);
                        global.set(&context, &v8::value::String::from_str(&isolate, "http"), &http);

                        // response helper functions
                        let add_header = v8::value::Function::new(&isolate, &context, 2, Box::new(functions::add_header));
                        js_response.set(&context, &v8::value::String::from_str(&isolate, "addHeader"), &add_header);
                        // set default response values
                        global.set(&context, &v8::value::String::from_str(&isolate, "response"), &js_response);

                        // request helper functions
                        let get_header = v8::value::Function::new(&isolate, &context, 1, Box::new(functions::get_header));
                        js_request.set(&context, &v8::value::String::from_str(&isolate, "getHeader"), &get_header);
                        let parse_json = v8::value::Function::new(&isolate, &context, 0, Box::new(functions::parse_json));
                        js_request.set(&context, &v8::value::String::from_str(&isolate, "json"), &parse_json);

                        // FIXME: should be a part of Request::map
                        js_request.set(&context, &v8::value::String::from_str(&isolate, "body"), 
                            &v8::value::String::from_str(&isolate, body.as_str()));

                        // endpoint    
                        let value = global.get(&context, &v8::value::String::from_str(&isolate, "handler"));
                        let fun = value.into_function().unwrap();
                        match fun.call(&context, &[&js_request]) {
                            Ok(res) => {
                                // Convert the result to a value::String.
                                let result_str = res.to_string(&context);
                                let response_val = global.get(&context, &v8::value::String::from_str(&isolate, "response"));
                                let response_obj = Response::from_js(&isolate, &context, &response_val.into_object().unwrap());

                                let mut resp = HttpResponse::build(StatusCode::from_u16(response_obj.status as u16).unwrap());
                                for val in response_obj.headers {
                                    resp.header(val.key.as_str(), val.value.as_str());
                                }

                                Ok(resp.body(result_str.value()))
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
            .resource("/v1/lambda/{path}", |r| r.method(http::Method::POST).with3(create_lambda))
            .resource("/{path}", |r| r.route().with3(exec_lambda)))
        .bind(listen_addr)
        .unwrap()
        .start();

    let _ = sys.run();
}
