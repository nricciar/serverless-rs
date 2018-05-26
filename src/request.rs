extern crate v8;
extern crate actix;
extern crate actix_web;

use actix_web::{Path,HttpRequest,HttpMessage};
use db::{AppState};
use response;

#[derive(Debug)]
pub struct Request {
    pub host: String,
    pub proto: String,
    pub path: String,
    pub method: String,
    pub headers: Vec<response::Header>,
    pub body: Option<String>,
}

#[derive(Deserialize)]
pub struct LambdaPath {
    pub path: String,
}

impl Request {
    pub fn map(name: &Path<LambdaPath>, req: &HttpRequest<AppState>) -> Request {
        let mut headers = Vec::new();
        for (key, value) in req.headers().iter() {
            headers.push(response::Header{ key: key.as_str().to_string(), value: value.to_str().unwrap().to_string() })
        }

        let req = Request{ host: req.headers().get("host").unwrap().to_str().unwrap().to_string(),
                           path: format!("/{}", name.path.clone()),
                           proto: "http".to_string(),
                           method: req.method().as_str().to_string(),
                           headers: headers,
                           // FIXME: body
                           body: Some("".to_string()) };
        req
    }

    pub fn js(&self, isolate: &v8::isolate::Isolate, context: &v8::context::Context) -> v8::value::Object {
        let request = v8::value::Object::new(&isolate, &context);
        request.set(&context, &v8::value::String::from_str(&isolate, "host"),
            &v8::value::String::from_str(&isolate, &self.host));
        request.set(&context, &v8::value::String::from_str(&isolate, "proto"),
            &v8::value::String::from_str(&isolate, &self.proto));
        request.set(&context, &v8::value::String::from_str(&isolate, "path"),
            &v8::value::String::from_str(&isolate, &self.path));
        request.set(&context, &v8::value::String::from_str(&isolate, "method"),
            &v8::value::String::from_str(&isolate, &self.method));

        let headers = v8::value::Array::new(&isolate, &context, 0);
        for h in self.headers.iter() {
            response::append_header(&context, &isolate, &headers, &h);
        }

        request.set(&context, &v8::value::String::from_str(&isolate, "headers"),
            &headers);
        request.set(&context, &v8::value::String::from_str(&isolate, "body"),
            &v8::value::String::from_str(&isolate, &self.body.clone().unwrap()));
        request
    }

}