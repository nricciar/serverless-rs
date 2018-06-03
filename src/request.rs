extern crate url;
extern crate serde;
extern crate serde_json;
extern crate v8;
extern crate reqwest;

use headers::{Header};
use traits::{CanParse, ToString};
use db::{AppState};
use actix_web::{Path,HttpRequest,HttpMessage};
use functions;

#[derive(Deserialize)]
pub struct LambdaPath {
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Method {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    OPTIONS
}

impl Method {
    pub fn to_string(&self) -> String {
        match self {
            Method::GET => "GET",
            Method::POST => "POST",
            Method::DELETE => "DELETE",
            Method::PUT => "PUT",
            Method::PATCH => "PATCH",
            Method::OPTIONS => "OPTIONS",
        }
        .to_string()
    }

    pub fn to_reqwest(&self) -> reqwest::Method {
        match self {
            Method::GET => reqwest::Method::Get,
            Method::POST => reqwest::Method::Post,
            Method::DELETE => reqwest::Method::Delete,
            Method::PUT => reqwest::Method::Put,
            Method::PATCH => reqwest::Method::Patch,
            Method::OPTIONS => reqwest::Method::Options,
        }
    }

    pub fn js(&self, isolate: &v8::isolate::Isolate) -> v8::value::String {
        v8::value::String::from_str(&isolate, self.to_string().as_str())
    }

    pub fn from_js(js: &v8::value::Value) -> Result<Method,String> {
        match js.clone().into_string() {
            Some(s) => {
                match s.value().as_str() {
                    "GET" | "POST" | "PUT" | "DELETE" | "PATCH" | "OPTIONS" => Ok(Method::from_str(s.value()).unwrap()),
                    _ => Err("Invalid Method".to_string())
                }
            },
            None => Err("Invalid Method".to_string()),
        }
    }

    fn from_str<T: ToString>(method: T) -> Result<Method,String> {
        match method.get_string().as_str() {
            "GET" => Ok(Method::GET),
            "POST" => Ok(Method::POST),
            "PUT" => Ok(Method::PUT),
            "DELETE" => Ok(Method::DELETE),
            "PATCH" => Ok(Method::PATCH),
            "OPTIONS" => Ok(Method::OPTIONS),
            _ => Err("Invalid Method".to_string())
        }
    }
}

#[derive(Debug)]
pub struct Request {
    uri: url::Url,
    method: Method,
    headers: Vec<Header>,
    body: Option<String>,
}

#[derive(Debug)]
pub struct RequestBuilder {
    req: Option<Request>,
    err: Option<String>,
}

impl Request {
    pub fn new<T: CanParse>(uri: T) -> RequestBuilder {
        RequestBuilder::new(uri)
    }

    pub fn map(name: &Path<LambdaPath>, req: &HttpRequest<AppState>, body: Option<String>) -> Request {
        // headers
        let mut headers = Vec::new();
        for (key, value) in req.headers().iter() {
            headers.push(Header::new(key.as_str().to_string(), value.to_str().unwrap().to_string()));
        }

        // method
        let method = Method::from_str(req.method().as_str()).unwrap();

        // uri
        let host = req.headers().get("host").unwrap().to_str().unwrap().to_string();
        let path = format!("/{}", name.path.clone());
        let proto = "http".to_string();
        let uri = url::Url::parse(format!("{}://{}{}", proto, host, path).as_str()).unwrap();

        Request::new(uri)
            .method(method)
            .headers(headers)
            .body(body)
            .build()
    }

    pub fn path(&self) -> String {
        self.uri.path().to_string()
    }

    pub fn host(&self) -> String {
        self.uri.host_str().unwrap().to_string()
    }

    pub fn headers(&self) -> &Vec<Header> {
        &self.headers
    }

    fn construct_headers(&self) -> reqwest::header::Headers {
        let mut ret = reqwest::header::Headers::new();
        for h in self.headers.iter() {
            ret.set_raw(h.name(), h.value());
        }
        ret
    }

    pub fn to_reqwest(&self, client: &reqwest::Client) -> Result<reqwest::Request,reqwest::Error> {
        client.request(self.method.to_reqwest(), self.uri.as_str())
                      .headers(self.construct_headers())
                      .build()
    }

    pub fn js(&self, isolate: &v8::isolate::Isolate, context: &v8::context::Context) -> v8::value::Object {
        let ret = v8::value::Object::new(&isolate, &context);

        // uri
        ret.set(&context, &v8::value::String::from_str(&isolate, "uri"), 
            &v8::value::String::from_str(&isolate, self.uri.as_str()));

        // method
        ret.set(&context, &v8::value::String::from_str(&isolate, "method"),
            &self.method.js(&isolate));

        // headers
        let headers = v8::value::Array::new(&isolate, &context, 0);
        let mut count = 0;
        for h in self.headers.iter() {
            headers.set(&context, &v8::value::Integer::new(&isolate, count), 
                &h.js(&isolate, &context));
            count += 1;
        }
        ret.set(&context, &v8::value::String::from_str(&isolate, "headers"), &headers);

        // body
        let body =
            match self.body {
                Some(ref b) => {
                    let v8str = v8::value::String::from_str(&isolate, b.as_str());
                    v8::value::Value::from(v8str)
                },
                None => v8::value::Value::from(v8::value::null(&isolate)),
            };
        ret.set(&context, &v8::value::String::from_str(&isolate, "body"), &body);

        // functions
        let json = v8::value::Function::new(&isolate, &context, 0, Box::new(functions::parse_json));
        ret.set(&context, &v8::value::String::from_str(&isolate, "json"), &json);
        let get_header = v8::value::Function::new(&isolate, &context, 1, Box::new(functions::get_header));
        ret.set(&context, &v8::value::String::from_str(&isolate, "getHeader"), &get_header);

        ret
    }

    pub fn from_js(isolate: &v8::isolate::Isolate, context: &v8::context::Context, js: &v8::value::Value) -> Result<Request,String> {
        match js.clone().into_object() {
            Some(o) => {
                match o.get(&context, &v8::value::String::from_str(&isolate, "uri")).into_string() {
                    Some(uri) => {
                        // method
                        let method = 
                            match o.get(&context, &v8::value::String::from_str(&isolate, "method")).into_string() {
                                Some(m) => Method::from_js(&m).unwrap(),
                                None => Method::GET,
                            };

                        // body
                        let body =
                            match o.get(&context, &v8::value::String::from_str(&isolate, "body")).into_string() {
                                Some(b) => Some(b.value()),
                                None => None,
                            };

                        let headers =
                            match o.get(&context, &v8::value::String::from_str(&isolate, "headers")).into_array() {
                                Some(arr) => {
                                    let mut headers = Vec::new();
                                    let mut count = 0;
                                    while {
                                        let item = arr.get(&context, &v8::value::Integer::new(&isolate, count));
                                        if item.is_array() { 
                                            count += 1;
                                            let head = Header::from_js(&isolate, &context, &item).unwrap();
                                            headers.push(head);
                                            true
                                        } else {
                                            false
                                        }
                                    } {}
                                    headers
                                },
                                None => Vec::new(),
                            };

                        Ok(Request{ uri: url::Url::parse(uri.value().as_str()).unwrap(),
                                    method: method,
                                    headers: headers,
                                    body: body })
                    },
                    None => Err("Invalid Request: Missing URI".to_string()),
                }
            },
            None => Err("Invalid Request".to_string()),
        }
    }

    #[inline]
    fn method_mut(&mut self) -> &mut Method {
        &mut self.method
    }

    #[inline]
    fn headers_mut(&mut self) -> &mut Vec<Header> {
        &mut self.headers
    }

    #[inline]
    fn body_mut(&mut self) -> &mut Option<String> {
        &mut self.body
    }
}

impl RequestBuilder {
    pub fn new<T: CanParse>(uri: T) -> RequestBuilder {
        let (req, err) =
            match uri.parse() {
                Ok(u) => (Some(Request{ uri: u,
                                        method: Method::GET,
                                        headers: Vec::new(),
                                        body: None }), None),
                Err(_) => (None, Some("Invalid URL".to_string()))
            };
        RequestBuilder{ req: req, 
                        err: err }
    }

    #[inline]
    fn req_mut(&mut self) -> &mut Request {
        self.req.as_mut().expect("testing")
    }

    #[inline]
    fn err_mut(&mut self) -> &mut Option<String> {
        &mut self.err
    }

    pub fn build(&mut self) -> Request {
        self.req.take().expect("testing")
    }

    pub fn method<T: ToString>(&mut self, method: T) -> &mut RequestBuilder {
        match Method::from_str(method.get_string()) {
            Ok(m) => {
                *self.req_mut().method_mut() = m;
                self
            },
            Err(e) => {
                *self.err_mut() = Some(e);
                self
            },
        }
    }

    pub fn headers(&mut self, headers: Vec<Header>) -> &mut RequestBuilder {
        *self.req_mut().headers_mut() = headers;
        self
    }

    pub fn body<T: ToString>(&mut self, body: Option<T>) -> &mut RequestBuilder {
        let body =
            match body {
                Some(b) => Some(b.get_string()),
                None => None,
            };
        *self.req_mut().body_mut() = body;
        self
    }
}