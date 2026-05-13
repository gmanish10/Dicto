pub mod paste;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum InjectError {
    #[error("clipboard error: {0}")]
    Clipboard(String),
    #[error("event injection error: {0}")]
    Event(String),
    #[error("secure input mode is active — text copied to clipboard instead")]
    SecureInputActive,
}

pub trait Injector: Send + Sync {
    fn inject(&self, text: &str) -> Result<(), InjectError>;
}
