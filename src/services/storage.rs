//! Storage service client for object/file storage operations.

use serde::{Deserialize, Serialize};

use crate::context::Context;
use crate::types::*;

/// Metadata about a stored object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectInfo {
    pub key: String,
    pub size: i64,
    pub content_type: String,
}

/// A stored object: its content bytes together with metadata.
#[derive(Debug, Clone)]
pub struct Object {
    pub data: Vec<u8>,
    pub info: ObjectInfo,
}

/// Client for the host storage service.
pub struct StorageClient<'a> {
    ctx: &'a Context,
}

impl<'a> StorageClient<'a> {
    /// Create a new storage client bound to the given context.
    pub fn new(ctx: &'a Context) -> Self {
        Self { ctx }
    }

    /// Store an object in the given bucket under the given key.
    pub fn put(
        &self,
        bucket: &str,
        key: &str,
        data: &[u8],
        content_type: &str,
    ) -> std::result::Result<(), WaferError> {
        let mut msg = Message::new("svc.storage.put", data.to_vec());
        msg.set_meta("bucket", bucket);
        msg.set_meta("key", key);
        if !content_type.is_empty() {
            msg.set_meta("content_type", content_type);
        }

        let result = self.ctx.send(&msg);
        if result.action == Action::Error {
            return Err(result
                .error
                .unwrap_or_else(|| WaferError::new("unknown", "storage put failed")));
        }
        Ok(())
    }

    /// Retrieve an object from the given bucket and key.
    pub fn get(&self, bucket: &str, key: &str) -> std::result::Result<Object, WaferError> {
        let mut msg = Message::new("svc.storage.get", Vec::new());
        msg.set_meta("bucket", bucket);
        msg.set_meta("key", key);

        let result = self.ctx.send(&msg);
        if result.action == Action::Error {
            return Err(result
                .error
                .unwrap_or_else(|| WaferError::new("unknown", "storage get failed")));
        }

        let resp = result
            .response
            .ok_or_else(|| WaferError::new("no_response", "host returned no response data"))?;

        let ct = resp
            .meta
            .iter()
            .find(|(k, _)| k == "content_type")
            .map(|(_, v)| v.clone())
            .unwrap_or_default();

        Ok(Object {
            info: ObjectInfo {
                key: key.to_string(),
                size: resp.data.len() as i64,
                content_type: ct,
            },
            data: resp.data,
        })
    }

    /// Delete an object from the given bucket and key.
    pub fn delete(&self, bucket: &str, key: &str) -> std::result::Result<(), WaferError> {
        let mut msg = Message::new("svc.storage.delete", Vec::new());
        msg.set_meta("bucket", bucket);
        msg.set_meta("key", key);

        let result = self.ctx.send(&msg);
        if result.action == Action::Error {
            return Err(result
                .error
                .unwrap_or_else(|| WaferError::new("unknown", "storage delete failed")));
        }
        Ok(())
    }
}
