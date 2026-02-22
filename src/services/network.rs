//! Network service client for outbound HTTP requests.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::context::Context;
use crate::types::*;

/// An outbound network request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub method: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub headers: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<Vec<u8>>,
}

/// An outbound network response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub status_code: u16,
    #[serde(default)]
    pub headers: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub body: Vec<u8>,
}

/// Client for the host network service.
pub struct NetworkClient<'a> {
    ctx: &'a Context,
}

impl<'a> NetworkClient<'a> {
    /// Create a new network client bound to the given context.
    pub fn new(ctx: &'a Context) -> Self {
        Self { ctx }
    }

    /// Execute an outbound HTTP request.
    pub fn do_request(&self, req: &Request) -> std::result::Result<Response, WaferError> {
        let body = serde_json::to_vec(req)
            .map_err(|e| WaferError::new("encode_error", e.to_string()))?;

        let msg = Message::new("svc.network.do", body);

        let result = self.ctx.send(&msg);
        if result.action == Action::Error {
            return Err(result
                .error
                .unwrap_or_else(|| WaferError::new("unknown", "network request failed")));
        }

        let resp = result
            .response
            .ok_or_else(|| WaferError::new("no_response", "host returned no response data"))?;
        serde_json::from_slice(&resp.data)
            .map_err(|e| WaferError::new("decode_error", e.to_string()))
    }

    /// Convenience: perform a GET request to the given URL.
    pub fn get(&self, url: &str) -> std::result::Result<Response, WaferError> {
        self.do_request(&Request {
            method: "GET".to_string(),
            url: url.to_string(),
            headers: HashMap::new(),
            body: None,
        })
    }

    /// Convenience: perform a POST request with a JSON body.
    pub fn post_json<T: Serialize>(
        &self,
        url: &str,
        body: &T,
    ) -> std::result::Result<Response, WaferError> {
        let data = serde_json::to_vec(body)
            .map_err(|e| WaferError::new("encode_error", e.to_string()))?;

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        self.do_request(&Request {
            method: "POST".to_string(),
            url: url.to_string(),
            headers,
            body: Some(data),
        })
    }
}
