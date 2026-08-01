extern crate tmk_uefi_boot_services_gateway;

use std::path::Path;
use tmk_uefi_boot_services_gateway::{
    decode_execute_gateway, execute_registered_gateway, registered_gateway_image, GatewayState,
    EFI_BOOT_SERVICES_SIGNATURE, EFI_BUFFER_TOO_SMALL, EFI_LOAD_ERROR,
    EFI_SYSTEM_TABLE_SIGNATURE, GATEWAY_SHADOW_BYTES, GATEWAY_STACK_FRAME_BYTES,
    MEMORY_MAP_SIZE_LIMIT,
};

const MARKER: &[u8; 20] = b"TMK_M1_UEFI_GATE_OK\n";

fn state() -> GatewayState {
    GatewayState {
        long_mode: true,
        identity_mapped: true,
        direction_flag: false,
        entry_rsp: 0x0000_0000_0080_0008,
        return_address: 0x0000_0000_0010_2000,
        image_handle: 0x0000_0000_0070_0000,
        system_table: 0x0000_0000_0071_0000,
        system_table_registered: true,
        system_signature: EFI_SYSTEM_TABLE_SIGNATURE,
        system_header_size: 120,
        boot_services: 0x0000_0000_0072_0000,
        boot_services_registered: true,
        boot_signature: EFI_BOOT_SERVICES_SIGNATURE,
        boot_header_size: 376,
        get_memory_map_target: 0x0000_0000_0073_0000,
        target_registered: true,
        return_stack_registered: true,
        stack_registered: true,
        stack_writable_bytes: GATEWAY_STACK_FRAME_BYTES,
        firmware_returns: true,
        firmware_preserves_nonvolatile: true,
        firmware_status: EFI_BUFFER_TOO_SMALL,
        returned_required_size: 4096,
        rbx: 0x1111,
        rbp: 0x2222,
        rdi: 0x3333,
        rsi: 0x4444,
        r12: 0x5555,
        r13: 0x6666,
        r14: 0x7777,
        r15: 0x8888,
    }
}

fn emit(path: &Path) {
    let image = registered_gateway_image();
    let words = [
        image.q00, image.q01, image.q02, image.q03, image.q04, image.q05, image.q06,
        image.q07, image.q08, image.q09, image.q10, image.q11, image.q12, image.q13,
        image.q14, image.q15, image.q16, image.q17, image.q18, image.q19, image.q20,
        image.q21, image.q22, image.q23, image.q24, image.q25, image.q26, image.q27,
        image.q28, image.q29, image.q30, image.q31, image.q32, image.q33, image.q34,
        image.q35, image.q36, image.q37,
    ];
    let mut bytes = Vec::with_capacity(308);
    for word in words {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    bytes.extend_from_slice(&image.tail.to_le_bytes());
    assert_eq!(bytes.len(), 308);
    assert_eq!(&bytes[0x33..0x37], &[0x4c, 0x8b, 0x5a, 0x60]);
    assert_eq!(&bytes[0x6b..0x6f], &[0x4d, 0x8b, 0x53, 0x38]);
    assert_eq!(&bytes[0x78..0x7c], &[0x48, 0x83, 0xec, 0x68]);
    assert_eq!(&bytes[0xb5..0xba], &[0x48, 0x89, 0x44, 0x24, 0x20]);
    assert_eq!(&bytes[0xba..0xbd], &[0x41, 0xff, 0xd2]);
    assert_eq!(&bytes[0xe2..0xe6], &[0x66, 0xba, 0xe9, 0x00]);
    assert_eq!(&bytes[0x133..0x134], &[0xc3]);
    std::fs::write(path, bytes).expect("write exact UEFI gateway image");
}

fn clean_failure(candidate: GatewayState, call_expected: bool) {
    let step = execute_registered_gateway(candidate);
    assert!(step.accepted);
    assert_eq!(step.call_invoked, call_expected);
    assert_eq!(step.marker_bytes, 0);
    assert_eq!(step.rax, EFI_LOAD_ERROR);
    assert_eq!(step.post_rip, state().return_address);
    assert_eq!(step.rbx, state().rbx);
    assert_eq!(step.rbp, state().rbp);
    assert_eq!(step.rdi, state().rdi);
    assert_eq!(step.rsi, state().rsi);
    assert_eq!(step.r12, state().r12);
    assert_eq!(step.r13, state().r13);
    assert_eq!(step.r14, state().r14);
    assert_eq!(step.r15, state().r15);
    assert!(step.returned);
    assert!(step.nonvolatile_preserved);
}

fn main() {
    let mut args = std::env::args_os().skip(1);
    let output = args.next().expect("gateway image output path");
    assert!(args.next().is_none(), "expected one gateway image output path");
    emit(Path::new(&output));

    let success = execute_registered_gateway(state());
    assert!(success.accepted && success.system_header_valid && success.boot_header_valid);
    assert!(success.target_valid && success.call_invoked && success.stack_aligned);
    assert_eq!(success.call_site_rsp, state().entry_rsp - GATEWAY_STACK_FRAME_BYTES);
    assert_eq!(success.shadow_bytes, GATEWAY_SHADOW_BYTES);
    assert_eq!(success.arg_memory_map_size, state().entry_rsp - 64);
    assert_eq!(success.arg_memory_map, 0);
    assert_eq!(success.arg_map_key, state().entry_rsp - 56);
    assert_eq!(success.arg_descriptor_size, state().entry_rsp - 48);
    assert_eq!(success.fifth_arg_slot, state().entry_rsp - 72);
    assert_eq!(success.arg_descriptor_version, state().entry_rsp - 40);
    assert_eq!(success.observed_status, EFI_BUFFER_TOO_SMALL);
    assert_eq!(success.observed_required_size, 4096);
    assert_eq!(success.marker_bytes, MARKER.len() as u8);
    let mut observed_marker = Vec::new();
    observed_marker.extend_from_slice(&success.marker0.to_le_bytes());
    observed_marker.extend_from_slice(&success.marker1.to_le_bytes());
    observed_marker.extend_from_slice(&success.marker2.to_le_bytes());
    assert_eq!(observed_marker, MARKER);
    assert_eq!(success.rax, 0);
    assert_eq!(success.rsp, state().entry_rsp);
    assert_eq!(success.post_rip, state().return_address);
    assert_eq!(success.rbx, state().rbx);
    assert_eq!(success.rbp, state().rbp);
    assert_eq!(success.rdi, state().rdi);
    assert_eq!(success.rsi, state().rsi);
    assert_eq!(success.r12, state().r12);
    assert_eq!(success.r13, state().r13);
    assert_eq!(success.r14, state().r14);
    assert_eq!(success.r15, state().r15);
    assert!(success.returned && success.nonvolatile_preserved);

    clean_failure(
        GatewayState {
            system_table: 0,
            system_table_registered: false,
            ..state()
        },
        false,
    );
    clean_failure(GatewayState { system_table: 3, ..state() }, false);
    clean_failure(GatewayState { system_signature: 0, ..state() }, false);
    clean_failure(GatewayState { system_header_size: 103, ..state() }, false);
    clean_failure(
        GatewayState {
            boot_services: 0,
            boot_services_registered: false,
            ..state()
        },
        false,
    );
    clean_failure(GatewayState { boot_services: 3, ..state() }, false);
    clean_failure(GatewayState { boot_signature: 0, ..state() }, false);
    clean_failure(GatewayState { boot_header_size: 63, ..state() }, false);
    clean_failure(
        GatewayState {
            get_memory_map_target: 0,
            target_registered: false,
            stack_registered: false,
            firmware_returns: false,
            firmware_preserves_nonvolatile: false,
            ..state()
        },
        false,
    );
    clean_failure(GatewayState { firmware_status: 0, ..state() }, true);
    clean_failure(GatewayState { returned_required_size: 0, ..state() }, true);
    clean_failure(
        GatewayState { returned_required_size: MEMORY_MAP_SIZE_LIMIT + 1, ..state() },
        true,
    );

    let bad_environment = execute_registered_gateway(GatewayState {
        direction_flag: true,
        ..state()
    });
    assert!(!bad_environment.accepted && !bad_environment.returned);

    let mut bad_image = registered_gateway_image();
    bad_image.q37 ^= 1;
    let rejected_image = decode_execute_gateway(bad_image, state());
    assert!(!rejected_image.accepted && !rejected_image.returned);

    println!(
        "M1_UEFI_GATEWAY_MODEL_OK bytes=308 scenarios=15 rejected=14 call=get-memory-map args=5 shadow=32"
    );
}
