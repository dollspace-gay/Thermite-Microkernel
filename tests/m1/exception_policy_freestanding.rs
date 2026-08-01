#![no_std]
#![no_main]

extern crate tmk_exception_policy;

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[no_mangle]
pub extern "C" fn rust_eh_personality() {}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let observed =
        tmk_exception_policy::exception_policy_shell::exception_policy_observation();
    if observed != 262143 {
        panic!("verified exception policy diverged");
    }
    loop {
        core::hint::spin_loop();
    }
}
