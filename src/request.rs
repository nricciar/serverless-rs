extern crate v8;
extern crate actix;
extern crate actix_web;
extern crate url;

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
                           body: Some("".to_string()) };
        req
    }

    pub fn uri(&self) -> String {
        format!("{}://{}{}", self.proto, self.host, self.path)
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

    pub fn from_js(isolate: &v8::isolate::Isolate, context: &v8::context::Context, obj: &v8::value::Object) -> Result<Request,String> {
        let uri = 
            match obj.get(&context, &v8::value::String::from_str(&isolate, "host")).into_string() {
                Some(s) => {
                    let proto = 
                        match obj.get(&context, &v8::value::String::from_str(&isolate, "proto")).into_string() {
                            Some(s) => s.value(),
                            None => "http".to_string(),
                        };
                    let path = 
                        match obj.get(&context, &v8::value::String::from_str(&isolate, "path")).into_string() {
                            Some(s) => s.value(),
                            None => "/".to_string(),
                        };
                    Some((s.value(), proto, path))
                },
                None => {
                    match obj.get(&context, &v8::value::String::from_str(&isolate, "uri")).into_string() {
                        Some(s) => {
                            let uri = url::Url::parse(s.value().as_str()).unwrap();
                            let path =
                                match uri.query() {
                                    Some(q) => format!("{}?{}", uri.path(), q),
                                    None => uri.path().to_string(),
                                };
                            Some((uri.host().unwrap().to_string(), uri.scheme().to_string(), path))
                        },
                        None => None,
                    }
                }
            };
        match uri {
            Some((host, proto, path)) => {
                let method = 
                    match obj.get(&context, &v8::value::String::from_str(&isolate, "method")).into_string() {
                        Some(s) => s.value(),
                        None => "GET".to_string(),
                    };
                let body = 
                    match obj.get(&context, &v8::value::String::from_str(&isolate, "body")).into_string() {
                        Some(s) => Some(s.value()),
                        None => None,
                    };
                let headers_js = 
                    match obj.get(&context, &v8::value::String::from_str(&isolate, "headers")).into_array() {
                        Some(a) => a,
                        None => v8::value::Array::new(&isolate, &context, 0),
                    };
                let headers = response::get_header_list(&context, &isolate, &headers_js);

                Ok(Request { host: host,
                             proto: proto,
                             path: path,
                             method: method,
                             body: body,
                             headers: headers })
            },
            None => {
                Err("Missing or invalid uri".to_string())
            }
        }
    }
}