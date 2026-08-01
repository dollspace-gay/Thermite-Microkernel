extern crate tmk_exception_frame;

use tmk_exception_frame::exception_frame_shell::{
    dispatch_exception_frame, exception_frame_valid, DispatchContext, KERNEL_CODE_SELECTOR,
    USER_CODE_SELECTOR, USER_DATA_SELECTOR,
};
use tmk_exception_frame::{ExceptionAction, ExceptionState};

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

fn context(fault_endpoint_valid: bool) -> DispatchContext {
    DispatchContext {
        thread: 42,
        thread_live: true,
        fault_endpoint_valid,
        vspace_epoch: 77,
        irq_bound: false,
        acknowledge_required: false,
        wakes_higher_priority: false,
        shootdown_epoch: 0,
    }
}

fn kernel_frame(vector: u64, error: u64, cr2: u64) -> [u64; 21] {
    [
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
        cr2,
        0xa0,
        vector,
        error,
        0xffff_ffff_8000_2000,
        KERNEL_CODE_SELECTOR,
        0x202,
    ]
}

fn user_frame(vector: u64, error: u64, cr2: u64) -> [u64; 23] {
    [
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
        cr2,
        0xa0,
        vector,
        error,
        0x0000_0000_0040_1000,
        USER_CODE_SELECTOR,
        0x202,
        0x0000_7fff_ffff_e000,
        USER_DATA_SELECTOR,
    ]
}

fn expect_panic(words: &[u64], reason: u32) {
    let result = dispatch_exception_frame(state(), words, &context(true));
    match result.1 {
        ExceptionAction::Panic { reason: actual } => assert_eq!(actual, reason),
        _ => panic!("malformed exception frame did not fail-stop"),
    }
    assert!(result.0.panic_latched);
    assert!(!result.0.reschedule_pending);
}

fn main() {
    let mut observation = 0u64;

    let user_page = user_frame(14, 6, 0x1234_5000);
    assert!(exception_frame_valid(&user_page));
    let page = dispatch_exception_frame(state(), &user_page, &context(true));
    match page.1 {
        ExceptionAction::DeliverFault {
            generation,
            thread,
            kind,
            vector,
            error,
            address,
            access,
            vspace_epoch,
        } => {
            assert_eq!(generation, 1);
            assert_eq!(thread, 42);
            assert_eq!(kind, 1);
            assert_eq!(vector, 14);
            assert_eq!(error, 6);
            assert_eq!(address, 0x1234_5000);
            assert_eq!(access, 1);
            assert_eq!(vspace_epoch, 77);
        }
        _ => panic!("valid user page fault was not delivered"),
    }
    observation |= 1;

    let kernel_timer = kernel_frame(0xe0, 0, 0);
    assert!(exception_frame_valid(&kernel_timer));
    let timer = dispatch_exception_frame(state(), &kernel_timer, &context(true));
    assert!(matches!(
        timer.1,
        ExceptionAction::TimerRecorded { expiries: 1 }
    ));
    assert!(timer.0.reschedule_pending);
    observation |= 2;

    let user_general = user_frame(13, 0, 0);
    let terminate = dispatch_exception_frame(state(), &user_general, &context(false));
    assert!(matches!(
        terminate.1,
        ExceptionAction::TerminateThread {
            thread: 42,
            vector: 13
        }
    ));
    observation |= 4;

    let kernel_page = kernel_frame(14, 0, 0x3000);
    expect_panic(&kernel_page, 5);
    observation |= 8;

    assert!(!exception_frame_valid(&user_page[..21]));
    expect_panic(&user_page[..21], 2);
    observation |= 16;

    let mut bad_ss = user_page;
    bad_ss[22] = KERNEL_CODE_SELECTOR;
    assert!(!exception_frame_valid(&bad_ss));
    expect_panic(&bad_ss, 2);
    observation |= 32;

    let mut bad_rsp = user_page;
    bad_rsp[21] = 0x0000_8000_0000_0000;
    assert!(!exception_frame_valid(&bad_rsp));
    expect_panic(&bad_rsp, 2);
    observation |= 64;

    let mut bad_rip = kernel_timer;
    bad_rip[18] = 0x0040_1000;
    assert!(!exception_frame_valid(&bad_rip));
    expect_panic(&bad_rip, 2);
    observation |= 128;

    let mut bad_flags = kernel_timer;
    bad_flags[20] = 0;
    assert!(!exception_frame_valid(&bad_flags));
    expect_panic(&bad_flags, 2);
    observation |= 256;

    let mut bad_cs = kernel_timer;
    bad_cs[19] = USER_CODE_SELECTOR;
    assert!(!exception_frame_valid(&bad_cs));
    expect_panic(&bad_cs, 2);
    observation |= 512;

    let bad_vector = kernel_frame(256, 0, 0);
    assert!(!exception_frame_valid(&bad_vector));
    expect_panic(&bad_vector, 2);
    observation |= 1024;

    assert!(!exception_frame_valid(&kernel_timer[..16]));
    expect_panic(&kernel_timer[..16], 2);
    observation |= 2048;

    assert_eq!(observation, 4095);
    println!("M1_EXCEPTION_FRAME_OK words=21/23 scenarios=12 observation={observation}");
}
