use std::time::Duration;

use reqwest::{Body, Method};
use reqwest::header::HeaderMap;

#[derive(Debug)]
pub struct Request {
    pub method: Method,
    pub url: String,
    pub headers: Option<HeaderMap>,
    pub timeout: Option<Duration>,
    pub body: Option<Body>,
}


impl Request {
    pub fn new(method: Method, url: String) -> Self {
        Self {
            method,
            url,
            headers: None,
            timeout: Some(Duration::new(30, 0)),
            body: None,
        }
    }
}

impl Default for Request {
    fn default() -> Self {
        Self {
            method: Method::GET,
            url: "/".to_string(),
            headers: None,
            timeout: Some(Duration::new(30, 0)),
            body: None,
        }
    }
}

