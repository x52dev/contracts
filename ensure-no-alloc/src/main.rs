#![no_std]
#![no_main]

// Do not add a global allocator: the build must fail if generated contracts require one.
mod compat_test;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    compat_test::exercise();

    loop {
        core::hint::spin_loop();
    }
}
