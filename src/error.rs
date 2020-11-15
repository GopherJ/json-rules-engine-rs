use reqwest::{header::InvalidHeaderValue, Error as ReqwestError};
use serde_json::error::Error as SerializeJsonError;
use thiserror::Error as ThisError;

#[cfg(feature = "email")]
use sendgrid::error::SendgridError;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("Reqwest Error: `{0:?}`")]
    ReqwestError(#[from] ReqwestError),
    #[error("Reqwest Invalid Header Error: `{0:?}`")]
    ReqwestInvalidHeaderError(#[from] InvalidHeaderValue),
    #[error("Serialize Json Error: `{0:?}`")]
    SerializeJsonError(#[from] SerializeJsonError),
    #[cfg(feature = "email")]
    #[error("Send grid error: `{0:?}`")]
    SendgridError(#[from] SendgridError),
}

pub type Result<T> = std::result::Result<T, Error>;
