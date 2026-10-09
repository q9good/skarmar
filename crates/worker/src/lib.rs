// This crate is a Workers entrypoint; native tests exercise core/http/api instead.
#[cfg(target_arch = "wasm32")]
mod runtime;
#[cfg(target_arch = "wasm32")]
pub use runtime::*;
