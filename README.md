# Carte

Using this project as a learning experience for Rust.

Rust library for working with HTTP requests to simulate a browser. The core concept of the library is to allow you to
construct multiple-step schemas to e2e test or pen test. It is based
on [reqwest](https://docs.rs/reqwest/latest/reqwest/index.html)
and [tokio](https://docs.rs/tokio/latest/tokio/index.html) for async.

By utilizing `reqwest` and the internal `cookie stores`, you can simulate a browser session. From creating accounts,
logging
in, maintaining sessions, and more.

Work in progress, subject to change. Not ready for production use.

## Usage

```rust
use carte::{Bot, Context, Request, StepError, Stepable};

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let mut bot: Bot = Bot::new();
    bot.steps.insert_many(vec![Box::new(RobotsTxt {})]);

    match bot.handle_step(Steps::RobotsTxt.to_string()).await {
        Ok(res) => {
            eprintln!("Response: {:?}", res.response.unwrap().version());
            eprintln!("Next step: {:?}", res.next_step.unwrap());
            eprintln!("Time elapsed: {} ms", res.time_elapsed);
        }
        Err(err) => panic!("{}", err),
    }
    Ok(())
}

enum Steps {
    RobotsTxt,
}

// Display
impl fmt::Display for Steps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Steps::RobotsTxt => String::from("RobotsTxt"),
            }
        )
    }
}

#[derive(Clone, Copy)]
struct RobotsTxt;

#[async_trait]
impl Stepable for RobotsTxt {
    fn name(&self) -> String {
        Steps::RobotsTxt.to_string()
    }

    fn on_request(&mut self) -> Request {
        Request {
            method: Method::GET,
            url: "https://google.com".to_string(),
            headers: None,
            timeout: Some(Duration::new(30, 0)),
            body: None,
            status_codes: Some(vec![200]),
        }
    }

    fn on_success(&self, ctx: &mut Context) {
        let res = ctx.response.as_ref().unwrap();
        ctx.set_next_step(Steps::RobotsTxt.to_string());
    }

    fn on_error(&self, err: StepError) {
        todo!()
    }

    fn on_timeout(&self) {
        todo!()
    }
}
```