#![allow(dead_code, unused_variables)]

use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT, CONTENT_TYPE, HeaderName};
use std::collections::HashMap;

pub struct Context {
    // logger, file manager,
}

pub struct ResponseData {
    // Your fields here
}

impl ResponseData {
    fn new() -> Self {
        ResponseData {}
    }
}

pub enum Error {
    NetworkError,
    Timeout,
}

pub struct RequestCreator {}

impl RequestCreator {
    fn new() -> Self {
        RequestCreator {}
    }
}

pub trait HttpRequester {
    fn name(&self) -> &'static str;
    fn on_request(&mut self, ctx: &mut Context, req: &mut RequestCreator);
    fn on_success(&self, ctx: &Context, response: ResponseData);
    fn on_error(&self, ctx: &Context, response: ResponseData, err: Error);
    fn on_timeout(&self, ctx: &Context, request: &RequestCreator);
    fn execute(&self, previous_data: Option<&ResponseData>) -> Result<ResponseData, Error>;
}

pub struct Bot {
    pub steps: StepManager,
    context: Context,
}

pub struct StepManager {
    handlers: HashMap<String, Box<dyn HttpRequester>>,
}

impl StepManager {
    pub fn new() -> Self {
        let handlers = HashMap::new();
        StepManager {
            handlers
        }
    }

    pub fn insert(&mut self, step: impl HttpRequester + 'static) {
        self.handlers.insert(step.name().parse().unwrap(), Box::new(step));
    }

    pub fn get(&mut self, step: &str) -> Option<&mut Box<dyn HttpRequester>> {
        let step = self.handlers.get_mut(step).unwrap();
        Some(step)
    }

    pub fn len(&mut self) -> usize {
        self.handlers.len()
    }

    pub fn contains(&mut self, step: impl HttpRequester) -> bool {
        self.handlers.contains_key(step.name())
    }
}

impl Bot {
    pub fn new() -> Self {
        let steps = StepManager::new();
        let context = Context {};
        Bot { steps, context }
    }

    async fn handle_step(&mut self, step: impl HttpRequester + Copy) -> Result<(), &'static str> {
        if !self.steps.contains(step) {
            return Err("invalid step");
        }

        // Initialize a RequestCreator object or similar
        let mut req_creator = RequestCreator::new();

        let step = self.steps.get(step.name()).unwrap();
        step.on_request(&mut self.context, &mut req_creator);

        match step.execute(None) {
            Ok(response_data) => {
                Ok(step.on_success(&self.context, response_data))
            }
            Err(Error::Timeout) => {
                Ok(step.on_timeout(&self.context, &req_creator))
            }
            Err(err) => {
                Ok(step.on_error(&self.context, ResponseData::new(), err))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_initializes() {
        let mut bot = Bot::new();
        assert_eq!(bot.steps.len(), 0);
    }

    #[test]
    fn it_adds_step() {
        let mut bot = Bot::new();
        let step = RobotsTxt {};
        bot.steps.insert(step);
        assert_eq!(bot.steps.len(), 1);
        assert!(bot.steps.contains(step));
    }

    // #[test]
    // fn it_handles_step() {
    //     let mut bot = Bot::new();
    //     let step = RobotsTxt {};
    //     bot.steps.insert(step);
    //     assert!(!bot.handle_step(step).is_err());
    // }
    //
    // #[test]
    // fn it_should_error_if_step_is_not_found() {
    //     let mut bot = Bot::new();
    //     assert!(bot.handle_step(RobotsTxt {}).is_err());
    // }
}

struct Reqwester {}

pub trait HttpClient {
    fn send_request(&self, req_creator: &RequestCreator) -> Result<ResponseData, Error>;
}

impl HttpClient for Reqwester {
    fn send_request(&self, req_creator: &RequestCreator) -> Result<ResponseData, Error> {
        todo!()
    }
}

#[derive(Clone, Copy)]
struct RobotsTxt;

impl HttpRequester for RobotsTxt {
    fn name(&self) -> &'static str { "RobotsTxt" }

    fn on_request(&mut self, _ctx: &mut Context, _req: &mut RequestCreator) {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("reqwest"));
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("image/png"));
        headers.insert(HeaderName::from_static("x-api-key"), HeaderValue::from_static("123-123-123"));

        // let client = reqwest::Client::new();
        // let response = client
        //     .get("https://aishowcase.io")
        //     .headers(headers)
        //     .send()
        //     .await
        //     .unwrap();
        // println!("Success! {:?}", response);
    }

    fn on_success(&self, _ctx: &Context, _response: ResponseData) {}

    fn on_error(&self, _ctx: &Context, _response: ResponseData, err: Error) {
        // Handle error
    }

    fn on_timeout(&self, _ctx: &Context, _request: &RequestCreator) {
        // Handle timeout
    }

    fn execute(&self, _previous_data: Option<&ResponseData>) -> Result<ResponseData, Error> {
        // Your execution logic here
        Ok(ResponseData {})
    }
}