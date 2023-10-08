use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum StepError {
    Unsuccessful,
    StepNotFound(String),
    StatusCodeNotFound(i32, Vec<u16>),
}

impl fmt::Display for StepError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StepError::Unsuccessful => write!(f, "Step was unsuccessful"),
            StepError::StepNotFound(step_name) => write!(f, "Step not found: {}", step_name),
            StepError::StatusCodeNotFound(code, expected_codes) => {
                write!(
                    f,
                    "Unexpected status code {}. Expected one of: {:?}",
                    code, expected_codes
                )
            }
        }
    }
}

impl Error for StepError {}

impl From<std::io::Error> for StepError {
    fn from(_: std::io::Error) -> Self {
        StepError::Unsuccessful
    }
}
