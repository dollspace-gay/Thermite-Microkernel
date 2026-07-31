extern crate tmk_cr3_install_capsule;

use tmk_cr3_install_capsule::{
    MachineState, REGISTERED_WORD, ROOT_PHYSICAL, decode_execute, install_registered_root,
    registered_install_observation, registered_word,
};

fn state() -> MachineState {
    MachineState {
        rax: 0x55aa,
        rdi: ROOT_PHYSICAL,
        rsp: 0x2020,
        rip: 0xffff_ffff_8000_1000,
        rflags: 0x202,
        cr3: 0x0010_0000,
        cpl: 0,
        cr4_pcide: false,
        interrupts_enabled: true,
        stack_readable: true,
        return_address: 0xffff_ffff_8000_2000,
        non_global_tlb_valid: true,
    }
}

fn main() {
    let word = registered_word();
    assert_eq!(word, REGISTERED_WORD);
    let bytes = word.to_le_bytes();
    assert_eq!(bytes, [0x0f, 0x22, 0xdf, 0xc3]);

    let mut args = std::env::args_os().skip(1);
    if let Some(path) = args.next() {
        assert!(args.next().is_none(), "expected at most one capsule output path");
        std::fs::write(path, bytes).expect("write verified CR3 capsule bytes");
    }

    let installed = install_registered_root(state());
    assert!(installed.accepted);
    assert_eq!(installed.state.cr3, ROOT_PHYSICAL);
    assert_eq!(installed.state.rsp, 0x2028);
    assert_eq!(installed.state.rip, 0xffff_ffff_8000_2000);
    assert_eq!(installed.state.rax, 0x55aa);
    assert_eq!(installed.state.rdi, ROOT_PHYSICAL);
    assert_eq!(installed.state.rflags, 0x202);
    assert!(installed.state.interrupts_enabled);
    assert!(!installed.state.non_global_tlb_valid);
    assert_eq!(registered_install_observation(), 0x0040_2028);

    let other_root = decode_execute(
        REGISTERED_WORD,
        MachineState {
            rdi: 0x0080_0000,
            ..state()
        },
    );
    assert!(other_root.accepted);
    assert_eq!(other_root.state.cr3, 0x0080_0000);

    let malformed = decode_execute(REGISTERED_WORD ^ 1, state());
    assert!(!malformed.accepted);
    assert_eq!(malformed.state.cr3, 0x0010_0000);
    assert_eq!(malformed.state.rsp, 0x2020);

    let ring_three = decode_execute(
        REGISTERED_WORD,
        MachineState { cpl: 3, ..state() },
    );
    assert!(!ring_three.accepted);
    assert_eq!(ring_three.state.cr3, 0x0010_0000);

    let misaligned = decode_execute(
        REGISTERED_WORD,
        MachineState {
            rdi: ROOT_PHYSICAL + 1,
            ..state()
        },
    );
    assert!(!misaligned.accepted);

    let pcid = decode_execute(
        REGISTERED_WORD,
        MachineState {
            cr4_pcide: true,
            ..state()
        },
    );
    assert!(!pcid.accepted);

    let unreadable_return = decode_execute(
        REGISTERED_WORD,
        MachineState {
            stack_readable: false,
            ..state()
        },
    );
    assert!(!unreadable_return.accepted);

    let noncanonical_return = decode_execute(
        REGISTERED_WORD,
        MachineState {
            return_address: 0x0000_8000_0000_0000,
            ..state()
        },
    );
    assert!(!noncanonical_return.accepted);

    let stack_overflow = decode_execute(
        REGISTERED_WORD,
        MachineState {
            rsp: u64::MAX - 7,
            ..state()
        },
    );
    assert!(!stack_overflow.accepted);

    println!(
        "M1_CR3_CAPSULE_OK bytes=0f22dfc3 cr3={:016x} rsp={:x} invalidated=true",
        installed.state.cr3, installed.state.rsp
    );
}
