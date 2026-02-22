//! Logger service client for structured logging.

use serde::Serialize;
use std::collections::HashMap;

use crate::context::Context;
use crate::types::Message;

/// A structured log entry sent to the host.
#[derive(Debug, Serialize)]
struct LogEntry {
    message: String,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    fields: HashMap<String, serde_json::Value>,
}

/// Client for the host logger service.
pub struct LoggerClient<'a> {
    ctx: &'a Context,
}

impl<'a> LoggerClient<'a> {
    /// Create a new logger client bound to the given context.
    pub fn new(ctx: &'a Context) -> Self {
        Self { ctx }
    }

    /// Log a message at the DEBUG level.
    pub fn debug(&self, message: &str) {
        self.log("debug", message, &HashMap::new());
    }

    /// Log a message at the DEBUG level with structured fields.
    pub fn debug_with(&self, message: &str, fields: HashMap<String, serde_json::Value>) {
        self.log("debug", message, &fields);
    }

    /// Log a message at the INFO level.
    pub fn info(&self, message: &str) {
        self.log("info", message, &HashMap::new());
    }

    /// Log a message at the INFO level with structured fields.
    pub fn info_with(&self, message: &str, fields: HashMap<String, serde_json::Value>) {
        self.log("info", message, &fields);
    }

    /// Log a message at the WARN level.
    pub fn warn(&self, message: &str) {
        self.log("warn", message, &HashMap::new());
    }

    /// Log a message at the WARN level with structured fields.
    pub fn warn_with(&self, message: &str, fields: HashMap<String, serde_json::Value>) {
        self.log("warn", message, &fields);
    }

    /// Log a message at the ERROR level.
    pub fn error(&self, message: &str) {
        self.log("error", message, &HashMap::new());
    }

    /// Log a message at the ERROR level with structured fields.
    pub fn error_with(&self, message: &str, fields: HashMap<String, serde_json::Value>) {
        self.log("error", message, &fields);
    }

    // -- internal -----------------------------------------------------------

    fn log(&self, level: &str, message: &str, fields: &HashMap<String, serde_json::Value>) {
        let entry = LogEntry {
            message: message.to_string(),
            fields: fields.clone(),
        };

        let body = serde_json::to_vec(&entry).unwrap_or_else(|_| message.as_bytes().to_vec());

        let mut msg = Message::new(
            format!("svc.logger.{}", level),
            body,
        );
        msg.set_meta("level", level);

        // Fire-and-forget; logging should not fail the block.
        let _ = self.ctx.send(&msg);
    }
}
