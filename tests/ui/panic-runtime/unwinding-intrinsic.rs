//@ build-fail
//@ aux-build:calls-unwinding-intrinsic.rs
//@ only-wasm32
//@ compile-flags:-C panic=abort
//@ no-prefer-dynamic

extern crate calls_unwinding_intrinsic;

fn main() {}

//~? ERROR the crate `calls_unwinding_intrinsic` requires panic strategy `unwind` which is incompatible with this crate's strategy of `abort`
