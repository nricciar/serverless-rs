extern crate v8;
extern crate reqwest;
extern crate serde;
extern crate serde_json;

use std::str;

use response::{Response};
use request::{Request};
use headers::{Header};

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

pub fn make_request(info: v8::value::FunctionCallbackInfo) -> Result<v8::value::Value, v8::value::Value> {
    match info.args.as_slice() {
        [key] => {
            match key.clone().into_object() {
                Some(k) => {
                    let context = v8::Context::new(&info.isolate);
                    let request = Request::from_js(&info.isolate, &context, &k).unwrap();

                    let client = reqwest::Client::new();
                    match request.to_reqwest(&client) {
                        Ok(req) => {
                            let mut resp = client.execute(req);

                            match resp.as_mut() {
                                Ok(r) => {
                                    // response body
                                    let mut buf: Vec<u8> = vec![];
                                    r.copy_to(&mut buf).unwrap();
                                    let body = str::from_utf8(&buf).unwrap();

                                    // response
                                    let status = r.status().as_u16();
                                    let response = Response { status: status as i32, headers: Vec::new(), body: body.to_string() };
                                    let response_js = response.js(&info.isolate, &context);

                                    // add .json() helper to response object
                                    let parse_json = v8::value::Function::new(&info.isolate, &context, 0, Box::new(parse_json));
                                    response_js.set(&context, &v8::value::String::from_str(&info.isolate, "json"), &parse_json);

                                    Ok(v8::value::Value::from(response_js))
                                },
                                Err(_) => {
                                    let err = v8::value::String::from_str(&info.isolate, "Invalid Request Method!");
                                    Err(v8::value::Value::from(err)) 
                                }
                            }
                        },
                        Err(_) => {
                            let err = v8::value::String::from_str(&info.isolate, "Invalid Request!");
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
                    let request_obj = Request::from_js(&info.isolate, &context, &info.this.clone()).unwrap();

                    match request_obj.headers().iter().find(|&&ref x| x.name() == k.value()) {
                        Some(ret) => {
                            let msg = v8::value::String::from_str(&info.isolate, ret.value().as_str());
                            Ok(v8::value::Value::from(msg))
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

                    let new_header = Header::new(k.value(), v.value()).js(&info.isolate, &context);

                    let response_obj = info.this.clone();
                    let request_obj = Response::from_js(&info.isolate, &context, &response_obj);
                    let list = response_obj.get(&context, &v8::value::String::from_str(&info.isolate, "headers")).into_array().unwrap();

                    let index = request_obj.headers.len() as i32;
                    list.set(&context, &v8::value::Integer::new(&info.isolate, index), &new_header);

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