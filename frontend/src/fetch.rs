use std::io::Cursor;

use gloo_net::http::{Request, Response};
use serde::de::DeserializeOwned;
use wasm_bindgen::{JsError, JsValue};

const HTTP_ACCEPT: &str = "text/csv, application/json;q=0.9";

#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    /// The request returned a non-2XX status code.
    #[error("server responded with {code} {text}")]
    Status { code: u16, text: String },

    /// The response contained an unrecognized or missing content type.
    #[error("unknown content type {0:?}")]
    UnknownContentType(Option<String>),

    /// Another error occured.
    #[error("{0}")]
    Other(#[from] gloo_net::Error),

    #[error("error deserializing csv: {0}")]
    Csv(#[from] csv::Error),
}

impl From<FetchError> for JsValue {
    fn from(e: FetchError) -> Self {
        JsError::new(&e.to_string()).into()
    }
}

/// Perform a GET request.
pub async fn fetch(url: impl AsRef<str>) -> Result<Response, FetchError> {
    let response = Request::get(url.as_ref())
        .header("accept", HTTP_ACCEPT)
        .send()
        .await?;

    if !response.ok() {
        return Err(FetchError::Status {
            code: response.status(),
            text: response.status_text(),
        });
    }

    Ok(response)
}

/// Perform a GET request and try to deserialize the response as a `Vec<T>`.
pub async fn fetch_list_of<T: DeserializeOwned>(
    url: impl AsRef<str>,
) -> Result<Vec<T>, FetchError> {
    let response = fetch(url.as_ref()).await?;
    let headers = response.headers();
    let content_type = headers.get("Content-Type").map(|s| s.to_lowercase());

    match content_type.as_deref() {
        Some("text/csv") => {
            let text = response.text().await?;
            let reader = csv::Reader::from_reader(Cursor::new(text)).into_deserialize();
            let list = reader
                .map(|r| r.map_err(FetchError::from))
                .collect::<Result<_, _>>()?;
            Ok(list)
        }
        Some("application/json") => Ok(response.json().await?),
        _ => Err(FetchError::UnknownContentType(content_type)),
    }
}
