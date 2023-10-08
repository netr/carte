use std::error::Error;
use std::fmt;

#[derive(Debug, Clone)]
pub enum StepError {
    ReqwestError(String),
    StepNotFound(String),
    StatusCodeNotFound(i32, Vec<u16>),
}

impl fmt::Display for StepError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StepError::StepNotFound(step_name) => write!(f, "Step not found: {}", step_name),
            StepError::ReqwestError(err) => write!(f, "Reqwest error: {}", err),
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
