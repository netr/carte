use std::collections::HashMap;

use async_trait::async_trait;
use reqwest::{RequestBuilder, Response};

use crate::{HttpRequester, Request, StepError};

pub struct Bot {
    pub steps: StepManager,
}

impl Bot {
    pub fn new() -> Self {
        let steps = StepManager::new();
        Bot { steps }
    }

    /// Handles the step by executing the request and calling the step's `on_success` or `on_error` methods.
    pub async fn handle_step(&mut self, step_name: String) -> Result<Context, StepError> {
        let step = match self.steps.get(&step_name) {
            Some(step) => step,
            None => {
                return Err(StepError::StepNotFound(step_name.clone()));
            }
        };

        // Start processing the request and time it.
        let stop_watch = std::time::Instant::now();

        let req = step.on_request();
        let mut ctx = Self::new_context(req);
        let req_builder = ctx.request_builder.take().unwrap();

        let res = match req_builder.send().await {
            Ok(res) => res,
            Err(err) => {
                step.on_error(StepError::ReqwestError(err.to_string()));
                return Err(StepError::ReqwestError(err.to_string()));
            }
        };

        ctx.time_elapsed = stop_watch.elapsed().as_millis() as u64;

        // Check if the status code is in the list of expected status codes.
        let status_code = res.status().as_u16();
        let expected_codes = ctx.status_codes.as_ref();

        let error_condition = if let Some(codes) = expected_codes {
            !codes.contains(&status_code)
        } else {
            !res.status().is_success()
        };

        if error_condition {
            let error = StepError::StatusCodeNotFound(
                status_code as i32,
                expected_codes.cloned().unwrap_or_else(Vec::new),
            );

            step.on_error(error.clone());
            return Err(error);
        }

        // Everything is good, so call the step's `on_success` method.
        ctx.response = Some(res);
        step.on_success(&mut ctx); // Using the reference

        Ok(ctx)
    }

    fn new_context(req: Request) -> Context {
        let http_req: HttpRequester = HttpRequester::new();
        let status_codes = req.status_codes.clone();
        let req_builder = http_req.build_reqwest(req).unwrap();

        Context {
            http_requester: http_req,
            request_builder: Some(req_builder),
            response: None,
            next_step: None,
            status_codes,
            time_elapsed: 0,
        }
    }
}

/// The context for the bots current step's execution.
/// This is passed to the step's `on_success` and `on_error` methods.
pub struct Context {
    /// The HTTP requester which manages cookie store and client settings.
    pub http_requester: HttpRequester,
    /// The request builder from reqwest.
    pub request_builder: Option<RequestBuilder>,
    /// The response from the request.
    pub response: Option<Response>,
    /// The next step to be executed.
    pub next_step: Option<String>,
    /// If status codes are provided, then the response status code must be in the list.
    pub status_codes: Option<Vec<u16>>,
    /// The time elapsed in milliseconds for the request.
    pub time_elapsed: u64,
}

impl Context {
    pub fn set_next_step(&mut self, step: String) {
        self.next_step = Some(step);
    }

    pub fn get_next_step(&self) -> Option<String> {
        self.next_step.clone()
    }

    pub fn get_time_elapsed(&self) -> u64 {
        self.time_elapsed
    }

    pub fn set_time_elapsed(&mut self, time_elapsed: u64) {
        self.time_elapsed = time_elapsed;
    }
}

#[async_trait]
pub trait Stepable {
    fn name(&self) -> String;
    fn on_request(&mut self) -> Request;
    fn on_success(&self, ctx: &mut Context);
    fn on_error(&self, err: StepError);
    fn on_timeout(&self);
    // async fn execute(&self, res: StepperResponse) -> Result<StepperResponse, Error>;
}

pub struct StepManager {
    handlers: HashMap<String, Box<dyn Stepable>>,
}

impl StepManager {
    pub fn new() -> Self {
        let handlers = HashMap::new();
        StepManager { handlers }
    }

    pub fn insert(&mut self, step: impl Stepable + 'static) {
        self.handlers
            .insert(step.name().parse().unwrap(), Box::new(step));
    }

    pub fn insert_box(&mut self, step: Box<dyn Stepable>) {
        self.handlers.insert(step.name().parse().unwrap(), step);
    }

    pub fn insert_many(&mut self, steps: Vec<Box<dyn Stepable>>) {
        for step in steps {
            self.handlers.insert(step.name().parse().unwrap(), step);
        }
    }

    pub fn get(&mut self, step: &str) -> Option<&mut Box<dyn Stepable>> {
        let step = self.handlers.get_mut(step).unwrap();
        Some(step)
    }

    pub fn len(&mut self) -> usize {
        self.handlers.len()
    }

    pub fn contains_name(&mut self, step: &String) -> bool {
        self.handlers.contains_key(step)
    }

    pub fn contains_step(&mut self, step: impl Stepable) -> bool {
        self.handlers.contains_key(step.name().as_str())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
    use reqwest::Method;

    use crate::Request;

    use super::*;

    #[derive(Clone, Copy)]
    struct RobotsTxt;

    #[async_trait]
    impl Stepable for RobotsTxt {
        fn name(&self) -> String {
            "RobotsTxt".parse().unwrap()
        }

        fn on_request(&mut self) -> Request {
            let mut headers = HeaderMap::new();
            headers.insert(USER_AGENT, HeaderValue::from_static("reqwest"));

            Request {
                method: Method::GET,
                url: "https://test.com".to_string(),
                headers: Some(headers),
                timeout: Some(Duration::new(30, 0)),
                body: None,
                status_codes: Some(vec![200]),
            }
        }

        fn on_success(&self, ctx: &mut Context) {
            // sleep for 100 ms
            std::thread::sleep(Duration::from_millis(100));
            ctx.set_next_step("RobotsTxt".to_string());
        }

        fn on_error(&self, _err: StepError) {
            todo!()
        }

        fn on_timeout(&self) {
            todo!()
        }
    }

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
        assert!(bot.steps.contains_step(step));
    }

    #[tokio::test]
    async fn bot_should_have_next_step_in_store_as_expected() {
        let step = RobotsTxt {};
        let store = &mut Context {
            http_requester: HttpRequester::new(),
            request_builder: None,
            response: None,
            next_step: None,
            time_elapsed: 0,
            status_codes: None,
        };
        let _ = step.on_success(store);

        assert_eq!(store.next_step, Some("RobotsTxt".to_string()));
    }

    #[tokio::test]
    async fn bot_should_call_on_request_as_expected() {
        let mut step = RobotsTxt {};
        let req = step.on_request();

        assert_eq!(req.method, Method::GET);
        assert_eq!(req.status_codes, Some(vec![200]));
    }
}
