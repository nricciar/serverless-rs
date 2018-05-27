extern crate v8;

use response;

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