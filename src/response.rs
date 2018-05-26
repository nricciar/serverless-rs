extern crate v8;


#[derive(Debug)]
pub struct Header {
    pub key: String,
    pub value: String,
}

#[derive(Debug)]
pub struct Response {
    pub status: i32,
    pub headers: Vec<Header>,
}

fn count_headers(context: &v8::context::Context, isolate: &v8::isolate::Isolate, arr: &v8::value::Array) -> i32 {
    let mut count = 0;
    while {
        let item = arr.get(&context, &v8::value::Integer::new(&isolate, count));
        if item.is_array() { count += 1; }
        item.is_array()
    } {}
    count
}

fn get_header_list(context: &v8::context::Context, isolate: &v8::isolate::Isolate, arr: &v8::value::Array) -> Vec<Header> {
    let mut vec = Vec::new();
    let mut count = 0;
    while {
        let item = arr.get(&context, &v8::value::Integer::new(&isolate, count));
        if item.is_array() { 
            count += 1;
            let head = item.into_array().unwrap();
            vec.push(js_array_to_header(&context, &isolate, &head).unwrap());
            true
        } else {
            false
        }
    } {}
    vec
}

pub fn append_header(context: &v8::context::Context, isolate: &v8::isolate::Isolate, arr: &v8::value::Array, head: &Header) -> bool {
    let count = count_headers(&context, &isolate, &arr);
    let h = v8::value::Array::new(&isolate, &context, 2);
    h.set(&context, &v8::value::Integer::new(&isolate, 0), &v8::value::String::from_str(&isolate, &head.key));
    h.set(&context, &v8::value::Integer::new(&isolate, 1), &v8::value::String::from_str(&isolate, &head.value));
    arr.set(&context, &v8::value::Integer::new(&isolate, count), &h);
    true
}

fn js_array_to_header(context: &v8::context::Context, isolate: &v8::isolate::Isolate, arr: &v8::value::Array) -> Result<Header,String> {
    let key = arr.get(&context, &v8::value::Integer::new(&isolate, 0));
    let value = arr.get(&context, &v8::value::Integer::new(&isolate, 1));

    match (key.is_null(), key.into_string()) {
        (false, Some(key_value)) => {
            match (value.is_null(), value.into_string()) {
                (false, Some(value_value)) => {
                    Ok(Header { key: key_value.value(), value: value_value.value() })
                },
                _ => Err("Invalid header format".to_string())
            }
        },
        _ => Err("Invalid header format".to_string())
    }
}

impl Response {
    pub fn new() -> Response {
        Response { status: 200, headers: Vec::new() }
    }

    pub fn js(&self, isolate: &v8::isolate::Isolate, context: &v8::context::Context) -> v8::value::Object {
        let response = v8::value::Object::new(&isolate, &context);
        response.set(&context, &v8::value::String::from_str(&isolate, "status"),
            &v8::value::Integer::new(&isolate, self.status));
        let headers = v8::value::Array::new(&isolate, &context, 0);
        response.set(&context, &v8::value::String::from_str(&isolate, "headers"), &headers);
        response
    }

    pub fn from_js(isolate: &v8::isolate::Isolate, context: &v8::context::Context, obj: &v8::value::Object) -> Response {
        let status = obj.get(&context, &v8::value::String::from_str(&isolate, "status"));
        let headers = obj.get(&context, &v8::value::String::from_str(&isolate, "headers")).into_array().unwrap();

        Response { status: status.into_int32().unwrap().value(),
                   headers: get_header_list(&context, &isolate, &headers) }
    }
}
