use thiserror::Error;

use crate::model::ErrorModel;

#[derive(Error, Debug)]
pub enum Error {
    #[error("HTTP Request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("Serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("API Error: {status} - {message}")]
    Api { status: u16, message: String },
    #[error("API Error: {status} - {title}")]
    ApiProblem {
        status: u16,
        title: String,
        detail: Option<String>,
        error: Box<ErrorModel>,
    },
    #[error("Authentication failed")]
    Unauthorized,
    #[error("Token refresh failed")]
    RefreshFailed,
    #[error("Invalid URL: {0}")]
    Url(#[from] url::ParseError),
    #[error("Invalid URL path: {0}")]
    InvalidPath(String),
}

pub type Result<T> = std::result::Result<T, Error>;
