extern crate tmk_exception_scalar_entry;

use std::env;
use std::fs;
use tmk_exception_scalar_entry::{
    decode_execute, registered_image, MachineState, COMMON_CONTINUATION, CONTROL_FAIL_STOP,
    CONTROL_RETURN, CONTROL_SCHEDULE, SCALAR_CORE_VIRTUAL,
};

fn state(control: u8) -> MachineState {
    MachineState {
        cpl: 0,
        interrupts_enabled: false,
        direction_flag: false,
        rdi_cr2: 0x1234_5000,
        rsi_error: 6,
        rdx_rip: 0x0040_1000,
        rcx_rflags: 0x202,
        r8_user_rsp: 0x0000_7fff_ffff_e000,
        r9_metadata: 0x001b_0023_0000_000e,
        rbx_frame: 0xffff_e000_0000_2e80,
        r10_frame: 0xffff_e000_0000_2e80,
        rsp: 0xffff_e000_0000_2e78,
        return_address: COMMON_CONTINUATION,
        frame_registered: true,
        return_address_readable: true,
        core_registered: true,
        core_control: control,
        core_preserves_rbx: true,
        core_preserves_frame: true,
    }
}

fn main() {
    let output = env::args_os().nth(1).expect("capsule output path");
    let image = registered_image();
    let bytes = image.qword.to_le_bytes();
    assert_eq!(bytes, [0x48, 0x89, 0xdf, 0xe9, 0xf8, 0x00, 0x00, 0x00]);
    fs::write(output, bytes).expect("write scalar-entry capsule");

    let returning = decode_execute(image, state(CONTROL_RETURN));
    assert!(returning.accepted && returning.stack_neutral_tail_jump);
    assert_eq!(returning.arguments.frame, 0xffff_e000_0000_2e80);
    assert_eq!(returning.arguments.error, 6);
    assert_eq!(returning.arguments.rip, 0x0040_1000);
    assert_eq!(returning.arguments.rflags, 0x202);
    assert_eq!(returning.arguments.user_rsp, 0x0000_7fff_ffff_e000);
    assert_eq!(returning.arguments.metadata, 0x001b_0023_0000_000e);
    assert_eq!(returning.discarded_redundant_cr2, 0x1234_5000);
    assert_eq!(returning.core_address, SCALAR_CORE_VIRTUAL);
    assert!(returning.returns_to_common && !returning.schedules && !returning.fail_stops);
    assert_eq!(returning.post_rsp, 0xffff_e000_0000_2e80);
    assert_eq!(returning.post_rip, COMMON_CONTINUATION);

    let scheduled = decode_execute(registered_image(), state(CONTROL_SCHEDULE));
    assert!(scheduled.accepted && scheduled.schedules);
    assert!(!scheduled.returns_to_common && !scheduled.fail_stops);
    assert_eq!(scheduled.post_rsp, 0);
    assert_eq!(scheduled.post_rip, 0);

    let stopped = decode_execute(registered_image(), state(CONTROL_FAIL_STOP));
    assert!(stopped.accepted && stopped.fail_stops);
    assert!(!stopped.returns_to_common && !stopped.schedules);

    for rejected in [
        MachineState { rbx_frame: 0x2000, r10_frame: 0x2000, ..state(CONTROL_RETURN) },
        MachineState { r10_frame: 0xffff_e000_0000_2e88, ..state(CONTROL_RETURN) },
        MachineState { core_registered: false, ..state(CONTROL_RETURN) },
        MachineState { core_preserves_frame: false, ..state(CONTROL_RETURN) },
    ] {
        assert!(!decode_execute(registered_image(), rejected).accepted);
    }

    println!(
        "M1_EXCEPTION_SCALAR_ENTRY_OK bytes=8 controls=return,schedule,fail-stop rejected=4 core={SCALAR_CORE_VIRTUAL:016x}"
    );
}
