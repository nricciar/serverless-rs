extern crate v8;
extern crate reqwest;

use std::str;

use response;
use request;

pub fn hello(info: v8::value::FunctionCallbackInfo) -> Result<v8::value::Value, v8::value::Value> {
    let test = v8::value::String::from_str(&info.isolate, "World!");
    Ok(v8::value::Value::from(test))
}

pub fn parse_json(info: v8::value::FunctionCallbackInfo) -> Result<v8::value::Value, v8::value::Value> {
    let context = v8::Context::new(&info.isolate);
    let request_obj = info.this.clone();
    match request_obj.get(&context, &v8::value::String::from_str(&info.isolate, "body")).into_string() {
        Some(r) => {
            let global = context.global();
            let json = global.get(&context, &v8::value::String::from_str(&info.isolate, "JSON")).into_object().unwrap();
            let parse = json.get(&context, &v8::value::String::from_str(&info.isolate, "parse")).into_function().unwrap();
            let req_json = v8::value::String::from_str(&info.isolate, r.value().as_str());
            match parse.call(&context, &[&req_json]) {
                Ok(resp) => Ok(v8::value::Value::from(resp.into_object().unwrap())),
                _ => {
                    let err = v8::value::String::from_str(&info.isolate, "Invalid json");
                    Err(v8::value::Value::from(err))
                }
            }
        },
        _ => {
            let err = v8::value::String::from_str(&info.isolate, "Invalid json");
            Err(v8::value::Value::from(err))
        }
    }
}

fn construct_headers(headers: &Vec<response::Header>) -> reqwest::header::Headers {
    let mut resp = reqwest::header::Headers::new();
    for h in headers.iter() {
        resp.set_raw(h.key.as_str().to_string(), h.value.as_str().to_string());
    }
    resp
}

pub fn make_request(info: v8::value::FunctionCallbackInfo) -> Result<v8::value::Value, v8::value::Value> {
    match info.args.as_slice() {
        [key] => {
            match key.clone().into_object() {
                Some(k) => {
                    let context = v8::Context::new(&info.isolate);
                    let request = request::Request::from_js(&info.isolate, &context, &k).unwrap();

                    let client = reqwest::Client::new();
                    let uri = request.uri();
                    let req =
                        match (request.method.as_str(), request.body) {
                            ("GET", _) => Some(client.get(uri.as_str()).headers(construct_headers(&request.headers)).send()), 
                            ("POST", Some(b)) => Some(client.post(uri.as_str()).body(b).headers(construct_headers(&request.headers)).send()),
                            ("DELETE", None) => Some(client.delete(uri.as_str()).headers(construct_headers(&request.headers)).send()),
                            ("PATCH", Some(b)) => Some(client.patch(uri.as_str()).body(b).headers(construct_headers(&request.headers)).send()),
                            ("PUT", Some(b)) => Some(client.put(uri.as_str()).body(b).headers(construct_headers(&request.headers)).send()),
                            _ => None
                        };
                    match req {
                        Some(r) => {
                            // response
                            let mut ret = r.unwrap();

                            // response body
                            let mut buf: Vec<u8> = vec![];
                            ret.copy_to(&mut buf).unwrap();
                            let body = str::from_utf8(&buf).unwrap();

                            // response
                            let status = ret.status().as_u16();
                            let response = response::Response { status: status as i32, headers: Vec::new(), body: body.to_string() };
                            let response_js = response.js(&info.isolate, &context);

                            // add .json() helper to response object
                            let parse_json = v8::value::Function::new(&info.isolate, &context, 0, Box::new(parse_json));
                            response_js.set(&context, &v8::value::String::from_str(&info.isolate, "json"), &parse_json);

                            Ok(v8::value::Value::from(response_js))
                        },
                        None => {
                            let err = v8::value::String::from_str(&info.isolate, "Invalid Request Method!");
                            Err(v8::value::Value::from(err)) 
                        }
                    }
                },
                None => {
                    let err = v8::value::String::from_str(&info.isolate, "Invalid Request!");
                    Err(v8::value::Value::from(err)) 
                }
            }
        },
        _ => {
            let err = v8::value::String::from_str(&info.isolate, "Invalid Request!");
            Err(v8::value::Value::from(err)) 
        }
    }
}

pub fn get_header(info: v8::value::FunctionCallbackInfo) -> Result<v8::value::Value, v8::value::Value> {
    match info.args.as_slice() {
        [key] => {
            match key.clone().into_string() {
                Some(k) => {
                    let context = v8::Context::new(&info.isolate);
                    let request_obj = info.this.clone();
                    match request_obj.get(&context, &v8::value::String::from_str(&info.isolate, "headers")).into_array() {
                        Some(list) => {
                            match response::find_header_by_key(&context, &info.isolate, &list, k.value()) {
                                Some(ret) => {
                                    let msg = v8::value::String::from_str(&info.isolate, ret.value.as_str());
                                    Ok(v8::value::Value::from(msg))
                                },
                                _ => {
                                    let msg = v8::value::String::from_str(&info.isolate, "");
                                    Ok(v8::value::Value::from(msg))
                                }
                            }
                        },
                        _ => {
                            let msg = v8::value::String::from_str(&info.isolate, "");
                            Ok(v8::value::Value::from(msg))
                        }
                    }
                },
                _ => {
                    let err = v8::value::String::from_str(&info.isolate, "Invalid Request!");
                    Err(v8::value::Value::from(err))                     
                }
            }
        },
        _ => {
            let err = v8::value::String::from_str(&info.isolate, "Invalid Request!");
            Err(v8::value::Value::from(err)) 
        }
    }
}

pub fn add_header(info: v8::value::FunctionCallbackInfo) -> Result<v8::value::Value, v8::value::Value> {
    match info.args.as_slice() {
        [key, value] => {
            match (key.clone().into_string(), value.clone().into_string()) {
                (Some(k), Some(v)) => {
                    let context = v8::Context::new(&info.isolate);
                    let response_obj = info.this.clone();
                    let list = response_obj.get(&context, &v8::value::String::from_str(&info.isolate, "headers")).into_array().unwrap();

                    response::append_header(&context, &info.isolate, &list, &response::Header{ key: k.value(), value: v.value() });
                    let msg = v8::value::Boolean::new(&info.isolate, true);
                    Ok(v8::value::Value::from(msg))
                },
                _ => {
                    let err = v8::value::String::from_str(&info.isolate, "Invalid Request!");
                    Err(v8::value::Value::from(err))                     
                }
            }
        },
        _ => { 
            let err = v8::value::String::from_str(&info.isolate, "Invalid Request!");
            Err(v8::value::Value::from(err)) 
        }
    }
}