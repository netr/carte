use std::error::Error;

use encoding_rs::{Encoding, UTF_8};
use reqwest::RequestBuilder;
use serde::de::DeserializeOwned;

use crate::{HttpRequester, Request};

/// The context for the bots current step's execution.
/// This is passed to the step's `on_success` and `on_error` methods.
pub struct Context {
    /// The original request struct.
    pub request: Request,
    /// The next step to be executed.
    pub current_step: Option<String>,
    /// The HTTP requester which manages cookie store and client settings.
    pub http_requester: HttpRequester,
    /// The request builder from reqwest.
    pub request_builder: Option<RequestBuilder>,
    /// The response from the request.
    pub response_body: Option<bytes::Bytes>,
    /// The next step to be executed.
    pub next_step: Option<String>,
    /// If status codes are provided, then the response status code must be in the list.
    pub status_codes: Option<Vec<u16>>,
    /// The time elapsed in milliseconds for the request.
    pub time_elapsed: u64,
}

impl Context {
    pub fn new() -> Self {
        let request = Request::default();
        let http_requester = HttpRequester::new();
        let request_builder = http_requester.build_reqwest(request.clone()).unwrap();

        Context {
            request,
            current_step: None,
            http_requester,
            request_builder: Some(request_builder),
            response_body: None,
            next_step: None,
            status_codes: None,
            time_elapsed: 0,
        }
    }

    /// Sets the current step.
    pub fn set_current_step(&mut self, step: String) {
        self.current_step = Some(step);
    }

    /// Gets the current step.
    pub fn get_current_step(&self) -> Option<String> {
        self.current_step.clone()
    }

    /// Sets the next step.
    pub fn set_next_step(&mut self, step: String) {
        self.next_step = Some(step);
    }

    /// Clears the next step.
    pub fn clear_next_step(&mut self) {
        self.next_step = None;
    }

    /// Gets the next step.
    pub fn get_next_step(&self) -> Option<String> {
        self.next_step.clone()
    }

    /// Get the time elapsed in milliseconds.
    pub fn get_time_elapsed(&self) -> u64 {
        self.time_elapsed
    }

    /// Sets the time elapsed in milliseconds.
    pub fn set_time_elapsed(&mut self, time_elapsed: u64) {
        self.time_elapsed = time_elapsed;
    }

    /// Gets the time elapsed as a string. This is useful for logging.
    pub fn get_time_elapsed_as_string(&self) -> String {
        format!("{} ms", self.time_elapsed)
    }
    /// Sets the request builder.
    pub fn set_request_builder(&mut self, req_builder: RequestBuilder) {
        self.request_builder = Some(req_builder);
    }

    pub fn get_url(&self) -> String {
        self.request.url().clone()
    }

    /// Sets the response body in bytes.
    pub fn set_response_body(&mut self, res: bytes::Bytes) {
        self.response_body = Some(res);
    }

    /// Returns the response body as bytes.
    /// This is the base format for the response body. All other methods are convenience methods.
    pub fn body_bytes(&self) -> Result<bytes::Bytes, Box<dyn Error>> {
        if self.response_body.is_none() {
            return Err(Self::no_body_error());
        }

        Ok(self.response_body.clone().unwrap())
    }

    /// Returns the response body as text. This is a convenience method for `encoding_rs::decode`.
    pub fn body_text(&self) -> Result<String, Box<dyn Error>> {
        if self.response_body.is_none() {
            return Err(Self::no_body_error());
        }

        let encoding = Encoding::for_label(b"utf-8").unwrap_or(UTF_8);
        let (text, _, _) = encoding.decode(&self.response_body.as_ref().unwrap());

        Ok(text.to_string())
    }

    /// Returns the response body as JSON. This is a convenience method for `serde_json::from_slice`.
    pub async fn body_json<T: DeserializeOwned>(&self) -> Result<T, Box<dyn Error>> {
        if self.response_body.is_none() {
            return Err(Self::no_body_error());
        }

        serde_json::from_slice(&self.response_body.as_ref().unwrap())
            .map_err(|err| -> Box<dyn std::error::Error> { Box::new(err) })
    }

    fn no_body_error() -> Box<dyn Error> {
        return Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "No body has been set from the request.",
        ));
    }

    /// Updates the context from the request.
    /// This is useful for updating the success status codes, proxy, user agent, and compression settings.
    pub fn update_from_request(&mut self, req: Request) -> Result<(), Box<dyn std::error::Error>> {
        self.http_requester.settings.set_proxy(req.proxy());
        self.http_requester
            .settings
            .set_user_agent(req.user_agent());
        self.http_requester
            .settings
            .set_compression(req.is_compressed());

        self.status_codes = req.status_codes().clone();

        if let Ok(builder) = self.http_requester.build_reqwest(req.clone()) {
            self.request_builder = Some(builder);
        } else {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Unable to build request",
            )));
        }

        self.request = req;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_should_get_current_step_without_one_set() {
        let ctx = Context::new();
        assert_eq!(ctx.get_current_step(), None);
    }

    #[test]
    fn context_should_get_url_without_one_set() {
        let ctx = Context::new();
        assert_eq!(ctx.get_url(), "/");
    }

    #[test]
    fn context_body_test_should_throw_error_if_not_initialized() {
        let ctx = Context::new();

        let err = ctx.body_text().unwrap_err();
        assert_eq!(err.to_string(), "No body has been set from the request.");
    }

    #[test]
    fn context_body_bytes_should_throw_error_if_not_initialized() {
        let ctx = Context::new();

        let err = ctx.body_bytes().unwrap_err();
        assert_eq!(err.to_string(), "No body has been set from the request.");
    }

    #[tokio::test]
    async fn context_body_json_should_mock_response_and_get_name() {
        let mut ctx = Context::new();
        let res = bytes::Bytes::from_static(b"{\"name\": \"test\"}");
        ctx.set_response_body(res);

        let json: serde_json::Value = ctx.body_json().await.unwrap();
        assert_eq!(json["name"], "test");
    }

    #[tokio::test]
    async fn context_body_json_should_return_error_if_invalid_json() {
        let mut ctx = Context::new();
        let res = bytes::Bytes::from_static(b"{\"name\": \"test\"");
        ctx.set_response_body(res);

        let err = ctx.body_json::<serde_json::Value>().await.unwrap_err();
        assert!(err.to_string().len() > 0);
    }
}
