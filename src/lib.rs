//! WAFER guest SDK for writing blocks compiled to WebAssembly (`wasm32-wasip1`).
//!
//! This crate provides the types, traits, helper functions, and service clients
//! needed to implement a WAFER block as a WASM module. The block communicates
//! with the WAFER host runtime through imported host functions in the `"wafer"`
//! module and exports a set of well-known symbols that the host calls.
//!
//! # Quick start
//!
//! ```rust,ignore
//! use wafer_guest::*;
//!
//! struct MyBlock;
//!
//! impl Default for MyBlock {
//!     fn default() -> Self { Self }
//! }
//!
//! impl Block for MyBlock {
//!     fn info(&self) -> BlockInfo {
//!         BlockInfo {
//!             name: "my-block".into(),
//!             version: "0.1.0".into(),
//!             interface: "transform".into(),
//!             summary: "A demo block".into(),
//!             instance_mode: InstanceMode::PerNode,
//!         }
//!     }
//!
//!     fn handle(&self, ctx: &Context, msg: &mut Message) -> Result_ {
//!         // Process the message...
//!         msg.clone().cont()
//!     }
//! }
//!
//! // Generate the required WASM exports.
//! wafer_guest::register!(MyBlock);
//! ```
//!
//! # Architecture
//!
//! ```text
//! +-----------+     extern "C" imports     +-------------+
//! |   Guest   | --------------------------> |    Host     |
//! |  (WASM)   | <-------------------------- |  (Runtime)  |
//! +-----------+     WASM exports           +-------------+
//! ```
//!
//! The guest writes JSON into linear memory and passes `(ptr, len)` pairs to
//! the host. Return values are packed as `i64` where the high 32 bits contain
//! the pointer and the low 32 bits contain the length.

pub mod block;
pub mod context;
pub mod helpers;
pub mod services;
pub mod types;

// Re-export the most commonly used items at the crate root.
pub use block::Block;
pub use context::Context;
pub use helpers::*;
pub use types::*;

/// Generate the WASM exports required by the WAFER host runtime.
///
/// Given a type that implements [`Block`] and [`Default`], this macro generates:
///
/// - `malloc(size: i32) -> i32` -- allocate bytes in WASM linear memory
/// - `info() -> i64` -- return block info as packed `(ptr, len)` in `i64`
/// - `handle(msg_ptr: i32, msg_len: i32) -> i64` -- handle a message
/// - `lifecycle(event_ptr: i32, event_len: i32) -> i64` -- handle a lifecycle event
///
/// # Example
///
/// ```rust,ignore
/// struct MyBlock;
/// impl Default for MyBlock { fn default() -> Self { Self } }
/// impl wafer_guest::Block for MyBlock { /* ... */ }
///
/// wafer_guest::register!(MyBlock);
/// ```
#[macro_export]
macro_rules! register {
    ($block_ty:ty) => {
        // ------------------------------------------------------------------
        // malloc: let the host allocate memory inside the guest.
        // ------------------------------------------------------------------
        #[no_mangle]
        pub extern "C" fn malloc(size: i32) -> i32 {
            let mut buf = Vec::<u8>::with_capacity(size as usize);
            let ptr = buf.as_mut_ptr();
            core::mem::forget(buf);
            ptr as i32
        }

        // ------------------------------------------------------------------
        // Helper: write bytes into linear memory and return packed i64.
        // ------------------------------------------------------------------
        #[inline]
        fn __wafer_write_result(data: &[u8]) -> i64 {
            let ptr = malloc(data.len() as i32);
            unsafe {
                core::ptr::copy_nonoverlapping(data.as_ptr(), ptr as *mut u8, data.len());
            }
            $crate::context::pack_ptr_len(ptr, data.len() as i32)
        }

        // ------------------------------------------------------------------
        // info: return block metadata.
        // ------------------------------------------------------------------
        #[no_mangle]
        pub extern "C" fn info() -> i64 {
            let block = <$block_ty as Default>::default();
            let block_info = <$block_ty as $crate::Block>::info(&block);
            let wasm_info = block_info.to_wasm();
            match serde_json::to_vec(&wasm_info) {
                Ok(data) => __wafer_write_result(&data),
                Err(_) => 0i64,
            }
        }

        // ------------------------------------------------------------------
        // handle: process an incoming message.
        // ------------------------------------------------------------------
        #[no_mangle]
        pub extern "C" fn handle(msg_ptr: i32, msg_len: i32) -> i64 {
            // Read the incoming WasmMessage from linear memory.
            let msg_slice = unsafe {
                core::slice::from_raw_parts(msg_ptr as *const u8, msg_len as usize)
            };

            let wasm_msg: $crate::types::WasmMessage = match serde_json::from_slice(msg_slice) {
                Ok(m) => m,
                Err(e) => {
                    let err_result = $crate::types::WasmResult {
                        action: "error".to_string(),
                        response: None,
                        error: Some($crate::types::WasmError {
                            code: "decode_error".to_string(),
                            message: e.to_string(),
                            meta: Vec::new(),
                        }),
                    };
                    let data = serde_json::to_vec(&err_result).unwrap_or_default();
                    return __wafer_write_result(&data);
                }
            };

            let mut msg = $crate::types::Message::from_wasm(wasm_msg);
            let ctx = $crate::Context::new();
            let block = <$block_ty as Default>::default();
            let result = <$block_ty as $crate::Block>::handle(&block, &ctx, &mut msg);
            let wasm_result = result.to_wasm();

            match serde_json::to_vec(&wasm_result) {
                Ok(data) => __wafer_write_result(&data),
                Err(_) => 0i64,
            }
        }

        // ------------------------------------------------------------------
        // lifecycle: handle init/start/stop events.
        // ------------------------------------------------------------------
        #[no_mangle]
        pub extern "C" fn lifecycle(event_ptr: i32, event_len: i32) -> i64 {
            let event_slice = unsafe {
                core::slice::from_raw_parts(event_ptr as *const u8, event_len as usize)
            };

            let wasm_event: $crate::types::WasmLifecycleEvent =
                match serde_json::from_slice(event_slice) {
                    Ok(e) => e,
                    Err(e) => {
                        let err = $crate::types::WasmResult {
                            action: "error".to_string(),
                            response: None,
                            error: Some($crate::types::WasmError {
                                code: "decode_error".to_string(),
                                message: e.to_string(),
                                meta: Vec::new(),
                            }),
                        };
                        let data = serde_json::to_vec(&err).unwrap_or_default();
                        return __wafer_write_result(&data);
                    }
                };

            let event = match $crate::types::LifecycleEvent::from_wasm(wasm_event) {
                Some(e) => e,
                None => {
                    // Unknown event type -- treat as success.
                    let ok = $crate::types::WasmResult {
                        action: "continue".to_string(),
                        response: None,
                        error: None,
                    };
                    let data = serde_json::to_vec(&ok).unwrap_or_default();
                    return __wafer_write_result(&data);
                }
            };

            let ctx = $crate::Context::new();
            let block = <$block_ty as Default>::default();

            match <$block_ty as $crate::Block>::lifecycle(&block, &ctx, event) {
                Ok(()) => {
                    let ok = $crate::types::WasmResult {
                        action: "continue".to_string(),
                        response: None,
                        error: None,
                    };
                    let data = serde_json::to_vec(&ok).unwrap_or_default();
                    __wafer_write_result(&data)
                }
                Err(e) => {
                    let err = $crate::types::WasmResult {
                        action: "error".to_string(),
                        response: None,
                        error: Some($crate::types::WasmError {
                            code: e.code,
                            message: e.message,
                            meta: e
                                .meta
                                .iter()
                                .map(|(k, v)| [k.clone(), v.clone()])
                                .collect(),
                        }),
                    };
                    let data = serde_json::to_vec(&err).unwrap_or_default();
                    __wafer_write_result(&data)
                }
            }
        }
    };
}
