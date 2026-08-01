extern crate tmk_exception_scalar_core_wrapper;

use std::env;
use std::fs;
use std::path::Path;
use tmk_exception_scalar_core_wrapper::{
    decode_execute_wrapper, install_gs, registered_control_images, registered_gs_setup_image,
    registered_wrapper_image, GsSetupState, WrapperState, COMMON_CONTINUATION,
    CONTROL_FAIL_STOP, CONTROL_RETURN, CONTROL_SCHEDULE, GS_HEADER_FLAGS, SCALAR_CORE_BLOCK_BYTES,
    USER_CODE_SELECTOR, USER_DATA_SELECTOR,
};

fn write_images(directory: &Path) {
    fs::create_dir_all(directory).expect("create image output directory");

    let gs = registered_gs_setup_image();
    let mut gs_bytes = Vec::new();
    for word in [gs.q00, gs.q01, gs.q02, gs.q03] {
        gs_bytes.extend_from_slice(&word.to_le_bytes());
    }
    gs_bytes.extend_from_slice(&gs.tail.to_le_bytes()[..3]);
    assert_eq!(gs_bytes.len(), 35);
    fs::write(directory.join("gs-setup.bin"), gs_bytes).expect("write GS setup image");

    let wrapper = registered_wrapper_image();
    let mut wrapper_bytes = Vec::new();
    for word in [
        wrapper.q00,
        wrapper.q01,
        wrapper.q02,
        wrapper.q03,
        wrapper.q04,
        wrapper.q05,
        wrapper.q06,
        wrapper.q07,
        wrapper.q08,
        wrapper.q09,
        wrapper.q10,
        wrapper.q11,
        wrapper.q12,
        wrapper.q13,
        wrapper.q14,
        wrapper.q15,
        wrapper.q16,
        wrapper.q17,
        wrapper.q18,
        wrapper.q19,
        wrapper.q20,
        wrapper.q21,
        wrapper.q22,
        wrapper.q23,
        wrapper.q24,
        wrapper.q25,
        wrapper.q26,
        wrapper.q27,
        wrapper.q28,
        wrapper.q29,
        wrapper.q30,
        wrapper.q31,
        wrapper.q32,
        wrapper.q33,
        wrapper.q34,
        wrapper.q35,
        wrapper.q36,
        wrapper.q37,
        wrapper.q38,
    ] {
        wrapper_bytes.extend_from_slice(&word.to_le_bytes());
    }
    wrapper_bytes.extend_from_slice(&wrapper.tail.to_le_bytes());
    assert_eq!(wrapper_bytes.len(), 314);
    fs::write(directory.join("scalar-wrapper.bin"), wrapper_bytes)
        .expect("write scalar wrapper image");

    let controls = registered_control_images();
    fs::write(
        directory.join("fail-stop.bin"),
        controls.fail_stop.to_le_bytes(),
    )
    .expect("write fail-stop image");
    fs::write(
        directory.join("schedule-unavailable.bin"),
        &controls.schedule_unavailable.to_le_bytes()[..5],
    )
    .expect("write schedule-unavailable image");
}

fn gs_state() -> GsSetupState {
    GsSetupState {
        cpl: 0,
        interrupts_enabled: false,
        gs_base_operand: 0xffff_e000_0000_0000,
        kernel_gs_base_operand: 0xffff_e000_0000_0000,
        current_gs_header_registered: true,
        kernel_gs_header_registered: true,
        msr_access_registered: true,
        return_stack_registered: true,
        return_address: 0xffff_ffff_8000_2000,
    }
}

fn user_state(control: u8) -> WrapperState {
    WrapperState {
        cpl: 0,
        interrupts_enabled: false,
        direction_flag: false,
        rsp: 0xffff_e000_0000_2e78,
        return_address: COMMON_CONTINUATION,
        gs_header_registered: true,
        gs_self: 0xffff_e000_0000_0000,
        gs_core_block: 0xffff_e000_0000_1000,
        gs_active_frame: 0xffff_e000_0000_2e80,
        gs_flags: GS_HEADER_FLAGS,
        frame_pointer: 0xffff_e000_0000_2e80,
        frame_prefix_registered: true,
        frame_user_tail_registered: true,
        frame_cr2: 0x1234_5000,
        frame_vector: 14,
        frame_error: 6,
        frame_rip: 0x0040_1000,
        frame_cs: USER_CODE_SELECTOR,
        frame_rflags: 0x202,
        frame_user_rsp: 0x0000_7fff_ffff_e000,
        frame_ss: USER_DATA_SELECTOR,
        transport_cr2: 0x1234_5000,
        transport_error: 6,
        transport_rip: 0x0040_1000,
        transport_rflags: 0x202,
        transport_user_rsp: 0x0000_7fff_ffff_e000,
        transport_metadata: 0x001b_0023_0000_000e,
        core_block_registered: true,
        core_block_exclusive: true,
        core_block_writable_bytes: SCALAR_CORE_BLOCK_BYTES,
        core_block_layout_80_u64: true,
        adapter_registered: true,
        adapter_receipt_bound: true,
        adapter_stack_registered: true,
        adapter_preserves_rbx: true,
        adapter_control: control,
        adapter_bridge_valid: true,
        adapter_crash_latched: control == CONTROL_FAIL_STOP,
        fail_stop_registered: true,
        schedule_stub_registered: true,
        common_return_registered: true,
    }
}

fn execute(state: WrapperState) -> tmk_exception_scalar_core_wrapper::WrapperStep {
    decode_execute_wrapper(
        registered_wrapper_image(),
        registered_control_images(),
        state,
    )
}

fn main() {
    let output = env::args_os().nth(1).expect("image output directory");
    write_images(Path::new(&output));

    let installed = install_gs(registered_gs_setup_image(), gs_state());
    assert!(installed.accepted && installed.returned);
    assert_eq!(installed.wrmsr_count, 2);
    assert_eq!(installed.gs_base, 0xffff_e000_0000_0000);
    assert_eq!(installed.kernel_gs_base, 0xffff_e000_0000_0000);
    assert!(!install_gs(
        registered_gs_setup_image(),
        GsSetupState {
            interrupts_enabled: true,
            ..gs_state()
        }
    )
    .accepted);

    let returning = execute(user_state(CONTROL_RETURN));
    assert!(returning.accepted && returning.adapter_invoked && returning.returned);
    assert!(returning.cr2_cross_checked && returning.adapter_bridge_valid);
    assert_eq!(returning.block_word_count, 23);
    assert_eq!(returning.block_frame_cr2, returning.block_arg_cr2);
    assert_eq!(returning.post_rsp, 0xffff_e000_0000_2e80);
    assert_eq!(returning.post_rip, COMMON_CONTINUATION);

    let scheduled = execute(user_state(CONTROL_SCHEDULE));
    assert!(scheduled.accepted && scheduled.schedule_requested);
    assert!(scheduled.fail_stopped && scheduled.interrupts_disabled_at_stop);
    assert!(!scheduled.returned);

    let stopped = execute(user_state(CONTROL_FAIL_STOP));
    assert!(stopped.accepted && stopped.adapter_invoked && stopped.fail_stopped);
    assert!(!stopped.schedule_requested && !stopped.returned);

    let mismatch = execute(WrapperState {
        transport_cr2: 0x9999_9000,
        adapter_control: CONTROL_FAIL_STOP,
        adapter_bridge_valid: false,
        adapter_crash_latched: true,
        ..user_state(CONTROL_RETURN)
    });
    assert!(mismatch.accepted && mismatch.adapter_invoked && mismatch.fail_stopped);
    assert!(!mismatch.cr2_cross_checked && !mismatch.adapter_bridge_valid);

    let bad_header = execute(WrapperState {
        gs_flags: 0,
        ..user_state(CONTROL_RETURN)
    });
    assert!(bad_header.accepted && !bad_header.header_valid);
    assert!(!bad_header.adapter_invoked && bad_header.fail_stopped);

    let unaligned = execute(WrapperState {
        gs_core_block: 0xffff_e000_0000_1008,
        ..user_state(CONTROL_RETURN)
    });
    assert!(unaligned.accepted && !unaligned.header_valid && unaligned.fail_stopped);

    let kernel = execute(WrapperState {
        frame_cs: 0x08,
        frame_rip: 0xffff_ffff_8000_4000,
        frame_user_rsp: 0,
        frame_ss: 0,
        frame_user_tail_registered: false,
        transport_rip: 0xffff_ffff_8000_4000,
        transport_user_rsp: 0,
        transport_metadata: 0x0000_0008_0000_000e,
        ..user_state(CONTROL_RETURN)
    });
    assert!(kernel.accepted && kernel.returned);
    assert_eq!(kernel.block_word_count, 21);
    assert_eq!(kernel.block_frame_user_rsp, 0);
    assert_eq!(kernel.block_frame_ss, 0);

    for rejected in [
        WrapperState {
            frame_user_tail_registered: false,
            ..user_state(CONTROL_RETURN)
        },
        WrapperState {
            adapter_registered: false,
            ..user_state(CONTROL_RETURN)
        },
        WrapperState {
            core_block_exclusive: false,
            ..user_state(CONTROL_RETURN)
        },
    ] {
        assert!(!execute(rejected).accepted);
    }

    println!(
        "M1_EXCEPTION_SCALAR_CORE_WRAPPER_OK images=35,314,4,5 scenarios=10 rejected=4 routes=return,schedule-fail-closed,fail-stop cross-check=frame-vs-register"
    );
}
