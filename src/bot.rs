use std::collections::HashMap;
use std::error;
use reqwest::{Error, Request, RequestBuilder, Response};
use async_trait::async_trait;
use crate::HttpRequester;

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

    pub async fn handle_step(&mut self, step_id: String) -> Result<StepperResponse, Box<dyn error::Error>> {
        let step = match self.steps.get(&step_id) {
            Some(step) => step,
            None => return Err(Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "step not found"))),
        };

        let s = step.on_request();

        match step.execute(s).await {
            Ok(res) => {
                println!("{:?}", &res.response.as_ref().unwrap());
                Ok(res)
            }
            Err(err) => Err(Box::try_from(err).unwrap()),
        }
    }
}

#[async_trait]
pub trait Stepper {
    fn name(&self) -> &'static str;
    fn on_request(&mut self) -> StepperResponse;
    async fn on_success(self, res: &StepperResponse);
    fn on_error(&self, err: Error);
    fn on_timeout(&self);
    async fn execute(&self, res: StepperResponse) -> Result<StepperResponse, Error>;
}

pub struct StepperResponse {
    pub http_requester: HttpRequester,
    pub request_builder: Option<RequestBuilder>,
    pub response: Option<Response>,
}

impl StepperResponse {
    pub fn new(http_requester: HttpRequester, request_builder: Option<RequestBuilder>, response: Option<Response>) -> Self {
        Self {
            http_requester,
            request_builder,
            response,
        }
    }
}

#[cfg(test)]
mod tests {
    #[derive(Clone, Copy)]
    struct RobotsTxt;

    #[async_trait]
    impl Stepper for RobotsTxt {
        fn name(&self) -> &'static str { "RobotsTxt" }
        fn on_request(&mut self) -> StepperResponse {
            let req = HttpRequester::new();

            let mut headers = HeaderMap::new();
            headers.insert(USER_AGENT, HeaderValue::from_static("reqwest"));

            let builder = req.build_reqwest(Request {
                method: Method::GET,
                url: "https://google.com".to_string(),
                headers: Some(headers),
                timeout: Some(Duration::new(30, 0)),
                body: None,
            }).unwrap();

            StepperResponse {
                http_requester: req,
                request_builder: Some(builder),
                response: None,
            }
        }

        fn on_success(self, res: StepperResponse) {
            todo!()
        }

        fn on_error(&self, err: Error) {
            todo!()
        }

        fn on_timeout(&self) {
            todo!()
        }

        async fn execute(&self, mut response: StepperResponse) -> Result<StepperResponse, Error> {
            let request_builder = response.request_builder.take().unwrap();
            let res = &request_builder.send().await?;

            if !res.status().is_success() {
                println!("failed: {}", res.status());
                return Ok(response);
            }

            Ok(response)
        }
    }


    use std::time::Duration;
    use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
    use reqwest::{Method, Response};
    use crate::{HttpRequester, Request};
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
        assert!(bot.steps.contains_step(step));
    }
}