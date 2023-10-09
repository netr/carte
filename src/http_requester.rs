use std::sync::Arc;
use std::time::Duration;

use reqwest::header::HeaderMap;
use reqwest::{Body, Client, IntoUrl, Method, RequestBuilder, Response};
use reqwest_cookie_store::{CookieStore, CookieStoreMutex};

// http_requester.rs
use crate::client_settings::ClientSettings;
use crate::request::Request;

#[derive(Clone)]
pub struct HttpRequester {
    cookie_store: Arc<CookieStoreMutex>,
    pub settings: Box<ClientSettings>,
}

impl HttpRequester {
    pub fn new() -> Self {
        let cookie_store = new_cookie_store();
        let settings = ClientSettings::new();

        Self {
            cookie_store,
            settings: Box::new(settings),
        }
    }

    /// Builds a client with all of the internal client settings.
    /// We are unable to attach proxies, gzip, etc. with a client that has already been initialized.
    fn build_client(&self) -> Result<Client, reqwest::Error> {
        let mut builder = Client::builder()
            .cookie_provider(std::sync::Arc::clone(&self.cookie_store))
            .gzip(self.settings.is_compressed());

        if let Some(proxy) = self.settings.proxy() {
            builder = builder.proxy(proxy.clone());
        }

        if let Some(ua) = self.settings.user_agent() {
            builder = builder.user_agent(ua.clone());
        }

        builder.build()
    }

    /// Sends a request with all of the internal client settings.
    pub async fn req<U, B, H>(
        &self,
        method: Method,
        url: U,
        body: B,
        headers: H,
    ) -> Result<Response, reqwest::Error>
    where
        U: IntoUrl,
        B: Into<Option<Body>>,
        H: Into<Option<HeaderMap>>,
    {
        let client = &self.build_client()?;

        let mut client = client.request(method, url).timeout(Duration::new(30, 0));

        if let Some(h) = headers.into() {
            client = client.headers(h);
        }
        if let Some(b) = body.into() {
            client = client.body(b);
        }

        let res = client.send().await?;

        Ok(res)
    }

    /// Sends a request with all of the internal client settings.
    pub fn build_reqwest(&self, req: Request) -> Result<RequestBuilder, reqwest::Error> {
        let client = &self.build_client()?;

        let mut client = client
            .request(req.method(), req.url())
            .timeout(Duration::new(30, 0));

        match req.timeout().into() {
            Some(to) => client = client.timeout(to),
            None => client = client.timeout(Duration::new(30, 0)),
        }

        if let Some(h) = req.headers().into() {
            client = client.headers(h);
        }
        if let Some(b) = req.body() {
            client = client.body(b);
        }
        if let Some(f) = req.multipart() {
            client = client.multipart(f);
        }

        Ok(client)
    }

    // Method to get cookies as JSON string
    pub fn get_cookies(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = Vec::new();
        let store = self.cookie_store.lock().unwrap();
        store.save_json(&mut buffer).unwrap();
        buffer
    }
}

fn new_cookie_store() -> Arc<CookieStoreMutex> {
    let cookie_store = CookieStoreMutex::new(CookieStore::new(None));
    let cookie_store = Arc::new(cookie_store);
    cookie_store
}

#[cfg(test)]
mod tests {
    use crate::request::{MimicBody, MimicForm};
    use reqwest::header::HeaderValue;
    use reqwest::Proxy;

    use super::*;

    #[test]
    fn it_should_set_proxy() {
        let mut req = HttpRequester::new();
        let proxy = Proxy::http("https://secure.example").unwrap();
        req.settings.set_proxy(Some(proxy.clone()));

        assert_eq!(
            format!("{:?}", req.settings.proxy().unwrap()),
            format!("{:?}", proxy)
        )
    }

    #[test]
    fn it_should_set_proxy_as_none() {
        let mut req = HttpRequester::new();
        let proxy = Proxy::http("https://secure.example").unwrap();
        req.settings.set_proxy(Some(proxy.clone()));

        assert_eq!(
            format!("{:?}", req.settings.proxy().unwrap()),
            format!("{:?}", proxy)
        );

        req.settings.set_proxy(None);

        assert!(req.settings.proxy().is_none())
    }

    #[test]
    fn it_should_set_user_agent() {
        let expected = "useragent".to_string();

        let mut req = HttpRequester::new();
        req.settings.set_user_agent(Some(expected.clone()));

        assert_eq!(req.settings.user_agent().unwrap(), &expected)
    }

    #[test]
    fn it_should_disable_compression() {
        let mut req = HttpRequester::new();
        req.settings.disable_compression();

        assert!(!req.settings.is_compressed())
    }

    #[test]
    fn it_should_disable_and_re_enable_compression() {
        let mut req = HttpRequester::new();
        req.settings.disable_compression();
        assert!(!req.settings.is_compressed());
        req.settings.enable_compression();
        assert!(req.settings.is_compressed())
    }

    #[test]
    fn it_should_build_clients() {
        let mut req = HttpRequester::new();
        req.settings.disable_compression();

        let proxy = Proxy::http("https://secure.example").unwrap();
        req.settings.set_proxy(Some(proxy.clone()));

        let expected = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/116.0.0.0 Safari/537.36".to_string();
        req.settings.set_user_agent(Some(expected.clone()));

        match req.build_client() {
            Ok(client) => {
                assert!(format!("{:?}", client).contains("gzip: false"));
                assert!(format!("{:?}", client).contains("Http(https://secure.example)"));
                assert!(format!("{:?}", client).contains("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/116.0.0.0 Safari/537.36"));
            }
            Err(_) => panic!("invalid"),
        }
    }

    #[test]
    fn it_should_build_a_request() {
        let http = HttpRequester::new();
        let mut headers = HeaderMap::new();
        headers.insert("X-API-KEY", HeaderValue::from_static("1234"));

        let req = Request::new(Method::POST, "https://google.com".to_string())
            .with_headers(headers)
            .with_body(MimicBody::from_bytes(vec![2, 3, 4]))
            .with_status_codes(vec![200])
            .build();

        match http.build_reqwest(req) {
            Ok(b) => {
                println!("{:?}", b);
                assert!(format!("{:?}", b).contains("method: POST"));
                assert!(format!("{:?}", b).contains("google.com"));
                assert!(format!("{:?}", b).contains("x-api-key"));
            }
            Err(_) => panic!("invalid"),
        }
    }

    #[test]
    fn it_should_build_a_request_with_multipart() {
        let http = HttpRequester::new();
        let mut headers = HeaderMap::new();
        headers.insert("X-API-KEY", HeaderValue::from_static("1234"));

        let form = MimicForm::new(
            vec![("name".to_string(), "value".to_string())],
            vec![("name".to_string(), vec![1, 2, 3])],
        );

        let req = Request::new(Method::POST, "https://google.com".to_string())
            .with_headers(headers)
            .with_multipart(form)
            .with_status_codes(vec![200])
            .build();

        match http.build_reqwest(req) {
            Ok(b) => {
                println!("{:?}", b);
                assert!(format!("{:?}", b).contains("method: POST"));
                assert!(format!("{:?}", b).contains("multipart/form-data; boundary="));
            }
            Err(_) => panic!("invalid"),
        }
    }

    #[test]
    fn it_should_build_a_request_using_default() {
        let http = HttpRequester::new();
        let req = Request::new(Method::POST, "https://test.com".to_string());

        match http.build_reqwest(req) {
            Ok(b) => {
                assert!(format!("{:?}", b).contains("method: POST"));
                assert!(format!("{:?}", b).contains("test.com"));
            }
            Err(_) => panic!("invalid"),
        }
    }

    #[test]
    fn it_should_build_a_request_using_new() {
        let http = HttpRequester::new();
        let req = Request::new(Method::PATCH, "https://aol.com".to_string());

        match http.build_reqwest(req) {
            Ok(b) => {
                assert!(format!("{:?}", b).contains("method: PATCH"));
                assert!(format!("{:?}", b).contains("aol.com"));
            }
            Err(_) => panic!("invalid"),
        }
    }
}
