extern crate tmk_uefi_capsule;

use tmk_uefi_capsule::{
    decode_execute, entry_code, execute_registered, UefiEntryCode, UefiMachineState,
};

const MARKER: &[u8; 16] = b"TMK_M0_UEFI_OK!\n";

fn main() {
    let mut args = std::env::args_os().skip(1);
    let output = args.next().expect("expected capsule output path");
    assert!(args.next().is_none(), "expected exactly one capsule output path");

    let code = entry_code();
    let words = [
        code.word0,
        code.word1,
        code.word2,
        code.word3,
        code.word4,
        code.word5,
        code.word6,
    ];
    let mut bytes = [0u8; 56];
    for (index, word) in words.into_iter().enumerate() {
        bytes[index * 8..(index + 1) * 8].copy_from_slice(&word.to_le_bytes());
    }
    assert_eq!(&bytes[0..4], &[0x66, 0xba, 0xe9, 0x00]);
    for (index, marker_byte) in MARKER.iter().copied().enumerate() {
        let offset = 4 + index * 3;
        assert_eq!(&bytes[offset..offset + 3], &[0xb0, marker_byte, 0xee]);
    }
    assert_eq!(&bytes[52..56], &[0x31, 0xc0, 0xc3, 0xcc]);
    std::fs::write(output, bytes).expect("write verified UEFI entry bytes");

    let state = UefiMachineState {
        rax: u64::MAX,
        rbx: 0x1122,
        rcx: 0x3344,
        rdx: 0x5566,
        rsp: 0x8000,
        write0: 0,
        write1: 0,
        returned: false,
    };
    let executed = execute_registered(state);
    assert!(executed.accepted);
    assert_eq!(executed.state.rax, 0);
    assert_eq!(executed.state.rbx, 0x1122);
    assert_eq!(executed.state.rcx, 0x3344);
    assert_eq!(executed.state.rdx, 0xe9);
    assert_eq!(executed.state.rsp, 0x8000);
    assert_eq!(executed.state.write0.to_le_bytes(), MARKER[0..8]);
    assert_eq!(executed.state.write1.to_le_bytes(), MARKER[8..16]);
    assert!(executed.state.returned);

    let rejected = decode_execute(
        UefiEntryCode {
            word0: code.word0 ^ 1,
            word1: code.word1,
            word2: code.word2,
            word3: code.word3,
            word4: code.word4,
            word5: code.word5,
            word6: code.word6,
        },
        UefiMachineState {
            rax: 7,
            rbx: 11,
            rcx: 13,
            rdx: 17,
            rsp: 19,
            write0: 23,
            write1: 29,
            returned: false,
        },
    );
    assert!(!rejected.accepted);
    assert_eq!(rejected.state.rax, 7);
    assert_eq!(rejected.state.rdx, 17);
    assert!(!rejected.state.returned);
    println!("M0_UEFI_CAPSULE_OK:56:00e9:0000000000000000");
}
