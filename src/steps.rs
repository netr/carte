use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

use crate::context::Context;
use crate::{Request, StepError};

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

impl Default for StepManager {
    fn default() -> Self {
        StepManager::new()
    }
}

#[allow(dead_code)]
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
            self.insert_arc(step);
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

    #[tokio::test]
    async fn step_should_call_on_request_as_expected() {
        let step = RobotsTxt {};
        let req = step.on_request();

        assert_eq!(req.method(), Method::GET);
        assert_eq!(req.status_codes(), Some(vec![200]));
    }
}
