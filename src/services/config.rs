//! Config service client for key-value configuration access.

use crate::context::Context;
use crate::types::*;

/// Client for the host config service.
pub struct ConfigClient<'a> {
    ctx: &'a Context,
}

impl<'a> ConfigClient<'a> {
    /// Create a new config client bound to the given context.
    pub fn new(ctx: &'a Context) -> Self {
        Self { ctx }
    }

    /// Retrieve a config value by key, returning `None` if not found.
    pub fn get(&self, key: &str) -> Option<String> {
        let mut msg = Message::new("svc.config.get", Vec::new());
        msg.set_meta("key", key);

        let result = self.ctx.send(&msg);
        if result.action == Action::Error || result.response.is_none() {
            return None;
        }
        result
            .response
            .map(|r| String::from_utf8_lossy(&r.data).into_owned())
    }

    /// Retrieve a config value, returning `default_value` if the key is absent.
    pub fn get_default(&self, key: &str, default_value: &str) -> String {
        self.get(key).unwrap_or_else(|| default_value.to_string())
    }

    /// Store a config key-value pair.
    pub fn set(&self, key: &str, value: &str) -> std::result::Result<(), WaferError> {
        let mut msg = Message::new("svc.config.set", value.as_bytes().to_vec());
        msg.set_meta("key", key);

        let result = self.ctx.send(&msg);
        if result.action == Action::Error {
            return Err(result
                .error
                .unwrap_or_else(|| WaferError::new("unknown", "config set failed")));
        }
        Ok(())
    }
}
