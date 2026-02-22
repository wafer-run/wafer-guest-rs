//! Guest context that wraps `extern "C"` host function calls.
//!
//! The guest communicates with the WAFER host runtime through three imported
//! functions in the `"wafer"` module:
//!
//! - `send(msg_ptr, msg_len) -> i64` -- send a message to a host capability
//! - `capabilities() -> i64` -- list available host capabilities
//! - `is_cancelled() -> i32` -- check if the current execution has been cancelled
//!
//! Return values that carry data encode `(ptr, len)` as a packed `i64`:
//! high 32 bits = pointer, low 32 bits = length.

use crate::types::*;

// ---------------------------------------------------------------------------
// Host function imports
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "wafer")]
extern "C" {
    /// Send a message to the host. The message is a JSON-encoded [`WasmMessage`]
    /// written to linear memory at `(msg_ptr, msg_len)`.
    ///
    /// Returns a packed `i64` where the high 32 bits are the pointer to the
    /// result JSON ([`WasmResult`]) and the low 32 bits are its length.
    #[link_name = "send"]
    fn host_send(msg_ptr: i32, msg_len: i32) -> i64;

    /// Query the capabilities advertised by the host.
    ///
    /// Returns a packed `i64` (ptr high, len low) pointing to a JSON array.
    #[link_name = "capabilities"]
    fn host_capabilities() -> i64;

    /// Check whether the current execution context has been cancelled.
    ///
    /// Returns `1` if cancelled, `0` otherwise.
    #[link_name = "is_cancelled"]
    fn host_is_cancelled() -> i32;
}

// ---------------------------------------------------------------------------
// Helpers for packed pointer/length
// ---------------------------------------------------------------------------

/// Pack a pointer and length into a single `i64`.
pub fn pack_ptr_len(ptr: i32, len: i32) -> i64 {
    ((ptr as i64) << 32) | ((len as i64) & 0xFFFF_FFFF)
}

/// Unpack a packed `i64` into `(ptr, len)`.
pub fn unpack_ptr_len(packed: i64) -> (i32, i32) {
    let ptr = (packed >> 32) as i32;
    let len = (packed & 0xFFFF_FFFF) as i32;
    (ptr, len)
}

// ---------------------------------------------------------------------------
// Context
// ---------------------------------------------------------------------------

/// Guest-side context providing access to host capabilities.
///
/// All methods internally serialize messages as JSON, write them into WASM
/// linear memory, call the corresponding host import, read the response
/// from linear memory, and deserialize it.
pub struct Context;

impl Context {
    /// Create a new guest context.
    ///
    /// This is essentially zero-cost since the context holds no state; all
    /// state lives on the host side.
    pub fn new() -> Self {
        Self
    }

    /// Send a [`Message`] to the host and return the resulting [`Result_`].
    #[cfg(target_arch = "wasm32")]
    pub fn send(&self, msg: &Message) -> Result_ {
        let wasm_msg = msg.to_wasm();
        let json = match serde_json::to_vec(&wasm_msg) {
            Ok(j) => j,
            Err(e) => {
                return Result_::error(WaferError::new("encode_error", e.to_string()));
            }
        };

        let (result_ptr, result_len) = unsafe {
            let packed = host_send(json.as_ptr() as i32, json.len() as i32);
            unpack_ptr_len(packed)
        };

        if result_ptr == 0 && result_len == 0 {
            return Result_::error(WaferError::new("host_error", "host returned null"));
        }

        let result_slice = unsafe {
            core::slice::from_raw_parts(result_ptr as *const u8, result_len as usize)
        };

        match serde_json::from_slice::<WasmResult>(result_slice) {
            Ok(wr) => Result_::from_wasm(wr),
            Err(e) => Result_::error(WaferError::new("decode_error", e.to_string())),
        }
    }

    /// Send a [`Message`] to the host (no-op stub for non-WASM targets).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn send(&self, _msg: &Message) -> Result_ {
        Result_ {
            action: Action::Continue,
            response: None,
            error: None,
            message: None,
        }
    }

    /// Query the host's available capabilities as raw JSON bytes.
    #[cfg(target_arch = "wasm32")]
    pub fn capabilities_raw(&self) -> Vec<u8> {
        let (ptr, len) = unsafe {
            let packed = host_capabilities();
            unpack_ptr_len(packed)
        };

        if ptr == 0 && len == 0 {
            return b"[]".to_vec();
        }

        unsafe { core::slice::from_raw_parts(ptr as *const u8, len as usize).to_vec() }
    }

    /// Query the host's available capabilities (no-op stub for non-WASM targets).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn capabilities_raw(&self) -> Vec<u8> {
        b"[]".to_vec()
    }

    /// Check whether the host has cancelled the current execution.
    #[cfg(target_arch = "wasm32")]
    pub fn is_cancelled(&self) -> bool {
        unsafe { host_is_cancelled() != 0 }
    }

    /// Check whether the host has cancelled (always false for non-WASM targets).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn is_cancelled(&self) -> bool {
        false
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}
