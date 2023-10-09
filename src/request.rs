use std::time::Duration;

use reqwest::header::HeaderMap;
use reqwest::{Body, Method, Proxy};

#[derive(Debug, Clone)]
pub struct Request {
    method: Method,
    url: String,
    headers: Option<HeaderMap>,
    timeout: Option<Duration>,
    body: Option<MimicBody>,
    status_codes: Option<Vec<u16>>,
    proxy: Option<Proxy>,
    user_agent: Option<String>,
    gzip: bool,
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
            proxy: None,
            user_agent: None,
            gzip: true,
        }
    }

    pub fn method(&self) -> Method {
        self.method.clone()
    }

    pub fn url(&self) -> &String {
        &self.url
    }

    pub fn with_headers(mut self, headers: HeaderMap) -> Self {
        self.headers = Some(headers);
        self
    }

    pub fn headers(&self) -> Option<HeaderMap> {
        self.headers.clone()
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn timeout(&self) -> Option<Duration> {
        self.timeout.clone()
    }

    pub fn with_body(mut self, body: MimicBody) -> Self {
        self.body = Some(body);
        self
    }

    pub fn body(self) -> Option<Body> {
        if let Some(b) = self.body {
            Some(Body::from(b))
        } else {
            None
        }
    }

    pub fn with_status_codes(mut self, status_codes: Vec<u16>) -> Self {
        self.status_codes = Some(status_codes);
        self
    }

    pub fn status_codes(&self) -> Option<Vec<u16>> {
        self.status_codes.clone()
    }

    pub fn with_proxy(mut self, proxy: Proxy) -> Self {
        self.proxy = Some(proxy);
        self
    }

    pub fn proxy(&self) -> Option<Proxy> {
        self.proxy.clone()
    }

    pub fn with_user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = Some(user_agent);
        self
    }

    pub fn user_agent(&self) -> Option<String> {
        self.user_agent.clone()
    }

    pub fn compressed(mut self) -> Self {
        self.gzip = true;
        self
    }

    pub fn is_compressed(&self) -> bool {
        self.gzip
    }

    pub fn no_compression(mut self) -> Self {
        self.gzip = false;
        self
    }

    pub fn build(self) -> Self {
        self
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
            proxy: None,
            user_agent: None,
            gzip: true,
        }
    }
}

#[derive(Debug, Clone)]
pub enum MimicBody {
    Bytes(Vec<u8>),
    Text(String),
}

impl MimicBody {
    pub fn from_bytes(data: Vec<u8>) -> Self {
        Self::Bytes(data)
    }

    pub fn from_text(data: String) -> Self {
        Self::Text(data)
    }
}

impl From<MimicBody> for reqwest::Body {
    fn from(body: MimicBody) -> reqwest::Body {
        match body {
            MimicBody::Bytes(bytes) => reqwest::Body::from(bytes),
            MimicBody::Text(text) => reqwest::Body::from(text),
        }
    }
}

#[allow(unused_macros)]
#[macro_export]
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
    fn it_should_work_properly_with_a_blob_of_text_based_headers() {
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

    #[test]
    fn it_should_use_the_request_builder_pattern_to_create_a_basic_request() {
        let req = Request::new(Method::GET, "https://google.com".to_string())
            .with_headers(hdr!("Accept-Encoding: gzip, deflate, br"))
            .with_timeout(Duration::new(710, 0))
            .with_status_codes(vec![200, 210, 222])
            .with_proxy(Proxy::http("https://secure.example").unwrap())
            .with_user_agent("reqwest".to_string())
            .no_compression()
            .build();
        assert_eq!(req.method, Method::GET);
        assert_eq!(req.url, "https://google.com");
        assert_eq!(req.headers().unwrap().len(), 1);
        assert_eq!(req.timeout().unwrap().as_secs(), 710);
        assert_eq!(req.status_codes().unwrap().len(), 3);
        assert_eq!(
            format!("{:?}", req.proxy().unwrap()),
            "Proxy(Http(https://secure.example), None)"
        );
        assert_eq!(req.user_agent().unwrap(), "reqwest");
        assert_eq!(req.is_compressed(), false);
    }
}
