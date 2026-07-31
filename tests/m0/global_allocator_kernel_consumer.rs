#![no_std]
#![no_main]

extern crate alloc;
extern crate tmk_global_allocator;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_eh_personality() {}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let boxed = Box::new(0x544d_4b31_u64);
    let mut values = Vec::with_capacity(4);
    values.push(*boxed);
    values.push(0x4d30_u64);
    core::hint::black_box((&boxed, &values));
    tmk_global_allocator::tmk_global_alloc_seal();
    loop {}
}
