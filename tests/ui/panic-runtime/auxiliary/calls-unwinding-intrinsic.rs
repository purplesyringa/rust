//@ only-wasm32
//@ compile-flags:-C panic=unwind
//@ no-prefer-dynamic

#![feature(rustc_attrs)]
#![crate_type = "rlib"]
#![no_std]

#[rustc_intrinsic]
extern "C-unwind" fn wasm_throw<const TAG: i32>(ptr: *mut u8) -> !;

unsafe fn throw(ptr: *mut u8) -> ! {
    unsafe { wasm_throw::<0>(ptr) }
}
