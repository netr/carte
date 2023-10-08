use std::collections::HashMap;
use std::error;

use async_trait::async_trait;
use reqwest::{Error, RequestBuilder, Response};

use crate::{HttpRequester, Request};

pub struct StepManager {
    handlers: HashMap<String, Box<dyn Stepper>>,
}

impl StepManager {
    pub fn new() -> Self {
        let handlers = HashMap::new();
        StepManager {
            handlers
        }
    }

    pub fn insert(&mut self, step: impl Stepper + 'static) {
        self.handlers.insert(step.name().parse().unwrap(), Box::new(step));
    }

    pub fn get(&mut self, step: &str) -> Option<&mut Box<dyn Stepper>> {
        let step = self.handlers.get_mut(step).unwrap();
        Some(step)
    }

    pub fn len(&mut self) -> usize {
        self.handlers.len()
    }

    pub fn contains_name(&mut self, step: &String) -> bool {
        self.handlers.contains_key(step)
    }

    pub fn contains_step(&mut self, step: impl Stepper) -> bool {
        self.handlers.contains_key(step.name())
    }
}


pub struct Bot {
    pub steps: StepManager,
}


impl Bot {
    pub fn new() -> Self {
        let steps = StepManager::new();
        Bot { steps }
    }

    pub async fn handle_step(&mut self, step_id: String) -> Result<StepperStore, Box<dyn error::Error>> {
        let step = match self.steps.get(&step_id) {
            Some(step) => step,
            None => return Err(Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "step not found"))),
        };

        let stop_watch = std::time::Instant::now();

        let req = step.on_request();
        let mut store = Self::new_stepper_store(req);
        let req_builder = store.request_builder.take().unwrap();
        let res = req_builder.send().await?;

        store.time_elapsed = stop_watch.elapsed().as_millis() as u64;

        if let Some(codes) = store.success_codes.clone() {
            if !codes.contains(&res.status().as_u16()) {
                return Err(Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "step failed")));
            }
        } else {
            if !res.status().is_success() {
                return Err(Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "step failed")));
            }
        }

        store.response = Some(res);
        step.on_success(&mut store);  // Using the reference

        Ok(store)
    }

    fn new_stepper_store(req: Request) -> StepperStore {
        let http_req: HttpRequester = HttpRequester::new();
        let status_codes = req.status_codes.clone();
        let req_builder = http_req.build_reqwest(req).unwrap();

        StepperStore::new(
            http_req,
            Some(req_builder),
            None,
            None,
            status_codes,
        )
    }
}

#[async_trait]
pub trait Stepper {
    fn name(&self) -> &'static str;
    fn on_request(&mut self) -> Request;
    fn on_success(&self, store: &mut StepperStore);
    fn on_error(&self, err: Error);
    fn on_timeout(&self);
    // async fn execute(&self, res: StepperResponse) -> Result<StepperResponse, Error>;
}

pub struct StepperStore {
    pub http_requester: HttpRequester,
    pub request_builder: Option<RequestBuilder>,
    pub response: Option<Response>,
    pub next_step: Option<String>,
    pub time_elapsed: u64,
    pub success_codes: Option<Vec<u16>>,
}

impl StepperStore {
    pub fn new(
        http_requester: HttpRequester,
        request_builder: Option<RequestBuilder>,
        response: Option<Response>,
        next_step: Option<String>,
        success_codes: Option<Vec<u16>>,
    ) -> Self {
        Self {
            http_requester,
            request_builder,
            response,
            next_step,
            time_elapsed: 0,
            success_codes,
        }
    }

    pub fn set_next_step(&mut self, step: String) {
        self.next_step = Some(step);
    }

    pub fn get_next_step(&self) -> Option<String> {
        self.next_step.clone()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use reqwest::Method;
    use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};

    use crate::Request;

    use super::*;

    #[derive(Clone, Copy)]
    struct RobotsTxt;

    #[async_trait]
    impl Stepper for RobotsTxt {
        fn name(&self) -> &'static str { "RobotsTxt" }

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

        fn on_success(&self, store: &mut StepperStore) {
            // sleep for 100 ms
            std::thread::sleep(Duration::from_millis(100));
            store.set_next_step("RobotsTxt".to_string());
        }

        fn on_error(&self, _err: Error) {
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
        let store = &mut StepperStore::new(
            HttpRequester::new(),
            None,
            None,
            None,
            None,
        );
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