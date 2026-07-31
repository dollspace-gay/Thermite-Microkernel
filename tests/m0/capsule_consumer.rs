extern crate tmk_capsule;

use tmk_capsule::{decode_execute, execute_registered, registered_word, MachineState};

fn main() {
    let word = registered_word();
    let bytes = word.to_le_bytes();
    assert_eq!(bytes, [0x48, 0x89, 0xf8, 0xf4]);

    let mut args = std::env::args_os().skip(1);
    if let Some(path) = args.next() {
        assert!(args.next().is_none(), "expected at most one capsule output path");
        std::fs::write(path, bytes).expect("write verified capsule bytes");
    }

    let state = MachineState {
        rax: 0,
        rdi: 0x5aa5_1234_8765_cdef,
        rip: 0x1000,
        halted: false,
    };
    let executed = execute_registered(state);
    assert!(executed.accepted);
    assert_eq!(executed.state.rax, 0x5aa5_1234_8765_cdef);
    assert_eq!(executed.state.rdi, 0x5aa5_1234_8765_cdef);
    assert_eq!(executed.state.rip, 0x1004);
    assert!(executed.state.halted);

    let malformed = decode_execute(
        0xf4f88949,
        MachineState {
            rax: 7,
            rdi: 9,
            rip: 0x2000,
            halted: false,
        },
    );
    assert!(!malformed.accepted);
    assert_eq!((malformed.state.rax, malformed.state.rip), (7, 0x2000));

    let overflow = execute_registered(MachineState {
        rax: 11,
        rdi: 13,
        rip: u64::MAX - 3,
        halted: false,
    });
    assert!(!overflow.accepted);
    assert_eq!((overflow.state.rax, overflow.state.rip), (11, u64::MAX - 3));
    println!("M0_CAPSULE_OK:4889f8f4:5aa512348765cdef:1004");
}
