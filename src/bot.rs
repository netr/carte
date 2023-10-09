use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

use crate::context::Context;
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
        ctx.current_step = Some(step_name.clone());

        let req_builder = ctx.request_builder.take().unwrap();

        let res = match req_builder.send().await {
            Ok(res) => res,
            Err(err) => {
                ctx.set_time_elapsed(stop_watch.elapsed().as_millis() as u64);

                if err.is_timeout() {
                    step.on_timeout(&mut ctx);
                    return Err(StepError::ReqwestError(err.to_string()));
                }

                step.on_error(&mut ctx, StepError::ReqwestError(err.to_string()));
                return Err(StepError::ReqwestError(err.to_string()));
            }
        };

        ctx.set_time_elapsed(stop_watch.elapsed().as_millis() as u64);

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

            step.on_error(&mut ctx, error.clone());
            return Err(error);
        }

        // Everything is good, so call the step's `on_success` method.
        ctx.response_body = None;
        step.on_success(&mut ctx); // Using the reference

        Ok(ctx)
    }

    fn new_context(req: Request) -> Context {
        let mut http_req: HttpRequester = HttpRequester::new();

        // set the proxy, user agent, and compression settings before we give up ownership of the request.
        let status_codes = req.status_codes().clone();
        http_req.settings.set_proxy(req.proxy());
        http_req.settings.set_user_agent(req.user_agent());
        http_req.settings.set_compression(req.is_compressed());

        let req_builder = http_req.build_reqwest(req.clone()).unwrap();

        Context {
            request: req,
            current_step: None,
            http_requester: http_req,
            request_builder: Some(req_builder),
            response_body: None,
            next_step: None,
            status_codes,
            time_elapsed: 0,
        }
    }
}

#[async_trait]
pub trait Stepable {
    fn name(&self) -> String;
    fn on_request(&self) -> Request;
    fn on_success(&self, ctx: &mut Context);
    fn on_error(&self, ctx: &mut Context, err: StepError);
    fn on_timeout(&self, ctx: &mut Context);
    // async fn execute(&self, res: StepperResponse) -> Result<StepperResponse, Error>;
}

#[derive(Clone)]
pub struct StepManager {
    handlers: HashMap<String, Arc<dyn Stepable>>,
}

impl StepManager {
    pub fn new() -> Self {
        let handlers = HashMap::new();
        StepManager { handlers }
    }

    pub fn insert(&mut self, step: impl Stepable + 'static) {
        self.handlers
            .insert(step.name().parse().unwrap(), Arc::new(step));
    }

    pub fn insert_arc(&mut self, step: Arc<dyn Stepable>) {
        self.handlers.insert(step.name().parse().unwrap(), step);
    }

    pub fn insert_many(&mut self, steps: Vec<Arc<dyn Stepable>>) {
        for step in steps {
            self.handlers.insert(step.name().parse().unwrap(), step);
        }
    }

    pub fn get(&self, step: &str) -> Option<&Arc<dyn Stepable>> {
        let step = self.handlers.get(step).unwrap();
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

    use reqwest::header::HeaderMap;
    use reqwest::Method;

    use crate::{hdr, Request};

    use super::*;

    #[derive(Clone, Copy)]
    struct RobotsTxt;

    #[async_trait]
    impl Stepable for RobotsTxt {
        fn name(&self) -> String {
            "RobotsTxt".parse().unwrap()
        }

        fn on_request(&self) -> Request {
            let headers = hdr!(
                "User-Agent: reqwest
                Accept: */*"
            );

            Request::new(Method::GET, "https://test.com".to_string())
                .with_headers(headers)
                .with_timeout(Duration::new(30, 0))
                .with_status_codes(vec![200])
        }

        fn on_success(&self, ctx: &mut Context) {
            // sleep for 100 ms
            std::thread::sleep(Duration::from_millis(100));
            ctx.set_next_step("RobotsTxt".to_string());
        }

        fn on_error(&self, _ctx: &mut Context, _err: StepError) {
            todo!()
        }

        fn on_timeout(&self, _ctx: &mut Context) {
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
            request: Request::default(),
            current_step: None,
            http_requester: HttpRequester::new(),
            request_builder: None,
            response_body: None,
            next_step: None,
            time_elapsed: 0,
            status_codes: None,
        };
        let _ = step.on_success(store);

        assert_eq!(store.next_step, Some("RobotsTxt".to_string()));
    }

    #[tokio::test]
    async fn bot_should_call_on_request_as_expected() {
        let step = RobotsTxt {};
        let req = step.on_request();

        assert_eq!(req.method(), Method::GET);
        assert_eq!(req.status_codes(), Some(vec![200]));
    }
}
