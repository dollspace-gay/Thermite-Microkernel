extern crate tmk_platform_primitives;

use std::path::Path;
use tmk_platform_primitives::{
    BootArenaState, PrimitiveCapsuleImage, decode_execute_seal_capsule,
    execute_global_alloc_null_path, execute_registered_alloc_capsule,
    execute_registered_global_alloc_adapter, execute_registered_memcpy_observation,
    execute_registered_memset_observation, execute_registered_seal_capsule, registered_alloc_image,
    registered_global_alloc_relocations, registered_global_alloc_shim, registered_memcpy_image,
    registered_memset_image, registered_seal_image,
};

fn write_words(path: &Path, words: &[u64], length: usize) {
    let mut bytes = Vec::with_capacity(words.len() * 8);
    for word in words {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    bytes.truncate(length);
    std::fs::write(path, bytes).expect("write proved platform primitive bytes");
}

fn main() {
    let output = std::env::args_os()
        .nth(1)
        .expect("expected output directory");
    assert!(
        std::env::args_os().nth(2).is_none(),
        "expected one output directory"
    );
    let output = Path::new(&output);
    std::fs::create_dir_all(output).expect("create output directory");

    let alloc = registered_alloc_image();
    write_words(
        &output.join("alloc.bin"),
        &[
            alloc.word00,
            alloc.word01,
            alloc.word02,
            alloc.word03,
            alloc.word04,
            alloc.word05,
            alloc.word06,
            alloc.word07,
            alloc.word08,
            alloc.word09,
            alloc.word10,
            alloc.word11,
            alloc.word12,
            alloc.word13,
        ],
        111,
    );
    let seal = registered_seal_image();
    write_words(&output.join("seal.bin"), &[seal.word0, seal.word1], 12);
    let memcpy = registered_memcpy_image();
    write_words(&output.join("memcpy.bin"), &[memcpy.word0, memcpy.word1], 9);
    let memset = registered_memset_image();
    write_words(
        &output.join("memset.bin"),
        &[memset.word0, memset.word1],
        14,
    );

    let shim = registered_global_alloc_shim();
    assert_eq!(
        shim.rust_alloc.to_le_bytes(),
        [0x48, 0x89, 0xfa, 0xe9, 0, 0, 0, 0]
    );
    assert_eq!(&shim.rust_null.to_le_bytes()[..3], &[0x31, 0xc0, 0xc3]);
    assert_eq!(shim.rust_dealloc as u8, 0xc3);
    assert_eq!(
        shim.method_word0.to_le_bytes(),
        [0x48, 0x89, 0xf0, 0x48, 0xc7, 0xc7, 0, 0]
    );
    assert_eq!(
        shim.method_word1.to_le_bytes(),
        [0, 0, 0x48, 0x89, 0xd6, 0x48, 0x89, 0xc2]
    );
    assert_eq!(&shim.method_word2.to_le_bytes()[..5], &[0xe9, 0, 0, 0, 0]);
    assert_eq!(
        shim.seal_word0.to_le_bytes(),
        [0x48, 0xc7, 0xc7, 0, 0, 0, 0, 0xe9]
    );
    assert_eq!(&shim.seal_word1.to_le_bytes()[..4], &[0, 0, 0, 0]);
    let relocations = registered_global_alloc_relocations();
    assert_eq!(relocations.rust_alloc_dispatch_offset, 4);
    assert_eq!(relocations.method_arena_offset, 6);
    assert_eq!(relocations.method_capsule_offset, 17);
    assert_eq!(relocations.seal_arena_offset, 3);
    assert_eq!(relocations.seal_capsule_offset, 8);

    let initial = BootArenaState {
        base: 0x20_000,
        cursor: 3,
        sealed: false,
    };
    let first_execution = execute_registered_alloc_capsule(initial, 8, 8);
    assert!(first_execution.accepted);
    let first = first_execution.step;
    assert!(first.allocation.ok);
    assert_eq!(first.allocation.address, 0x20_008);
    assert_eq!(first.state.cursor, 16);

    let second_execution = execute_registered_alloc_capsule(first.state, 4096, 4096);
    assert!(second_execution.accepted);
    let second = second_execution.step;
    assert!(second.allocation.ok);
    assert_eq!(second.allocation.address, 0x21_000);
    assert_eq!(second.state.cursor, 8192);

    let abi_execution = execute_registered_global_alloc_adapter(second.state, 24, 8);
    assert!(abi_execution.accepted);
    let through_abi = abi_execution.step;
    assert!(through_abi.allocation.ok);
    assert_eq!(through_abi.allocation.size, 24);
    assert_eq!(through_abi.allocation.align, 8);
    assert_eq!(execute_global_alloc_null_path(), 0);

    let bad_alignment_execution = execute_registered_alloc_capsule(through_abi.state, 1, 128);
    assert!(bad_alignment_execution.accepted);
    let bad_alignment = bad_alignment_execution.step;
    assert!(!bad_alignment.allocation.ok);
    assert_eq!(bad_alignment.state.cursor, 8216);

    let exhausted_execution = execute_registered_alloc_capsule(bad_alignment.state, 65_536, 1);
    assert!(exhausted_execution.accepted);
    let exhausted = exhausted_execution.step;
    assert!(!exhausted.allocation.ok);
    assert_eq!(exhausted.state.cursor, 8216);

    let seal_execution = execute_registered_seal_capsule(exhausted.state);
    assert!(seal_execution.accepted);
    let sealed = seal_execution.state;
    assert!(sealed.sealed);
    let after_seal_execution = execute_registered_alloc_capsule(sealed, 1, 1);
    assert!(after_seal_execution.accepted);
    let after_seal = after_seal_execution.step;
    assert!(!after_seal.allocation.ok);
    assert_eq!(after_seal.state.cursor, 8216);

    let copied = execute_registered_memcpy_observation(100, 200, 8, 103, 0xaa, 0x5c, true);
    assert!(copied.accepted);
    assert_eq!(copied.observation.return_address, 100);
    assert_eq!(copied.observation.byte, 0x5c);
    let untouched = execute_registered_memcpy_observation(100, 200, 8, 99, 0xaa, 0x5c, true);
    assert!(untouched.accepted);
    assert_eq!(untouched.observation.byte, 0xaa);

    let set = execute_registered_memset_observation(300, 0x6d, 4, 302, 0xaa, true);
    assert!(set.accepted);
    assert_eq!(set.observation.return_address, 300);
    assert_eq!(set.observation.byte, 0x6d);
    let unset = execute_registered_memset_observation(300, 0x6d, 4, 304, 0xaa, true);
    assert!(unset.accepted);
    assert_eq!(unset.observation.byte, 0xaa);

    let rejected_image = decode_execute_seal_capsule(
        PrimitiveCapsuleImage { word0: 0, word1: 0 },
        BootArenaState {
            base: 0x20_000,
            cursor: 0,
            sealed: false,
        },
    );
    assert!(!rejected_image.accepted);
    assert!(!rejected_image.state.sealed);

    println!("M0_PLATFORM_PRIMITIVES_OK:111:12:9:14:20008:21000:sealed");
}
