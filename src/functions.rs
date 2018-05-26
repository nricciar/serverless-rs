extern crate v8;

use response;

pub fn hello(info: v8::value::FunctionCallbackInfo) -> Result<v8::value::Value, v8::value::Value> {
    let test = v8::value::String::from_str(&info.isolate, "World!");
    Ok(v8::value::Value::from(test))
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