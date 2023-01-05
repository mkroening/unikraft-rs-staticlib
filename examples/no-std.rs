#![cfg_attr(target_os = "linux", feature(c_unwind))]
#![cfg_attr(target_os = "linux", feature(lang_items))]
#![no_std]
#![no_main]

unikraft::can_run_this!();

use core::ffi::{c_char, c_int};

extern "C" {
    /// From `unikraft/lib/nolibc/include/stdio.h`:
    /// https://github.com/unikraft/unikraft/blob/f84b8bda0a0503028e67f4f7d526e2deab5f53ee/lib/nolibc/include/stdio.h#L76
    pub fn printf(format: *const c_char, ...) -> c_int;
}

#[no_mangle]
extern "C" fn main(_argc: c_int, _argv: *mut *mut c_char) -> c_int {
    unsafe {
        printf(b"Hello, no-std!\n\0".as_ptr().cast());
    }

    0
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}

#[cfg(target_os = "linux")]
#[lang = "eh_personality"]
fn eh_personality() {}

#[cfg(target_os = "linux")]
#[no_mangle]
pub extern "C-unwind" fn _Unwind_Resume(_exception: *mut ()) -> ! {
    unreachable!()
}
