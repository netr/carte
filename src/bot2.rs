#![allow(dead_code)]

use crate::bot::StepManager;
use crate::{Context, HttpRequester, Request, StepError, Stepable};
use std::io::Error;
use std::sync::Arc;

pub struct BotTwo {
    steps: StepManager,
    ctx: Context,
}

impl BotTwo {
    pub fn new() -> Self {
        let steps = StepManager::new();
        let ctx = Context::new();
        BotTwo { steps, ctx }
    }

    pub fn add_step(&mut self, step: impl Stepable + 'static) {
        self.steps.insert(step);
    }

    pub fn add_step_arc(&mut self, step: Arc<dyn Stepable>) {
        self.steps.insert_arc(step);
    }

    pub fn steps(self) -> StepManager {
        self.steps
    }

    // get the step by name
    fn get_step(&self, name: &str) -> Option<Arc<dyn Stepable>> {
        match self.steps.get(name) {
            Some(step) => Some(step.clone()),
            None => None,
        }
    }

    // call on_request() on the step
    // use the request to populate the request in the context
    // set current step in context
    fn update_ctx_with_request(
        &mut self,
        name: &str,
        req: Request,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut http_req: HttpRequester = HttpRequester::new();

        // set the proxy, user agent, and compression settings before we give up ownership of the request.
        let status_codes = req.status_codes().clone();
        http_req.settings.set_proxy(req.proxy());
        http_req.settings.set_user_agent(req.user_agent());
        http_req.settings.set_compression(req.is_compressed());

        self.ctx.status_codes = status_codes;

        if let Ok(builder) = http_req.build_reqwest(req.clone()) {
            self.ctx.request_builder = Some(builder);
        } else {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Unable to build request",
            )));
        }

        self.ctx.http_requester = http_req;
        self.ctx.current_step = Some(name.to_string());
        self.ctx.request = req;

        Ok(())
    }

    // start the instant timer to run the step
    // run send() on the request_builder
    // stop the instant timer
    pub async fn try_step(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let step = self.get_step(name).unwrap();
        let req = step.on_request();

        self.update_ctx_with_request(name, req)?;

        let req_builder = self.ctx.request_builder.take().unwrap();

        // Start processing the request and time it.
        let stop_watch = std::time::Instant::now();
        let res = match req_builder.send().await {
            Ok(res) => res,
            Err(err) => {
                if err.is_timeout() {
                    step.on_timeout(&mut self.ctx);

                    return Err(Self::timeout_error());
                }

                step.on_error(&mut self.ctx, StepError::ReqwestError(err.to_string()));
                return Err(Box::new(err));
            }
        };
        self.ctx
            .set_time_elapsed(stop_watch.elapsed().as_millis() as u64);

        if !self.check_status_code(res.status().as_u16()) {
            let error = StepError::StatusCodeNotFound(
                res.status().as_u16() as i32,
                self.ctx.status_codes.clone().unwrap_or_else(Vec::new),
            );

            step.on_error(&mut self.ctx, error.clone());
            return Err(Box::new(error));
        }

        self.ctx.response = Some(res);
        step.on_success(&mut self.ctx);

        Ok(())
    }

    fn timeout_error() -> Box<Error> {
        Box::new(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "Request timed out",
        ))
    }

    fn check_status_code(&self, status_code: u16) -> bool {
        match &self.ctx.status_codes {
            Some(codes) => {
                if codes.is_empty() {
                    return 300 > status_code && status_code >= 200;
                }
                codes.contains(&status_code)
            }
            None => 300 > status_code && status_code >= 200,
        }
    }

    // if the request is successful, call on_success() on the step
    // if the request is unsuccessful, call on_error() on the step
    // stop timers and store the time elapsed in the context
}

#[cfg(test)]
mod tests {
    use crate::bot2::BotTwo;
    use crate::{Context, Request, StepError, Stepable};
    use async_trait::async_trait;
    use reqwest::Method;
    use std::sync::Arc;

    #[derive(Clone, Copy)]
    struct RobotsTxt;

    #[async_trait]
    impl Stepable for RobotsTxt {
        fn name(&self) -> String {
            String::from("RobotsTxt")
        }

        fn on_request(&self) -> Request {
            Request::new(Method::GET, "https://google.com".to_string())
        }

        fn on_success(&self, _ctx: &mut Context) {
            eprintln!("Success!")
        }

        fn on_error(&self, _ctx: &mut Context, _err: StepError) {
            todo!()
        }

        fn on_timeout(&self, _ctx: &mut Context) {
            todo!()
        }
    }

    #[test]
    fn it_should_add_step() {
        let mut bot = BotTwo::new();
        bot.add_step(RobotsTxt);

        assert_eq!(bot.steps().len(), 1);
    }

    #[test]
    fn it_should_add_step_arc() {
        let mut bot = BotTwo::new();
        let step = Arc::new(RobotsTxt);
        bot.add_step_arc(step);

        assert_eq!(bot.steps().len(), 1);
    }

    #[test]
    fn it_should_get_step() {
        let mut bot = BotTwo::new();
        bot.add_step(RobotsTxt);

        let step = bot.get_step("RobotsTxt").unwrap();
        assert_eq!(step.name(), "RobotsTxt");
    }

    #[test]
    fn it_should_call_on_request() {
        let mut bot = BotTwo::new();
        bot.add_step(RobotsTxt);

        let step = bot.get_step("RobotsTxt").unwrap();
        let req = step.on_request();
        assert_eq!(req.method(), "GET");
    }

    #[test]
    fn it_should_update_context_from_request() {
        let mut bot = BotTwo::new();
        bot.add_step(RobotsTxt);

        let step = bot.get_step("RobotsTxt").unwrap();
        let req = step.on_request();

        match bot.update_ctx_with_request("RobotsTxt", req) {
            Ok(_) => {
                assert_eq!(bot.ctx.request.method(), "GET");
                assert_eq!(bot.ctx.request.url(), "https://google.com");
            }
            Err(e) => {
                println!("Error: {}", e);
                assert!(false);
            }
        }
    }

    #[tokio::test]
    async fn it_should_try_step() {
        let mut bot = BotTwo::new();
        bot.add_step(RobotsTxt);

        match bot.try_step("RobotsTxt").await {
            Ok(_) => {
                assert_eq!(bot.ctx.request.method(), "GET");
                assert_eq!(bot.ctx.request.url(), "https://google.com");
            }
            Err(e) => {
                println!("Error: {}", e);
                assert!(false);
            }
        }
    }
    #[test]
    fn check_status_codes_should_return_true_if_status_code_matches() {
        let mut bot = BotTwo::new();
        bot.ctx.status_codes = Some(vec![200]);

        assert!(bot.check_status_code(200));
    }

    #[test]
    fn check_status_codes_should_return_false_if_status_code_does_not_match() {
        let mut bot = BotTwo::new();
        bot.ctx.status_codes = Some(vec![200]);

        assert!(!bot.check_status_code(404));
    }

    #[test]
    fn check_status_codes_should_use_default_status_codes_if_200_to_300_if_status_cares_are_empty()
    {
        let mut bot = BotTwo::new();
        bot.ctx.status_codes = Some(vec![]);

        assert!(bot.check_status_code(200));
    }

    #[test]
    fn check_status_codes_should_use_default_status_codes_if_200_to_300_if_no_status_codes_are_set()
    {
        let bot = BotTwo::new();

        assert!(bot.check_status_code(200));
        assert!(!bot.check_status_code(404));
    }
}
