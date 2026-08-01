#![no_std]
#![no_main]

extern crate tmk_exception_frame;

use core::panic::PanicInfo;
use tmk_exception_frame::exception_frame_shell::{
    dispatch_exception_frame, DispatchContext, USER_CODE_SELECTOR, USER_DATA_SELECTOR,
};
use tmk_exception_frame::{ExceptionAction, ExceptionState};

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[no_mangle]
pub extern "C" fn rust_eh_personality() {}

fn state() -> ExceptionState {
    ExceptionState {
        fault_generation: 0,
        timer_expiries: 0,
        irq_deliveries: 0,
        quarantined_vectors: 0,
        spurious_vectors: 0,
        last_tlb_epoch: 0,
        reschedule_pending: false,
        panic_latched: false,
    }
}

fn context() -> DispatchContext {
    DispatchContext {
        thread: 42,
        thread_live: true,
        fault_endpoint_valid: true,
        vspace_epoch: 77,
        irq_bound: false,
        acknowledge_required: false,
        wakes_higher_priority: false,
        shootdown_epoch: 0,
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let words = [
        15,
        14,
        13,
        12,
        11,
        10,
        9,
        8,
        0xb0,
        0xd1,
        0x51,
        0xd2,
        0xc1,
        0xb3,
        0x1234_5000,
        0xa0,
        14,
        6,
        0x0040_1000,
        USER_CODE_SELECTOR,
        0x202,
        0x0000_7fff_ffff_e000,
        USER_DATA_SELECTOR,
    ];
    let result = dispatch_exception_frame(state(), &words, &context());
    match result.1 {
        ExceptionAction::DeliverFault {
            generation: 1,
            address: 0x1234_5000,
            access: 1,
            ..
        } => {}
        _ => panic!("verified exception-frame bridge diverged"),
    }
    loop {
        core::hint::spin_loop();
    }
}
