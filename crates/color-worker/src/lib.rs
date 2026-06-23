//! Library surface of the `color` VGI worker.
//!
//! The binary (`main.rs`) is the actual worker; this `lib` target exposes the
//! pure color-science engine so integration tests under `tests/` can exercise it
//! directly, without Arrow or RPC. See [`color`] for the engine.

pub mod color;
