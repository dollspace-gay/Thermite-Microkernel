extern crate tmk_descriptor_install_capsule;

use tmk_descriptor_install_capsule::{
    CapsuleImage, GDT_LIMIT, IDT_LIMIT, KERNEL_CODE_SELECTOR, KERNEL_DATA_SELECTOR, MachineState,
    REGISTERED_DWORD4, REGISTERED_QWORD0, REGISTERED_QWORD1, REGISTERED_QWORD2,
    REGISTERED_QWORD3, REGISTERED_WORD5, TSS_SELECTOR, decode_execute, install_registered_tables,
    registered_image, registered_install_observation,
};

fn state() -> MachineState {
    MachineState {
        rax: 0x55aa,
        rdi: 0xffff_e000_0000_7fe0,
        rsi: 0xffff_e000_0000_7ff0,
        rsp: 0xffff_e000_0000_1000,
        rip: 0xffff_ffff_8000_1010,
        rflags: 0x2,
        cpl: 0,
        interrupts_enabled: false,
        asynchronous_events_absent: true,
        gdtr_pointer_readable: true,
        idtr_pointer_readable: true,
        far_stack_writable: true,
        return_stack_readable: true,
        gdt_image_registered: true,
        idt_image_registered: true,
        tss_descriptor_writable: true,
        tss_descriptor_busy: false,
        gdtr_operand_base: 0xffff_e000_0000_8000,
        gdtr_operand_limit: GDT_LIMIT,
        idtr_operand_base: 0xffff_e000_0000_a000,
        idtr_operand_limit: IDT_LIMIT,
        gdtr_base: 0,
        gdtr_limit: 0,
        idtr_base: 0,
        idtr_limit: 0,
        cs: 0x38,
        ds: 0x30,
        es: 0x30,
        ss: 0x30,
        tr: 0,
        return_address: 0xffff_ffff_8000_2000,
    }
}

fn malformed_image() -> CapsuleImage {
    CapsuleImage {
        qword0: REGISTERED_QWORD0 ^ 1,
        qword1: REGISTERED_QWORD1,
        qword2: REGISTERED_QWORD2,
        qword3: REGISTERED_QWORD3,
        dword4: REGISTERED_DWORD4,
        word5: REGISTERED_WORD5,
    }
}

fn main() {
    let image = registered_image();
    let mut bytes = Vec::with_capacity(38);
    bytes.extend_from_slice(&image.qword0.to_le_bytes());
    bytes.extend_from_slice(&image.qword1.to_le_bytes());
    bytes.extend_from_slice(&image.qword2.to_le_bytes());
    bytes.extend_from_slice(&image.qword3.to_le_bytes());
    bytes.extend_from_slice(&image.dword4.to_le_bytes());
    bytes.extend_from_slice(&image.word5.to_le_bytes());
    assert_eq!(bytes.len(), 38);
    assert_eq!(
        bytes,
        [
            0x0f, 0x01, 0x17, 0xb8, 0x10, 0x00, 0x00, 0x00, 0x8e, 0xd8, 0x8e, 0xc0, 0x8e,
            0xd0, 0x6a, 0x08, 0x48, 0x8d, 0x05, 0x03, 0x00, 0x00, 0x00, 0x50, 0x48, 0xcb,
            0xb8, 0x28, 0x00, 0x00, 0x00, 0x0f, 0x00, 0xd8, 0x0f, 0x01, 0x1e, 0xc3,
        ]
    );

    let mut args = std::env::args_os().skip(1);
    if let Some(path) = args.next() {
        assert!(args.next().is_none(), "expected at most one capsule output path");
        std::fs::write(path, &bytes).expect("write verified descriptor-install capsule bytes");
    }

    let installed = install_registered_tables(state());
    assert!(installed.accepted);
    assert_eq!(installed.state.rax, TSS_SELECTOR as u64);
    assert_eq!(installed.state.rdi, 0xffff_e000_0000_7fe0);
    assert_eq!(installed.state.rsi, 0xffff_e000_0000_7ff0);
    assert_eq!(installed.state.rsp, 0xffff_e000_0000_1008);
    assert_eq!(installed.state.rip, 0xffff_ffff_8000_2000);
    assert_eq!(installed.state.rflags, 0x2);
    assert_eq!(installed.state.gdtr_base, 0xffff_e000_0000_8000);
    assert_eq!(installed.state.gdtr_limit, GDT_LIMIT);
    assert_eq!(installed.state.idtr_base, 0xffff_e000_0000_a000);
    assert_eq!(installed.state.idtr_limit, IDT_LIMIT);
    assert_eq!(installed.state.cs, KERNEL_CODE_SELECTOR);
    assert_eq!(installed.state.ds, KERNEL_DATA_SELECTOR);
    assert_eq!(installed.state.es, KERNEL_DATA_SELECTOR);
    assert_eq!(installed.state.ss, KERNEL_DATA_SELECTOR);
    assert_eq!(installed.state.tr, TSS_SELECTOR);
    assert!(installed.state.tss_descriptor_busy);
    assert_eq!(registered_install_observation(), 255);

    let malformed = decode_execute(malformed_image(), state());
    assert!(!malformed.accepted);
    assert_eq!(malformed.state.gdtr_base, 0);
    assert_eq!(malformed.state.rsp, 0xffff_e000_0000_1000);

    let ring_three = decode_execute(registered_image(), MachineState { cpl: 3, ..state() });
    assert!(!ring_three.accepted);
    let interrupts = decode_execute(
        registered_image(),
        MachineState {
            interrupts_enabled: true,
            ..state()
        },
    );
    assert!(!interrupts.accepted);
    let asynchronous = decode_execute(
        registered_image(),
        MachineState {
            asynchronous_events_absent: false,
            ..state()
        },
    );
    assert!(!asynchronous.accepted);
    let bad_gdt = decode_execute(
        registered_image(),
        MachineState {
            gdt_image_registered: false,
            ..state()
        },
    );
    assert!(!bad_gdt.accepted);
    let bad_idt_limit = decode_execute(
        registered_image(),
        MachineState {
            idtr_operand_limit: IDT_LIMIT - 1,
            ..state()
        },
    );
    assert!(!bad_idt_limit.accepted);
    let busy_tss = decode_execute(
        registered_image(),
        MachineState {
            tss_descriptor_busy: true,
            ..state()
        },
    );
    assert!(!busy_tss.accepted);
    let readonly_tss = decode_execute(
        registered_image(),
        MachineState {
            tss_descriptor_writable: false,
            ..state()
        },
    );
    assert!(!readonly_tss.accepted);
    let bad_stack = decode_execute(
        registered_image(),
        MachineState {
            rsp: 8,
            ..state()
        },
    );
    assert!(!bad_stack.accepted);
    let noncanonical_return = decode_execute(
        registered_image(),
        MachineState {
            return_address: 0x0000_8000_0000_0000,
            ..state()
        },
    );
    assert!(!noncanonical_return.accepted);

    println!(
        "M1_DESCRIPTOR_INSTALL_OK bytes=38 cs={:02x} ss={:02x} tr={:02x} rsp={:016x} busy=true",
        installed.state.cs, installed.state.ss, installed.state.tr, installed.state.rsp
    );
}
