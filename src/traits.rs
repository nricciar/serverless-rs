extern crate url;
extern crate v8;
extern crate serde;
extern crate serde_json;

use request::{Method};

pub trait CanParse {
    fn parse(self) -> Result<url::Url, url::ParseError>;
}
impl CanParse for url::Url {
    fn parse(self) -> Result<url::Url, url::ParseError> {
        Ok(self)
    }
}
impl<'a> CanParse for &'a str {
    fn parse(self) -> Result<url::Url, url::ParseError> {
        url::Url::parse(self)
    }
}
impl CanParse for String {
    fn parse(self) -> Result<url::Url, url::ParseError> {
        url::Url::parse(self.as_str())
    }
}

pub trait ToString {
    fn get_string(self) -> String;
}
impl ToString for String {
    fn get_string(self) -> String {
        self.to_string()
    }
}
impl<'a> ToString for &'a str {
    fn get_string(self) -> String {
        self.to_string()
    }
}
impl ToString for Method {
    fn get_string(self) -> String {
        self.to_string()
    }
}