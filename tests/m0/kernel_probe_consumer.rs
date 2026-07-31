#![no_std]
#![no_main]

extern crate tmk_probe;

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let _answer = tmk_probe::transition_probe(0x55aa, 0x0f0f);
    loop {
        core::hint::spin_loop();
    }
}
