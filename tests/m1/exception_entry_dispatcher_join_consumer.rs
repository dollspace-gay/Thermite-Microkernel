use std::process::ExitCode;
use tmk_exception_entry_dispatcher_join::{
    decode_execute_join, registered_image, EntryState, JoinedImageRegistration,
    COMMON_CONTINUATION, USER_CODE_SELECTOR, USER_DATA_SELECTOR,
};

fn state(user: bool) -> EntryState {
    if user {
        EntryState {
            cpl: 0,
            interrupts_enabled: false,
            direction_flag: true,
            gs_kernel_active: false,
            rsp: 0xffff_e000_0000_2f00,
            rbx: 0xbbbb_bbbb_bbbb_bbbb,
            cr2: 0x1234_5000,
            vector: 14,
            error: 6,
            resume_rip: 0x0040_1000,
            resume_cs: USER_CODE_SELECTOR,
            resume_rflags: 0x202,
            resume_rsp: 0x0000_7fff_ffff_e000,
            resume_ss: USER_DATA_SELECTOR,
            stack_switch: true,
            normalized_frame_registered: true,
            stack_low: 0xffff_e000_0000_2000,
            stack_high: 0xffff_e000_0000_4000,
            stack_readable: true,
            stack_writable: true,
            scalar_registered: true,
            scalar_returns: true,
            scalar_preserves_rbx: true,
            scalar_preserves_frame: true,
        }
    } else {
        EntryState {
            cpl: 0,
            interrupts_enabled: false,
            direction_flag: false,
            gs_kernel_active: true,
            rsp: 0xffff_e000_0000_3f08,
            rbx: 0x2222_2222_2222_2222,
            cr2: 0,
            vector: 0x20,
            error: 0,
            resume_rip: 0xffff_ffff_8002_0000,
            resume_cs: 0x08,
            resume_rflags: 0x202,
            resume_rsp: 0xffff_e000_0000_5000,
            resume_ss: 0x10,
            stack_switch: false,
            normalized_frame_registered: true,
            stack_low: 0xffff_e000_0000_3000,
            stack_high: 0xffff_e000_0000_5000,
            stack_readable: true,
            stack_writable: true,
            scalar_registered: true,
            scalar_returns: true,
            scalar_preserves_rbx: true,
            scalar_preserves_frame: true,
        }
    }
}

fn reject(state: EntryState) {
    let step = decode_execute_join(registered_image(), state);
    assert!(!step.accepted);
    assert_eq!(step.frame_base, 0);
    assert!(!step.dispatcher_precondition_established);
    assert!(!step.scalar_tail_transfer);
}

fn reject_image(image: JoinedImageRegistration) {
    let step = decode_execute_join(image, state(true));
    assert!(!step.accepted);
    assert_eq!(step.return_address, 0);
}

fn main() -> ExitCode {
    let user = decode_execute_join(registered_image(), state(true));
    assert!(user.accepted);
    assert_eq!(user.frame_base, 0xffff_e000_0000_2e80);
    assert_eq!(user.dispatcher_rsp, 0xffff_e000_0000_2e78);
    assert_eq!(user.dispatcher_rsp & 15, 8);
    assert_eq!(user.return_address, COMMON_CONTINUATION);
    assert_eq!(user.arguments.cr2, 0x1234_5000);
    assert_eq!(user.arguments.error, 6);
    assert_eq!(user.arguments.rip, 0x0040_1000);
    assert_eq!(user.arguments.rflags, 0x202);
    assert_eq!(user.arguments.user_rsp, 0x0000_7fff_ffff_e000);
    assert_eq!(user.arguments.metadata, 0x001b_0023_0000_000e);
    assert_eq!(user.frame_words_read, 8);
    assert!(user.prefix_readable && user.user_tail_readable);
    assert!(user.return_address_readable);
    assert!(user.dispatcher_precondition_established);
    assert!(user.dispatcher_df_clear);
    assert!(user.scalar_entry_aligned && user.scalar_tail_transfer);
    assert!(user.frame_unchanged && user.rbx_preserved);
    assert_eq!(user.dispatcher_return_rsp, 0xffff_e000_0000_2e80);
    assert_eq!(user.final_rip, 0x0040_1000);
    assert_eq!(user.final_rsp, 0x0000_7fff_ffff_e000);
    assert_eq!(user.final_rflags, 0x202);
    assert_eq!(user.swapgs_count, 2);

    let kernel = decode_execute_join(registered_image(), state(false));
    assert!(kernel.accepted);
    assert_eq!(kernel.frame_base, 0xffff_e000_0000_3e88);
    assert_eq!(kernel.dispatcher_rsp, 0xffff_e000_0000_3e78);
    assert_eq!(kernel.dispatcher_rsp & 15, 8);
    assert_eq!(kernel.arguments.user_rsp, 0);
    assert_eq!(kernel.arguments.metadata, 0x0000_0008_0000_0020);
    assert_eq!(kernel.frame_words_read, 6);
    assert!(kernel.prefix_readable && !kernel.user_tail_readable);
    assert_eq!(kernel.swapgs_count, 0);

    let mut invalid = state(true);
    invalid.stack_low = invalid.rsp - 143;
    reject(invalid);
    let mut invalid = state(true);
    invalid.stack_high = invalid.rsp + 55;
    reject(invalid);
    let mut invalid = state(true);
    invalid.rsp += 1;
    reject(invalid);
    let mut invalid = state(true);
    invalid.stack_readable = false;
    reject(invalid);
    let mut invalid = state(true);
    invalid.stack_writable = false;
    reject(invalid);
    let mut invalid = state(true);
    invalid.normalized_frame_registered = false;
    reject(invalid);
    let mut invalid = state(true);
    invalid.gs_kernel_active = true;
    reject(invalid);
    let mut invalid = state(true);
    invalid.resume_cs = 0x1b;
    reject(invalid);
    let mut invalid = state(true);
    invalid.vector = 256;
    reject(invalid);
    let mut invalid = state(true);
    invalid.scalar_registered = false;
    reject(invalid);
    let mut invalid = state(true);
    invalid.scalar_returns = false;
    reject(invalid);

    let mut image = registered_image();
    image.common_last ^= 1;
    reject_image(image);
    let mut image = registered_image();
    image.dispatcher_tail ^= 1;
    reject_image(image);

    assert_eq!(tmk_exception_entry_dispatcher_join::entry_dispatcher_join_observation(), 4095);
    println!(
        "M1_EXCEPTION_ENTRY_DISPATCHER_JOIN_OK common=105 dispatcher=93 user_rsp={:016x} kernel_rsp={:016x} alignment=8 continuation={:016x} rejects=13",
        user.dispatcher_rsp,
        kernel.dispatcher_rsp,
        user.return_address,
    );
    ExitCode::SUCCESS
}
