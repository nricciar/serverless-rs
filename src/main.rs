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
extern crate reqwest;

use actix::prelude::{SyncArbiter};
use actix_web::{http, server, Path, App, AsyncResponder, FutureResponse,
                HttpResponse, HttpRequest};
use http::{StatusCode};
use actix_web::middleware::Logger;
use diesel::pg::PgConnection;
use diesel::r2d2::ConnectionManager;
use futures::Future;
use std::env;
use traits::{ToString};

mod models;
mod db;
mod schema;
mod headers;
mod request;
mod response;
mod traits;
mod functions;

use request::{Request};
use headers::{Header};
use response::{Response};
use db::{GetLambda, CreateLambda, DbExecutor, AppState};

fn create_lambda(body: String, name: Path<request::LambdaPath>, req: HttpRequest<AppState>) -> FutureResponse<HttpResponse> {
    let request = Request::map(&name, &req, None); 
    req.clone()
        .state()
        .db
        .send(CreateLambda {
            path: request.path(),
            hostname: request.host(),
            code: body,
        })
        .from_err()
        .and_then(move |res| match res {
            Ok(lambda) => Ok(HttpResponse::Ok().json(lambda)),
            Err(_) => Ok(HttpResponse::InternalServerError().into()),
        })
        .responder()
}

fn make_response<T: ToString>(status: u16, headers: &Vec<Header>, body: T) -> HttpResponse {
    let mut resp = HttpResponse::build(StatusCode::from_u16(status).unwrap());
    for h in headers.iter() {
        resp.header(h.name().as_str(), h.value().as_str());
    }
    resp.body(body.get_string())
}

fn exec_lambda(body: String, name: Path<request::LambdaPath>, req: HttpRequest<AppState>) -> FutureResponse<HttpResponse> {
    req.clone()
        .state()
        .db
        .send(GetLambda {
            request: Request::map(&name, &req, None),
        })
        .from_err()
        .and_then(move |res| match res {
            Ok(lambda) => {
                let isolate = v8::Isolate::new();
                let context = v8::Context::new(&isolate);

                let request = Request::map(&name, &req, Some(body));

                let js_request = request.js(&isolate, &context);
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
                        let http = v8::value::Object::new(&isolate, &context);
                        let http_request = v8::value::Function::new(&isolate, &context, 1, Box::new(functions::make_request));
                        http.set(&context, &v8::value::String::from_str(&isolate, "request"), &http_request);
                        global.set(&context, &v8::value::String::from_str(&isolate, "http"), &http);

                        // set default response values
                        global.set(&context, &v8::value::String::from_str(&isolate, "response"), &js_response);

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
                                    resp.header(val.name().as_str(), val.value().as_str());
                                }

                                // doc says to run this "frequently" ??
                                isolate.run_enqueued_tasks();

                                Ok(resp.body(result_str.value()))
                            },
                            Err(e) => {
                                println!("ERR! {:?}", e);
                                Ok(HttpResponse::InternalServerError().body("Internal Error"))
                            },
                        }
                    },
                    Err(_) => {
                        Ok(HttpResponse::InternalServerError().body("Internal Error"))
                    }
                }
            },
            Err(_) => {
                Ok(make_response(404, &Vec::new(), "Not Found"))
            }
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