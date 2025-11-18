//! A shim for libunwind implemented in terms of the native wasm `throw` instruction.

#![allow(nonstandard_style)]

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum _Unwind_Reason_Code {
    _URC_NO_REASON = 0,
    _URC_FOREIGN_EXCEPTION_CAUGHT = 1,
    _URC_FATAL_PHASE2_ERROR = 2,
    _URC_FATAL_PHASE1_ERROR = 3,
    _URC_NORMAL_STOP = 4,
    _URC_END_OF_STACK = 5,
    _URC_HANDLER_FOUND = 6,
    _URC_INSTALL_CONTEXT = 7,
    _URC_CONTINUE_UNWIND = 8,
    _URC_FAILURE = 9, // used only by ARM EHABI
}
pub use _Unwind_Reason_Code::*;

pub type _Unwind_Exception_Class = u64;
pub type _Unwind_Word = *const u8;

pub const unwinder_private_data_size: usize = 2;

#[repr(C)]
pub struct _Unwind_Exception {
    pub exception_class: _Unwind_Exception_Class,
    pub exception_cleanup: _Unwind_Exception_Cleanup_Fn,
    pub private: [_Unwind_Word; unwinder_private_data_size],
}

pub type _Unwind_Exception_Cleanup_Fn =
    Option<extern "C" fn(unwind_code: _Unwind_Reason_Code, exception: *mut _Unwind_Exception)>;

pub unsafe fn _Unwind_DeleteException(exception: *mut _Unwind_Exception) {
    if let Some(exception_cleanup) = unsafe { (*exception).exception_cleanup } {
        exception_cleanup(_URC_FOREIGN_EXCEPTION_CAUGHT, exception);
    }
}

pub unsafe fn _Unwind_RaiseException(exception: *mut _Unwind_Exception) -> _Unwind_Reason_Code {
    // This implementation is only used for `wasm*-unknown-unknown` targets. Such targets are not
    // guaranteed to support exceptions, and they default to `-C panic=abort`. Because an unknown
    // instruction is a load-time error on wasm, instead of a runtime error like on traditional
    // architectures, we never want to codegen a `throw` instruction unless the user explicitly
    // enabled exceptions via `-Z build-std` with `-C panic=unwind`.
    cfg_select! {
        panic = "unwind" => {
            // This intrinsic is marked `extern "C-unwind"` instead of implicit `extern "Rust"`
            // because it can unwind even if the `panic_abort` runtime is used, and leaking
            // exceptions into `-C panic=abort` Rust code is unsound. Using `C-unwind` guarantees
            // that if this module is compiled with `-C panic=unwind`, the required panic strategy
            // for the binary is inferred as `unwind`, and thus any safety violations are detected.
            //
            // This particular module is never precompiled with `-C panic=unwind` because it's only
            // used for bare-metal targets, so this error can only arise if the user both manually
            // recompiles `std` with `-C panic=unwind` and manually compiles the binary crate with
            // `-C panic=abort`, which we don't care to support.
            //
            // On the flip side, it's important that this intrinsic is defined here rather than in
            // `core`, since it allows unwinding `core` shipped for Emscripten to be linked with
            // `-C panic=abort` binaries.
            //
            // See https://github.com/rust-lang/rust/issues/148246.
            #[rustc_intrinsic]
            extern "C-unwind" unsafe fn wasm_throw<const TAG: i32>(ptr: *mut u8) -> !;

            // The wasm `throw` instruction takes a "tag", which differentiates certain types of
            // exceptions from others. LLVM currently just identifies these via integers, with 0
            // corresponding to C++ exceptions and 1 to C setjmp()/longjmp(). Ideally, we'd be able
            // to choose something unique for Rust, but for now, we pretend to be C++ and implement
            // the Itanium exception-handling ABI.
            // corresponds with llvm::WebAssembly::Tag::CPP_EXCEPTION
            //     in llvm-project/llvm/include/llvm/CodeGen/WasmEHFuncInfo.h
            const CPP_EXCEPTION_TAG: i32 = 0;
            unsafe { wasm_throw::<CPP_EXCEPTION_TAG>(exception.cast()) }
        }
        _ => {
            let _ = exception;
            core::arch::wasm::unreachable()
        }
    }
}
