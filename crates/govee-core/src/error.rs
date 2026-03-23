use thiserror::Error;

#[derive(Debug, Error)]
pub enum GoveeError {
    #[error("network error: {0}")]
    Network(#[from] std::io::Error),

    #[cfg(feature = "http-api")]
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("device not found: {0}")]
    DeviceNotFound(String),

    #[error("invalid brightness {value}: must be 1–100")]
    InvalidBrightness { value: u8 },

    #[error("invalid color temperature {value}K: must be 2000–9000")]
    InvalidColorTemp { value: u16 },

    #[error("API error ({status}): {message}")]
    ApiError { status: u16, message: String },

    #[error("timeout waiting for device response")]
    Timeout,
}

pub type Result<T> = std::result::Result<T, GoveeError>;
