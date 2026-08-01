extern crate tmk_uefi_raw_memory_map_capsule;

use std::path::Path;
use tmk_uefi_raw_memory_map_capsule::{
    decode_execute_raw_map_capsule, execute_registered_raw_map_capsule,
    registered_raw_map_image, RawMapMachineState, DESCRIPTOR_LIMIT,
    EFI_BOOT_SERVICES_SIGNATURE, EFI_BUFFER_TOO_SMALL, EFI_LOAD_ERROR,
    EFI_SYSTEM_TABLE_SIGNATURE, PAGE_LIMIT, PROBE_SIZE_LIMIT,
    RAW_MAP_IMAGE_WORD_COUNT, RAW_MAP_STACK_FRAME_BYTES,
};

const MARKER: &[u8; 11] = b"TMK_MAP_OK\n";

fn state() -> RawMapMachineState {
    RawMapMachineState {
        long_mode: true,
        identity_mapped: true,
        direction_flag: false,
        entry_rsp: 0x0000_0000_0080_0008,
        return_address: 0x0000_0000_0010_2000,
        system_table: 0x0000_0000_0071_0000,
        system_table_registered: true,
        system_signature: EFI_SYSTEM_TABLE_SIGNATURE,
        system_header_size: 120,
        boot_services: 0x0000_0000_0072_0000,
        boot_services_registered: true,
        boot_signature: EFI_BOOT_SERVICES_SIGNATURE,
        boot_header_size: 376,
        get_memory_map_target: 0x0000_0000_0073_0000,
        allocate_pool_target: 0x0000_0000_0074_0000,
        free_pool_target: 0x0000_0000_0075_0000,
        targets_registered: true,
        return_stack_registered: true,
        stack_registered: true,
        stack_writable_bytes: RAW_MAP_STACK_FRAME_BYTES,
        firmware_returns: true,
        firmware_preserves_nonvolatile: true,
        probe_status: EFI_BUFFER_TOO_SMALL,
        probe_required_size: 4096,
        allocate_status: 0,
        allocated_buffer: 0x0000_0000_0090_0000,
        allocated_buffer_registered: true,
        second_status: 0,
        returned_size: 192,
        map_key: 77,
        descriptor_size: 48,
        descriptor_version: 1,
        descriptor_count: 4,
        map_bytes_registered: true,
        descriptors_valid: true,
        usable_pages: 16,
        free_status: 0,
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
    let image = registered_raw_map_image();
    assert_eq!(image.words.len(), RAW_MAP_IMAGE_WORD_COUNT);
    let mut bytes = Vec::with_capacity(RAW_MAP_IMAGE_WORD_COUNT * 8);
    for word in image.words {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    assert_eq!(bytes.len(), 1016);
    assert_eq!(&bytes[..3], &[0x48, 0x85, 0xd2]);
    assert_eq!(&bytes[0xdf..0xe3], &[0x41, 0xff, 0x53, 0x38]);
    assert_eq!(&bytes[0x12f..0x133], &[0x41, 0xff, 0x53, 0x40]);
    assert_eq!(&bytes[0x1a6..0x1aa], &[0x41, 0xff, 0x53, 0x38]);
    assert_eq!(&bytes[0x323..0x325], &[0x74, 0x11]);
    assert_eq!(&bytes[0x328..0x32a], &[0x74, 0x0c]);
    assert_eq!(&bytes[0x3a3..0x3a7], &[0x41, 0xff, 0x53, 0x48]);
    assert_eq!(bytes[1015], 0xc3);
    std::fs::write(path, bytes).expect("write exact UEFI raw-map capsule");
}

fn clean_failure(candidate: RawMapMachineState) {
    let step = execute_registered_raw_map_capsule(candidate);
    assert!(step.accepted && step.returned && step.nonvolatile_preserved);
    assert_eq!(step.marker_bytes, 0);
    assert_eq!(step.rax, EFI_LOAD_ERROR);
    assert_eq!(step.rsp, state().entry_rsp);
    assert_eq!(step.post_rip, state().return_address);
    assert_eq!(step.rbx, state().rbx);
    assert_eq!(step.rbp, state().rbp);
    assert_eq!(step.rdi, state().rdi);
    assert_eq!(step.rsi, state().rsi);
    assert_eq!(step.r12, state().r12);
    assert_eq!(step.r13, state().r13);
    assert_eq!(step.r14, state().r14);
    assert_eq!(step.r15, state().r15);
}

fn main() {
    let mut args = std::env::args_os().skip(1);
    let output = args.next().expect("raw-map capsule output path");
    assert!(args.next().is_none(), "expected one output path");
    emit(Path::new(&output));

    let success = execute_registered_raw_map_capsule(state());
    assert!(success.accepted && success.system_header_valid && success.boot_header_valid);
    assert!(success.targets_valid && success.probe_called && success.allocate_called);
    assert!(success.second_map_called && success.free_called);
    assert!(!success.buffer_owned_at_return);
    assert_eq!(success.call_site_rsp, state().entry_rsp - RAW_MAP_STACK_FRAME_BYTES);
    assert!(success.stack_aligned);
    assert_eq!(success.shadow_bytes, 32);
    assert_eq!(success.pool_type, 2);
    assert_eq!(success.allocation_size, 4608);
    assert_eq!(success.descriptors_scanned, 4);
    assert_eq!(success.observed_map_key, 77);
    assert_eq!(success.observed_descriptor_size, 48);
    assert_eq!(success.observed_descriptor_version, 1);
    assert_eq!(success.observed_usable_pages, 16);
    assert_eq!(success.marker_bytes, MARKER.len() as u8);
    let mut marker = Vec::new();
    marker.extend_from_slice(&success.marker0.to_le_bytes());
    marker.extend_from_slice(&success.marker1.to_le_bytes()[..3]);
    assert_eq!(marker, MARKER);
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

    clean_failure(RawMapMachineState { system_table: 0, system_table_registered: false, ..state() });
    clean_failure(RawMapMachineState { system_signature: 0, ..state() });
    clean_failure(RawMapMachineState { system_header_size: 103, ..state() });
    clean_failure(RawMapMachineState { boot_services: 0, boot_services_registered: false, ..state() });
    clean_failure(RawMapMachineState { boot_signature: 0, ..state() });
    clean_failure(RawMapMachineState { boot_header_size: 79, ..state() });
    clean_failure(RawMapMachineState { get_memory_map_target: 0, ..state() });
    clean_failure(RawMapMachineState { allocate_pool_target: 0, ..state() });
    clean_failure(RawMapMachineState { free_pool_target: 0, ..state() });
    clean_failure(RawMapMachineState { probe_status: 0, ..state() });
    clean_failure(RawMapMachineState { probe_required_size: 0, ..state() });
    clean_failure(RawMapMachineState { probe_required_size: PROBE_SIZE_LIMIT + 1, ..state() });
    clean_failure(RawMapMachineState { allocate_status: 1, ..state() });
    clean_failure(RawMapMachineState { allocated_buffer: 0, ..state() });
    clean_failure(RawMapMachineState { allocated_buffer: 3, ..state() });
    clean_failure(RawMapMachineState { second_status: 1, ..state() });
    clean_failure(RawMapMachineState { returned_size: 0, ..state() });
    clean_failure(RawMapMachineState { returned_size: 4609, ..state() });
    clean_failure(RawMapMachineState { map_key: 0, ..state() });
    clean_failure(RawMapMachineState { descriptor_size: 39, ..state() });
    clean_failure(RawMapMachineState { descriptor_size: 264, ..state() });
    clean_failure(RawMapMachineState { descriptor_size: 44, ..state() });
    clean_failure(RawMapMachineState { descriptor_version: 2, ..state() });
    clean_failure(RawMapMachineState { descriptor_count: 0, ..state() });
    clean_failure(RawMapMachineState { descriptor_count: DESCRIPTOR_LIMIT + 1, ..state() });
    clean_failure(RawMapMachineState { returned_size: 193, ..state() });
    clean_failure(RawMapMachineState { descriptors_valid: false, ..state() });
    clean_failure(RawMapMachineState { usable_pages: 0, ..state() });
    clean_failure(RawMapMachineState { usable_pages: PAGE_LIMIT + 1, ..state() });

    let free_failure = execute_registered_raw_map_capsule(RawMapMachineState {
        free_status: 1,
        ..state()
    });
    assert!(free_failure.accepted && free_failure.free_called);
    assert!(free_failure.buffer_owned_at_return);
    assert_eq!(free_failure.marker_bytes, 0);
    assert_eq!(free_failure.rax, EFI_LOAD_ERROR);

    let bad_environment = execute_registered_raw_map_capsule(RawMapMachineState {
        direction_flag: true,
        ..state()
    });
    assert!(!bad_environment.accepted && !bad_environment.returned);

    let mut bad_image = registered_raw_map_image();
    bad_image.words[126] ^= 1;
    let rejected_image = decode_execute_raw_map_capsule(bad_image, state());
    assert!(!rejected_image.accepted && !rejected_image.returned);

    println!(
        "M1_UEFI_RAW_MAP_MODEL_OK bytes=1016 scenarios=33 rejected=32 calls=4 descriptors=4 free=all-paths"
    );
}
