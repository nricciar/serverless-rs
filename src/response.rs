extern crate v8;

use headers::{Header};
use functions;

#[derive(Debug)]
pub struct Response {
    pub status: i32,
    pub headers: Vec<Header>,
    pub body: String,
}

impl Response {
    pub fn new() -> Response {
        Response { status: 200, headers: Vec::new(), body: "".to_string() }
    }

    pub fn js(&self, isolate: &v8::isolate::Isolate, context: &v8::context::Context) -> v8::value::Object {
        let response = v8::value::Object::new(&isolate, &context);
        response.set(&context, &v8::value::String::from_str(&isolate, "status"),
            &v8::value::Integer::new(&isolate, self.status));
        let headers = v8::value::Array::new(&isolate, &context, 0);
        response.set(&context, &v8::value::String::from_str(&isolate, "headers"), &headers);
        response.set(&context, &v8::value::String::from_str(&isolate, "body"), 
            &v8::value::String::from_str(&isolate, self.body.as_str()));

        // functions
        let add_header = v8::value::Function::new(&isolate, &context, 2, Box::new(functions::add_header));
        response.set(&context, &v8::value::String::from_str(&isolate, "addHeader"), &add_header);

        response
    }

    pub fn from_js(isolate: &v8::isolate::Isolate, context: &v8::context::Context, obj: &v8::value::Object) -> Response {
        let status = obj.get(&context, &v8::value::String::from_str(&isolate, "status"));
        let body = obj.get(&context, &v8::value::String::from_str(&isolate, "body")).into_string().unwrap().value();

        // headers
        let arr = obj.get(&context, &v8::value::String::from_str(&isolate, "headers")).into_array().unwrap();
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

        Response { status: status.into_int32().unwrap().value(),
                   headers: headers,
                   body: body }
    }
}

