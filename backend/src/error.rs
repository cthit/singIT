use std::fmt::{Debug, Display};

use actix_web::ResponseError;
use diesel_async::pooled_connection::deadpool::PoolError;

pub type Result<R> = core::result::Result<R, Error>;

pub struct Error {
    error: eyre::Report,
}

impl From<eyre::Report> for Error {
    fn from(error: eyre::Report) -> Self {
        Self { error }
    }
}

impl From<PoolError> for Error {
    fn from(error: PoolError) -> Self {
        let error = eyre::Report::new(error).wrap_err("Failed to get database connection");
        Self { error }
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.error, f)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.error, f)
    }
}

impl ResponseError for Error {
    fn status_code(&self) -> reqwest::StatusCode {
        reqwest::StatusCode::INTERNAL_SERVER_ERROR
    }

    fn error_response(&self) -> actix_web::HttpResponse<actix_web::body::BoxBody> {
        log::error!("{:?}", self.error);
        actix_web::HttpResponse::new(self.status_code())
    }
}
