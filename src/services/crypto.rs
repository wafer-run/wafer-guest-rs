//! Crypto service client for hashing, token signing, and verification.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::context::Context;
use crate::types::*;

/// A signed token response from the host.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub token: String,
}

/// Verified token claims returned by the host.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaims {
    pub claims: HashMap<String, serde_json::Value>,
}

/// Client for the host crypto service.
pub struct CryptoClient<'a> {
    ctx: &'a Context,
}

impl<'a> CryptoClient<'a> {
    /// Create a new crypto client bound to the given context.
    pub fn new(ctx: &'a Context) -> Self {
        Self { ctx }
    }

    /// Produce a one-way hash of the given password.
    pub fn hash(&self, password: &str) -> std::result::Result<String, WaferError> {
        let msg = Message::new("svc.crypto.hash", password.as_bytes().to_vec());

        let result = self.ctx.send(&msg);
        if result.action == Action::Error {
            return Err(result
                .error
                .unwrap_or_else(|| WaferError::new("unknown", "crypto hash failed")));
        }

        let resp = result
            .response
            .ok_or_else(|| WaferError::new("no_response", "host returned no response data"))?;
        Ok(String::from_utf8_lossy(&resp.data).into_owned())
    }

    /// Compare a password against a previously-computed hash.
    pub fn compare_hash(
        &self,
        password: &str,
        hash: &str,
    ) -> std::result::Result<(), WaferError> {
        let mut msg = Message::new("svc.crypto.compare_hash", password.as_bytes().to_vec());
        msg.set_meta("hash", hash);

        let result = self.ctx.send(&msg);
        if result.action == Action::Error {
            return Err(result
                .error
                .unwrap_or_else(|| WaferError::new("unknown", "crypto compare_hash failed")));
        }
        Ok(())
    }

    /// Create a signed token from claims with the given expiry in seconds.
    pub fn sign(
        &self,
        claims: &HashMap<String, serde_json::Value>,
        expiry_secs: u64,
    ) -> std::result::Result<String, WaferError> {
        let body = serde_json::to_vec(claims)
            .map_err(|e| WaferError::new("encode_error", e.to_string()))?;

        let mut msg = Message::new("svc.crypto.sign", body);
        msg.set_meta("expiry", expiry_secs.to_string());

        let result = self.ctx.send(&msg);
        if result.action == Action::Error {
            return Err(result
                .error
                .unwrap_or_else(|| WaferError::new("unknown", "crypto sign failed")));
        }

        let resp = result
            .response
            .ok_or_else(|| WaferError::new("no_response", "host returned no response data"))?;
        Ok(String::from_utf8_lossy(&resp.data).into_owned())
    }

    /// Verify a token and return its claims.
    pub fn verify(
        &self,
        token: &str,
    ) -> std::result::Result<HashMap<String, serde_json::Value>, WaferError> {
        let msg = Message::new("svc.crypto.verify", token.as_bytes().to_vec());

        let result = self.ctx.send(&msg);
        if result.action == Action::Error {
            return Err(result
                .error
                .unwrap_or_else(|| WaferError::new("unknown", "crypto verify failed")));
        }

        let resp = result
            .response
            .ok_or_else(|| WaferError::new("no_response", "host returned no response data"))?;
        serde_json::from_slice(&resp.data)
            .map_err(|e| WaferError::new("decode_error", e.to_string()))
    }
}
