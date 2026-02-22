//! The `Block` trait that every WAFER guest block must implement.

use crate::context::Context;
use crate::types::*;

/// The core interface that every WAFER guest block must implement.
///
/// The WAFER runtime calls:
/// - [`info`](Block::info) to discover the block's identity and capabilities.
/// - [`handle`](Block::handle) to process each incoming message.
/// - [`lifecycle`](Block::lifecycle) on init, start, and stop transitions.
///
/// Use the [`register!`](crate::register) macro to generate the required WASM
/// exports from your `Block` implementation.
pub trait Block {
    /// Return metadata describing this block.
    fn info(&self) -> BlockInfo;

    /// Process an incoming message and return a result indicating the desired
    /// action (continue, respond, drop, or error).
    fn handle(&self, ctx: &Context, msg: &mut Message) -> Result_;

    /// Handle a lifecycle event. The default implementation is a no-op.
    fn lifecycle(&self, _ctx: &Context, _event: LifecycleEvent) -> std::result::Result<(), WaferError> {
        Ok(())
    }
}
