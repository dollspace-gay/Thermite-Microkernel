#![no_std]
#![no_main]

extern crate tmk_composition_probe;

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn rust_eh_personality() {}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let _observed = tmk_composition_probe::composition_shell::boot_observation();
    loop {}
}
