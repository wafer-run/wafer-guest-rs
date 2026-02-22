//! Service clients that communicate with the WAFER host runtime.
//!
//! Each client wraps the guest [`Context::send`](crate::context::Context::send)
//! call, constructing well-known message kinds that the host dispatches to the
//! corresponding platform service.

pub mod config;
pub mod crypto;
pub mod database;
pub mod logger;
pub mod network;
pub mod storage;
