extern crate tmk_exception_common_capsule;

use tmk_exception_common_capsule::{
    CapsuleImage, COMMON_ENTRY_VIRTUAL, KERNEL_CODE_SELECTOR, MachineState, USER_CODE_SELECTOR,
    USER_DATA_SELECTOR, common_entry_observation, decode_execute, registered_image,
};

fn state() -> MachineState {
    MachineState {
        rax: 1, rbx: 2, rcx: 3, rdx: 4, rsi: 5, rdi: 6, rbp: 7,
        r8: 8, r9: 9, r10: 10, r11: 11, r12: 12, r13: 13, r14: 14, r15: 15,
        rsp: 0xffff_e000_0000_2f00,
        rip: COMMON_ENTRY_VIRTUAL,
        rflags: 0x402,
        cr2: 0x0000_1234_5000,
        cs: KERNEL_CODE_SELECTOR,
        ss: 0x10,
        cpl: 0,
        interrupts_enabled: false,
        direction_flag: true,
        gs_kernel_active: false,
        vector: 14,
        error_code: 7,
        resume_rip: 0x0000_0000_0040_1000,
        resume_cs: USER_CODE_SELECTOR,
        resume_rflags: 0x602,
        resume_rsp: 0x0000_7fff_ffff_e000,
        resume_ss: USER_DATA_SELECTOR,
        stack_switch: true,
        normalized_frame_registered: true,
        entry_stack_writable: true,
        entry_stack_readable: true,
        dispatcher_registered: true,
        dispatcher_returns: true,
        dispatcher_preserves_rbx: true,
        dispatcher_preserves_frame: true,
    }
}

fn bytes(image: &CapsuleImage) -> Vec<u8> {
    let mut result = Vec::with_capacity(105);
    for qword in [
        image.qword0, image.qword1, image.qword2, image.qword3, image.qword4,
        image.qword5, image.qword6, image.qword7, image.qword8, image.qword9,
        image.qword10, image.qword11, image.qword12,
    ] {
        result.extend_from_slice(&qword.to_le_bytes());
    }
    result.push(image.tail);
    result
}

fn main() {
    let image = registered_image();
    let encoded = bytes(&image);
    assert_eq!(encoded.len(), 105);
    assert_eq!(&encoded[..5], &[0x50, 0x0f, 0x20, 0xd0, 0x50]);
    assert_eq!(&encoded[27..40], &[0xf6, 0x84, 0x24, 0x98, 0, 0, 0, 3, 0x74, 3, 0x0f, 1, 0xf8]);
    assert_eq!(&encoded[51..56], &[0xe8, 0xc8, 0, 0, 0]);
    assert_eq!(&encoded[59..72], &[0xf6, 0x84, 0x24, 0x98, 0, 0, 0, 3, 0x74, 3, 0x0f, 1, 0xf8]);
    assert_eq!(&encoded[99..], &[0x48, 0x83, 0xc4, 0x10, 0x48, 0xcf]);

    let mut args = std::env::args_os().skip(1);
    if let Some(path) = args.next() {
        assert!(args.next().is_none(), "expected at most one capsule output path");
        std::fs::write(path, &encoded).expect("write verified common-entry bytes");
    }

    let user = decode_execute(registered_image(), state());
    assert!(user.accepted);
    assert_eq!(user.captured_cr2, 0x0000_1234_5000);
    assert_eq!(user.dispatcher_frame, 0xffff_e000_0000_2e80);
    assert_eq!(user.dispatcher_vector, 14);
    assert_eq!(user.dispatcher_error, 7);
    assert!(user.dispatcher_df_clear);
    assert_eq!(user.swapgs_count, 2);
    assert_eq!(user.state.rax, 1);
    assert_eq!(user.state.r15, 15);
    assert_eq!(user.state.rip, 0x0000_0000_0040_1000);
    assert_eq!(user.state.rsp, 0x0000_7fff_ffff_e000);
    assert_eq!(user.state.cs, USER_CODE_SELECTOR);
    assert_eq!(user.state.ss, USER_DATA_SELECTOR);
    assert_eq!(user.state.cpl, 3);
    assert!(user.state.interrupts_enabled);
    assert!(user.state.direction_flag);
    assert!(!user.state.gs_kernel_active);
    assert_eq!(common_entry_observation(), 255);

    let kernel = decode_execute(
        registered_image(),
        MachineState {
            gs_kernel_active: true,
            vector: 0xe0,
            error_code: 0,
            resume_rip: 0xffff_ffff_8000_4000,
            resume_cs: KERNEL_CODE_SELECTOR,
            resume_rflags: 0x2,
            resume_rsp: 0xffff_e000_0000_2f80,
            resume_ss: 0x10,
            stack_switch: false,
            ..state()
        },
    );
    assert!(kernel.accepted);
    assert_eq!(kernel.swapgs_count, 0);
    assert_eq!(kernel.state.cpl, 0);
    assert!(kernel.state.gs_kernel_active);

    let malformed = decode_execute(
        CapsuleImage { tail: 0xce, ..registered_image() },
        state(),
    );
    assert!(!malformed.accepted);
    assert_eq!(malformed.state.rsp, 0xffff_e000_0000_2f00);
    for rejected in [
        MachineState { cpl: 3, ..state() },
        MachineState { interrupts_enabled: true, ..state() },
        MachineState { normalized_frame_registered: false, ..state() },
        MachineState { entry_stack_writable: false, ..state() },
        MachineState { dispatcher_registered: false, ..state() },
        MachineState { dispatcher_returns: false, ..state() },
        MachineState { dispatcher_preserves_rbx: false, ..state() },
        MachineState { dispatcher_preserves_frame: false, ..state() },
        MachineState { rsp: 150, ..state() },
        MachineState { resume_rip: 0x0000_8000_0000_0000, ..state() },
        MachineState { resume_rflags: 0x3002, ..state() },
        MachineState { resume_ss: 0x10, ..state() },
        MachineState { gs_kernel_active: true, ..state() },
    ] {
        assert!(!decode_execute(registered_image(), rejected).accepted);
    }

    println!(
        "M1_EXCEPTION_COMMON_OK bytes=105 vector={} cr2={:016x} frame={:016x} swapgs={} iret_cpl={}",
        user.dispatcher_vector, user.captured_cr2, user.dispatcher_frame, user.swapgs_count,
        user.state.cpl
    );
}
