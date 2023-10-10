#![allow(dead_code)]

use crate::context::Context;
use crate::steps::StepManager;
use crate::{StepError, Stepable};
use std::io::Error;
use std::sync::Arc;

pub struct Worker {
    steps: StepManager,
    pub ctx: Context,
}

impl Default for Worker {
    fn default() -> Self {
        Worker::new()
    }
}

impl Worker {
    pub fn new() -> Self {
        let steps = StepManager::new();
        let ctx = Context::new();
        Worker { steps, ctx }
    }

    pub fn add_step(&mut self, step: impl Stepable + 'static) {
        self.steps.insert(step);
    }

    pub fn add_many_steps(&mut self, steps: Vec<Arc<dyn Stepable>>) {
        self.steps.insert_many(steps);
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

    // start the instant timer to run the step
    // run send() on the request_builder
    // stop the instant timer
    pub async fn try_step(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let step = self.get_step(name).unwrap();
        let req = step.on_request();

        if req.get_skip_to_step().is_some() {
            self.ctx
                .set_next_step(req.get_skip_to_step().unwrap().clone());
            return Ok(());
        }

        self.ctx.update_from_request(req)?;
        self.ctx.set_current_step(name.to_string());

        let req_builder = self.ctx.get_request_builder().unwrap();

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
                self.ctx.get_status_codes().unwrap_or_default(),
            );

            step.on_error(&mut self.ctx, error.clone());
            return Err(Box::new(error));
        }

        let body = match res.bytes().await {
            Ok(body) => body,
            Err(err) => {
                step.on_error(&mut self.ctx, StepError::ReqwestError(err.to_string()));
                return Err(Box::new(err));
            }
        };

        self.ctx.set_response_body(body);

        // clear the next step since the context is being reused, this fixes the infinite loop bug
        self.ctx.clear_next_step();
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
        match &self.ctx.get_status_codes() {
            Some(codes) => {
                if codes.is_empty() {
                    return (200..300).contains(&status_code);
                }
                codes.contains(&status_code)
            }
            None => (200..300).contains(&status_code),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::worker::Worker;
    use crate::{Context, Request, StepError, Stepable};
    use async_trait::async_trait;
    use reqwest::Method;
    use std::sync::Arc;

    #[derive(Clone, Copy)]
    struct RobotsTxt;

    const ROBOTS_TXT: &str = "RobotsTxt";
    const SKIPPABLE_STEP: &str = "SkippableStep";

    #[async_trait]
    impl Stepable for RobotsTxt {
        fn name(&self) -> String {
            String::from(ROBOTS_TXT)
        }

        fn on_request(&self) -> Request {
            Request::new(Method::GET, "https://google.com".to_string())
        }

        fn on_success(&self, ctx: &mut Context) {
            eprintln!(
                "Successfully fetched: {} in {}\n\nURL: {}\nBody: {:?}",
                ctx.get_current_step().unwrap(),
                ctx.get_time_elapsed_as_string(),
                ctx.get_url(),
                ctx.body_text().unwrap(),
            );
        }

        fn on_error(&self, _ctx: &mut Context, _err: StepError) {
            todo!()
        }

        fn on_timeout(&self, _ctx: &mut Context) {
            todo!()
        }
    }

    #[derive(Clone, Copy)]
    struct SkippableStep;

    #[async_trait]
    impl Stepable for SkippableStep {
        fn name(&self) -> String {
            String::from(SKIPPABLE_STEP)
        }

        fn on_request(&self) -> Request {
            Request::new(Method::GET, "https://google.com".to_string())
                .skip_to(Some(ROBOTS_TXT.to_string()))
        }

        fn on_success(&self, _ctx: &mut Context) {
            todo!("This step should never be called")
        }

        fn on_error(&self, _ctx: &mut Context, _err: StepError) {
            todo!("This step should never be called")
        }

        fn on_timeout(&self, _ctx: &mut Context) {
            todo!("This step should never be called")
        }
    }

    #[test]
    fn it_should_add_step() {
        let mut worker = Worker::new();
        worker.add_step(RobotsTxt);

        assert_eq!(worker.steps().len(), 1);
    }

    #[test]
    fn it_should_add_step_arc() {
        let mut worker = Worker::new();
        let step = Arc::new(RobotsTxt);
        worker.add_step_arc(step);

        assert_eq!(worker.steps().len(), 1);
    }

    #[test]
    fn it_should_get_step() {
        let mut worker = Worker::new();
        worker.add_step(RobotsTxt);

        let step = worker.get_step(ROBOTS_TXT).unwrap();
        assert_eq!(step.name(), ROBOTS_TXT);
    }

    #[test]
    fn it_should_call_on_request() {
        let mut worker = Worker::new();
        worker.add_step(RobotsTxt);

        let step = worker.get_step(ROBOTS_TXT).unwrap();
        let req = step.on_request();
        assert_eq!(req.method(), "GET");
    }

    #[test]
    fn it_should_update_context_from_request() {
        let mut worker = Worker::new();
        worker.add_step(RobotsTxt);

        let step = worker.get_step(ROBOTS_TXT).unwrap();
        let req = step.on_request();

        match worker.ctx.update_from_request(req) {
            Ok(_) => {
                assert_eq!(worker.ctx.get_method(), "GET");
                assert_eq!(worker.ctx.get_url(), "https://google.com");
            }
            Err(e) => {
                println!("Error: {}", e);
                unreachable!()
            }
        }
    }

    #[test]
    fn it_should_update_current_step() {
        let mut worker = Worker::new();
        worker.ctx.set_current_step(ROBOTS_TXT.to_string());
        assert_eq!(worker.ctx.get_current_step().unwrap(), ROBOTS_TXT);
    }

    /// This actually goes to https://google.com and fetches the page.
    /// It's ignored because it can break if the internet is down.
    /// It's here for testing purposes only.
    #[tokio::test]
    #[ignore]
    async fn it_should_try_step() {
        let mut worker = Worker::new();
        worker.add_step(RobotsTxt);

        match worker.try_step(ROBOTS_TXT).await {
            Ok(_) => {
                assert_eq!(worker.ctx.get_method(), "GET");
                assert_eq!(worker.ctx.get_url(), "https://google.com");
            }
            Err(e) => {
                println!("Error: {}", e);
                assert!(false);
            }
        }
    }

    #[test]
    fn check_status_codes_should_return_true_if_status_code_matches() {
        let mut worker = Worker::new();
        worker.ctx.set_status_codes(vec![200]);

        assert!(worker.check_status_code(200));
    }

    #[test]
    fn check_status_codes_should_return_false_if_status_code_does_not_match() {
        let mut worker = Worker::new();
        worker.ctx.set_status_codes(vec![200]);

        assert!(!worker.check_status_code(404));
    }

    #[test]
    fn check_status_codes_should_use_default_status_codes_if_200_to_300_if_status_cares_are_empty()
    {
        let mut worker = Worker::new();
        worker.ctx.set_status_codes(vec![]);

        assert!(worker.check_status_code(200));
    }

    #[test]
    fn check_status_codes_should_use_default_status_codes_if_200_to_300_if_no_status_codes_are_set()
    {
        let worker = Worker::new();

        assert!(worker.check_status_code(200));
        assert!(!worker.check_status_code(404));
    }

    #[test]
    fn it_should_skip_to_step() {
        let mut worker = Worker::new();
        worker.add_step(SkippableStep);

        let step = worker.get_step(SKIPPABLE_STEP).unwrap();
        let req = step.on_request();

        assert_eq!(req.get_skip_to_step().unwrap(), ROBOTS_TXT);
    }
}
