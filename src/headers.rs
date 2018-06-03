extern crate serde;
extern crate serde_json;
extern crate v8;

use traits::{ToString};

#[derive(Debug, Serialize, Deserialize)]
pub struct Header {
    name: String,
    value: String,
}

impl Header {
    pub fn new<T: ToString>(name: T, value: T) -> Header {
        Header { name: name.get_string(), value: value.get_string() }
    }

    pub fn js(&self, isolate: &v8::isolate::Isolate, context: &v8::context::Context) -> v8::value::Array {
        let h = v8::value::Array::new(&isolate, &context, 2);
        h.set(&context, &v8::value::Integer::new(&isolate, 0), &v8::value::String::from_str(&isolate, self.name.as_str()));
        h.set(&context, &v8::value::Integer::new(&isolate, 1), &v8::value::String::from_str(&isolate, self.value.as_str()));
        h
    }

    pub fn from_js(isolate: &v8::isolate::Isolate, context: &v8::context::Context, js: &v8::value::Value) -> Result<Header,String> {
        match js.clone().into_array() {
            Some(arr) => {
                let key = arr.get(&context, &v8::value::Integer::new(&isolate, 0));
                let value = arr.get(&context, &v8::value::Integer::new(&isolate, 1));

                match ((key.is_null(), key.into_string()), (value.is_null(), value.into_string())) {
                    ((false, Some(key_value)), (false, Some(value_value))) => {
                        Ok(Header{ name: key_value.value(), value: value_value.value() })
                    },
                    _ => Err("Invalid Header".to_string())
                }
            },
            None => Err("Invalid Header".to_string()),
        }
    }

    pub fn name(&self) -> String {
        self.name.to_string()
    }

    pub fn value(&self) -> String {
        self.value.to_string()
    }
}