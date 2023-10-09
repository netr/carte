# mimicr

![mimicr github actions workflow](https://github.com/netr/mimicr/actions/workflows/mimicr.yml/badge.svg)

Using this project as a learning experience for Rust.

Rust library for working with HTTP requests to simulate a browser. The core concept of the library is to allow you to
construct multiple-step schemas to e2e test or pen test. It is based
on [reqwest](https://docs.rs/reqwest/latest/reqwest/index.html)
and [tokio](https://docs.rs/tokio/latest/tokio/index.html) for async.

By utilizing `reqwest` and the internal `cookie stores`, you can simulate a browser session. From creating accounts,
logging
in, maintaining sessions, and more.

Work in progress, subject to change. Not ready for production use.

## Todo / Ideas

- [x] Basic functionality for a simple recursive bot with multiple steps
- [x] Create a macro for `hdr!`(r#"Content-type: application/json#Accept: application/json") to make it easier to create
  headers
- [x] Set up a `Request` builder pattern to make it less verbose to create requests
- [X] Move `Context` to its own module and remove `pub` from `Context` struct
- [ ] Allow for `shared state` such as databases or other resources
- [ ] Allow for skipping steps during `on_request()`
- [ ] Create a more robust `example using an api` that requires authentication, session management, etc.
- [ ] More robust timeout and pause functionality [Timeout, AfterSuccess, AfterError, etc]
- [ ] Move `Context` to a `ContextBuilder` pattern to make it less verbose to create contexts?
- [ ] Create a `curl command parser` to transform curl commands to mimicr steps

## Usage for a 2 step bot

```rust
use mimicr::{Bot, Context, Request, StepError, Stepable};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

// TODO: incorporate handle_recursive into Bot
async fn handle_recursive(mut bot: Bot, step: String) -> Result<(), reqwest::Error> {
    match bot.handle_step(step).await {
        Ok(res) => {
            if let Some(next_step) = res.next_step {
                let fut: Pin<Box<dyn Future<Output=Result<(), reqwest::Error>>>> =
                    Box::pin(handle_recursive(bot, next_step));
                fut.await?;
            }
        }
        Err(err) => panic!("{}", err),
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let mut bot: Bot = Bot::new();
    bot.steps
        .insert_many(vec![Box::new(Google {}), Box::new(Facebook {})]);
    handle_recursive(bot, Steps::Google.to_string()).await
}

enum Steps {
    Google,
    Facebook,
}

// Display
impl fmt::Display for Steps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Steps::Google => String::from("Google"),
                Steps::Facebook => String::from("Facebook"),
            }
        )
    }
}

#[derive(Clone, Copy)]
struct Google;

#[async_trait]
impl Stepable for Google {
    fn name(&self) -> String {
        Steps::Google.to_string()
    }

    fn on_request(&mut self) -> Request {
        let headers = hdr!(
            r#"User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/116.0.0.0 Safari/537.36
            Accept: */*"#
        );

        Request::new(Method::GET, "https://google.com".to_string())
            .with_headers(headers)
            .with_timeout(Duration::new(60, 0))
            .with_status_codes(vec![200])
            .build();
    }

    fn on_success(&self, ctx: &mut Context) {
        let res = ctx.response.as_ref().unwrap();
        println!(
            "Successfully fetched: {} in {} ms",
            ctx.current_step.as_ref().unwrap(),
            ctx.time_elapsed,
        );

        ctx.set_next_step(Steps::Facebook.to_string());
    }

    fn on_error(&self, ctx: &mut Context, err: StepError) {
        todo!()
    }

    fn on_timeout(&self, ctx: &mut Context) {
        todo!()
    }
}


#[derive(Clone, Copy)]
struct Facebook;

#[async_trait]
impl Stepable for Facebook {
    fn name(&self) -> String {
        Steps::Facebook.to_string()
    }

    fn on_request(&mut self) -> Request {
        Request::new(Method::GET, "https://facebook.com".to_string())
            .with_headers(hdr!("Accept-Encoding: gzip, deflate, br"))
            .with_timeout(Duration::new(60, 0))
            .with_status_codes(vec![200])
            .build();
    }

    fn on_success(&self, ctx: &mut Context) {
        let res = ctx.response.as_ref().unwrap();
        println!(
            "Successfully fetched: {} in {} ms",
            ctx.current_step.as_ref().unwrap(),
            ctx.time_elapsed,
        );
        // without setting a next_step, the bot will stop
    }

    fn on_error(&self, ctx: &mut Context, err: StepError) {
        todo!()
    }

    fn on_timeout(&self, ctx: &mut Context) {
        todo!()
    }
}
```