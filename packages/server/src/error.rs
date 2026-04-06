use thiserror::Error;

#[derive(Debug, Error)]
pub enum UpslimError {
    #[error("Config error: {0}")]
    Config(String),

    #[error("Check error: {0}")]
    Check(String),

    #[error("Alert error: {0}")]
    Alert(String),

    #[error("State error: {0}")]
    State(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

pub type Result<T> = std::result::Result<T, UpslimError>;
