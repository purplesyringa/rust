#[lang = "eh_personality"]
fn eh_personality() {}

mod internal {
    #[rustc_intrinsic]
    extern "C-unwind" unsafe fn wasm_throw<const TAG: i32>(ptr: *mut u8) -> !;
}

unsafe fn wasm_throw(ptr: *mut u8) -> ! {
    internal::wasm_throw::<0>(ptr);
}

#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo<'_>) -> ! {
    use alloc::boxed::Box;
    use alloc::string::ToString;

    let msg = info.message().to_string();
    let exception = Box::new(msg);
    unsafe {
        let exception_raw = Box::into_raw(exception);
        wasm_throw(exception_raw as *mut u8);
    }
}
