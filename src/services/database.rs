//! Database service client for CRUD operations on collections.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::context::Context;
use crate::types::*;

/// A record returned from the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub id: String,
    pub data: HashMap<String, serde_json::Value>,
}

/// A paginated list of records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordList {
    pub records: Vec<Record>,
    pub total_count: i64,
    pub page: i64,
    pub page_size: i64,
}

/// Options for listing records.
#[derive(Debug, Clone, Default)]
pub struct ListOptions {
    /// Filter expression (e.g. `"status = 'active'"`).
    pub filter: String,
    /// Sort expression (e.g. `"created_at DESC"`).
    pub sort: String,
    /// Page number (1-based).
    pub page: i64,
    /// Number of records per page.
    pub page_size: i64,
}

/// Client for the host database service.
///
/// All operations are synchronous and cross the WASM boundary via
/// [`Context::send`].
pub struct DatabaseClient<'a> {
    ctx: &'a Context,
}

impl<'a> DatabaseClient<'a> {
    /// Create a new database client bound to the given context.
    pub fn new(ctx: &'a Context) -> Self {
        Self { ctx }
    }

    /// Retrieve a single record by ID from a collection.
    pub fn get(&self, collection: &str, id: &str) -> std::result::Result<Record, WaferError> {
        let mut msg = Message::new("svc.database.get", Vec::new());
        msg.set_meta("collection", collection);
        msg.set_meta("id", id);

        let result = self.ctx.send(&msg);
        self.parse_record(result)
    }

    /// List records with optional filtering, sorting, and pagination.
    pub fn list(
        &self,
        collection: &str,
        opts: &ListOptions,
    ) -> std::result::Result<RecordList, WaferError> {
        let mut msg = Message::new("svc.database.list", Vec::new());
        msg.set_meta("collection", collection);
        if !opts.filter.is_empty() {
            msg.set_meta("filter", &opts.filter);
        }
        if !opts.sort.is_empty() {
            msg.set_meta("sort", &opts.sort);
        }
        if opts.page > 0 {
            msg.set_meta("page", opts.page.to_string());
        }
        if opts.page_size > 0 {
            msg.set_meta("page_size", opts.page_size.to_string());
        }

        let result = self.ctx.send(&msg);
        self.parse_response::<RecordList>(result)
    }

    /// Insert a new record into a collection. The data is serialized as JSON
    /// in the message body.
    pub fn create(
        &self,
        collection: &str,
        data: &HashMap<String, serde_json::Value>,
    ) -> std::result::Result<Record, WaferError> {
        let body = serde_json::to_vec(data)
            .map_err(|e| WaferError::new("encode_error", e.to_string()))?;

        let mut msg = Message::new("svc.database.create", body);
        msg.set_meta("collection", collection);

        let result = self.ctx.send(&msg);
        self.parse_record(result)
    }

    /// Update an existing record by ID.
    pub fn update(
        &self,
        collection: &str,
        id: &str,
        data: &HashMap<String, serde_json::Value>,
    ) -> std::result::Result<Record, WaferError> {
        let body = serde_json::to_vec(data)
            .map_err(|e| WaferError::new("encode_error", e.to_string()))?;

        let mut msg = Message::new("svc.database.update", body);
        msg.set_meta("collection", collection);
        msg.set_meta("id", id);

        let result = self.ctx.send(&msg);
        self.parse_record(result)
    }

    /// Delete a record by ID.
    pub fn delete(&self, collection: &str, id: &str) -> std::result::Result<(), WaferError> {
        let mut msg = Message::new("svc.database.delete", Vec::new());
        msg.set_meta("collection", collection);
        msg.set_meta("id", id);

        let result = self.ctx.send(&msg);
        if result.action == Action::Error {
            return Err(result
                .error
                .unwrap_or_else(|| WaferError::new("unknown", "database delete failed")));
        }
        Ok(())
    }

    // -- internal helpers ---------------------------------------------------

    fn parse_record(&self, result: Result_) -> std::result::Result<Record, WaferError> {
        self.parse_response::<Record>(result)
    }

    fn parse_response<T: serde::de::DeserializeOwned>(
        &self,
        result: Result_,
    ) -> std::result::Result<T, WaferError> {
        if result.action == Action::Error {
            return Err(result
                .error
                .unwrap_or_else(|| WaferError::new("unknown", "database operation failed")));
        }
        let resp = result
            .response
            .ok_or_else(|| WaferError::new("no_response", "host returned no response data"))?;
        serde_json::from_slice(&resp.data)
            .map_err(|e| WaferError::new("decode_error", e.to_string()))
    }
}
