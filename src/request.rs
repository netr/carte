use std::time::Duration;

use reqwest::header::HeaderMap;
use reqwest::{Body, Method};

#[derive(Debug)]
pub struct Request {
    pub method: Method,
    pub url: String,
    pub headers: Option<HeaderMap>,
    pub timeout: Option<Duration>,
    pub body: Option<Body>,
    pub status_codes: Option<Vec<u16>>,
}

impl Request {
    pub fn new(method: Method, url: String) -> Self {
        Self {
            method,
            url,
            headers: None,
            timeout: Some(Duration::new(30, 0)),
            body: None,
            status_codes: None,
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
            status_codes: None,
        }
    }
}

macro_rules! hdr {
    ($text:expr) => {{
        let mut headers = HeaderMap::new();
        for line in $text.lines() {
            if line.is_empty() || !line.contains(":") {
                continue;
            }
            // split at the first occurrence of ":" and only take the first two parts
            // this should split most headers correctly
            let mut parts = line.splitn(2, ":");
            let key = parts.next().unwrap().trim();
            let value = parts.next().unwrap().trim();
            headers.insert(key, value.parse().unwrap());
        }
        headers
    }};
    () => {{
        let headers = HeaderMap::new();
        headers
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_properly_use_hdr_macro_to_parse_large_amount_of_headers() {
        let text = r#"Accept-Encoding: gzip, deflate, br
Referer:https://github.com/rust-lang/rust
User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/116.0.0.0 Safari/537.36
X-Requested-With: XMLHttpRequest"#;

        let headers = hdr!(text);
        assert_eq!(headers.len(), 4);
        assert_eq!(
            headers.get("User-Agent").unwrap().to_str().unwrap(),
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/116.0.0.0 Safari/537.36",
            "we are testing a very long user agent string"
        );
        assert_eq!(
            headers.get("Referer").unwrap().to_str().unwrap(),
            "https://github.com/rust-lang/rust",
            "testing a value with `:` in it, which is why we use `splitn`. also testing no space between key and value"
        );
        assert_eq!(
            headers.get("Accept-Encoding").unwrap().to_str().unwrap(),
            "gzip, deflate, br",
            "testing spaces"
        );
    }

    #[test]
    fn it_should_return_no_headers_if_empty() {
        let headers = hdr!();
        assert_eq!(headers.len(), 0);
    }

    #[test]
    fn it_should_return_no_headers_if_invalid_text() {
        let headers = hdr!("this is not a real header and should not work");
        assert_eq!(headers.len(), 0);
    }
}
