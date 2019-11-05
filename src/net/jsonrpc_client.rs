use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};

use serde::{Deserialize, Serialize};
use serde_json::{json, value::from_value, Value};

#[derive(Debug)]
pub enum HeaderError {
    /// Received no header
    WorkQueueFull,
    /// Received text/html; charset=ISO-8859-1
    IncorrectCredentials,
    /// Received none of the above nor application/json
    Unexpected(http::HeaderValue),
}

#[derive(Debug)]
pub enum ClientError {
    /// Json decoding error.
    Json(serde_json::Error),
    /// Client error
    Client(reqwest::Error),
    /// Rpc error,
    Rpc(serde_json::Value),
    /// Response has neither error nor result.
    NoErrorOrResult,
    /// Response to a request did not have the expected nonce
    NonceMismatch,
    /// Received unexpected header
    Header(HeaderError),
}

impl From<serde_json::Error> for ClientError {
    fn from(e: serde_json::Error) -> Self {
        ClientError::Json(e)
    }
}

impl From<reqwest::Error> for ClientError {
    fn from(e: reqwest::Error) -> Self {
        ClientError::Client(e)
    }
}

impl From<HeaderError> for ClientError {
    fn from(e: HeaderError) -> Self {
        ClientError::Header(e)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Request {
    pub method: String,
    pub params: Vec<Value>,
    pub id: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Response {
    pub result: Option<Value>,
    pub error: Option<Value>,
    pub id: Value,
}

impl Response {
    /// Extract the result from a response
    pub fn into_result<T>(self) -> Result<T, ClientError>
    where
        for<'de> T: serde::Deserialize<'de>,
    {
        if let Some(ref e) = self.error {
            return Err(ClientError::Rpc(e.clone()));
        }
        match self.result {
            Some(res) => from_value(res).map_err(ClientError::Json),
            None => Err(ClientError::NoErrorOrResult),
        }
    }
}

/// JSONRPC Client containing handle, counter and client struct
pub struct JsonClient {
    endpoint: String,
    username: String,
    password: String,
    client: reqwest::Client,
    nonce: AtomicUsize,
}

impl JsonClient {
    /// Create new JSONRPC client
    pub fn new(endpoint: String, username: String, password: String) -> JsonClient {
        JsonClient {
            endpoint,
            username,
            password,
            client: reqwest::Client::new(),
            nonce: AtomicUsize::new(0),
        }
    }

    fn check_headers(response: &reqwest::Response) -> Result<(), HeaderError> {
        match response.headers().get("content-type") {
            None => return Err(HeaderError::WorkQueueFull),
            Some(some) => {
                let json_header = http::header::HeaderValue::from_str("application/json").unwrap();
                if some == json_header {
                    return Ok(());
                }
                let html_header =
                    http::header::HeaderValue::from_str("text/html; charset=ISO-8859-1").unwrap();
                if some == &html_header {
                    return Err(HeaderError::IncorrectCredentials);
                }

                return Err(HeaderError::Unexpected(some.clone()));
            }
        }
    }

    /// Sends a request to a async client
    pub async fn send_request(&self, request: &Request) -> Result<Response, ClientError> {
        let mut request_builder = self.client.post(&self.endpoint);

        request_builder =
            request_builder.basic_auth(self.username.clone(), Some(self.password.clone()));
        let request_id = request.id.clone();
        let response = request_builder
            .json(request)
            .send()
            .await
            .map_err(ClientError::from)?;

        // Check headers
        JsonClient::check_headers(&response)?;

        // Parse response
        let json_response: Response = response.json().await?;

        // Check IDs match
        if json_response.id != request_id {
            return Err(ClientError::NonceMismatch);
        }

        Ok(json_response)
    }

    /// Builds a request
    pub fn build_request(&self, method: String, params: Vec<Value>) -> Request {
        self.nonce.fetch_add(1, SeqCst);
        Request {
            method,
            params,
            id: json!(self.nonce.load(SeqCst)),
        }
    }
}
