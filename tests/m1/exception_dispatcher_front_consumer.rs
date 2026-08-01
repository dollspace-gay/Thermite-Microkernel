extern crate tmk_exception_dispatcher_front;

use std::path::Path;
use tmk_exception_dispatcher_front::{
    decode_execute, dispatcher_front_observation, registered_image, CapsuleImage, MachineState,
    SavedFrameMemory, KERNEL_CODE_SELECTOR, SCALAR_SEAM_VIRTUAL, USER_CODE_SELECTOR,
};

fn write_image(path: &Path, image: &CapsuleImage) {
    let mut bytes = Vec::with_capacity(93);
    for word in [
        image.qword0,
        image.qword1,
        image.qword2,
        image.qword3,
        image.qword4,
        image.qword5,
        image.qword6,
        image.qword7,
        image.qword8,
        image.qword9,
        image.qword10,
    ] {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    bytes.extend_from_slice(&image.tail.to_le_bytes()[..5]);
    assert_eq!(bytes.len(), 93);
    std::fs::write(path, bytes).expect("write dispatcher-front capsule bytes");
}

fn state(user: bool) -> MachineState {
    MachineState {
        cpl: 0,
        interrupts_enabled: false,
        direction_flag: false,
        rdi: 0xffff_e000_0000_2e80,
        rbx: 0xbbbb_bbbb_bbbb_bbbb,
        rsp: 0xffff_e000_0000_2e78,
        return_address: 0xffff_ffff_8001_1038,
        frame: SavedFrameMemory {
            base: 0xffff_e000_0000_2e80,
            cr2: if user { 0x1234_5000 } else { 0 },
            vector: if user { 14 } else { 0xe0 },
            error: if user { 6 } else { 0 },
            rip: if user {
                0x0040_1000
            } else {
                0xffff_ffff_8000_2000
            },
            cs: if user {
                USER_CODE_SELECTOR
            } else {
                KERNEL_CODE_SELECTOR
            },
            rflags: 0x202,
            user_rsp: if user { 0x0000_7fff_ffff_e000 } else { 0 },
            user_ss: if user { 0x1b } else { 0 },
        },
        prefix_readable: true,
        user_tail_readable: user,
        scalar_return_address_readable: true,
        scalar_registered: true,
        scalar_returns: true,
        scalar_preserves_rbx: true,
        scalar_preserves_frame: true,
    }
}

fn main() {
    let output = std::env::args_os()
        .nth(1)
        .expect("dispatcher-front output path");
    let image = registered_image();
    write_image(Path::new(&output), &image);

    assert_eq!(dispatcher_front_observation(), 1023);

    let user = decode_execute(registered_image(), state(true));
    assert!(user.accepted);
    assert_eq!(user.arguments.cr2, 0x1234_5000);
    assert_eq!(user.arguments.error, 6);
    assert_eq!(user.arguments.rip, 0x0040_1000);
    assert_eq!(user.arguments.rflags, 0x202);
    assert_eq!(user.arguments.user_rsp, 0x0000_7fff_ffff_e000);
    assert_eq!(user.arguments.metadata, 0x001b_0023_0000_000e);
    assert_eq!(user.scalar_address, SCALAR_SEAM_VIRTUAL);
    assert_eq!(user.frame_words_read, 8);
    assert!(user.frame_memory_unchanged);
    assert_eq!(user.post_rbx, 0xbbbb_bbbb_bbbb_bbbb);
    assert_eq!(user.post_rsp, 0xffff_e000_0000_2e80);
    assert_eq!(user.post_rip, 0xffff_ffff_8001_1038);
    assert_eq!(user.scalar_entry_rsp & 15, 8);
    assert!(user.scalar_stack_aligned);
    assert!(user.scalar_tail_transfer);
    assert!(user.return_address_consumed);

    let kernel = decode_execute(registered_image(), state(false));
    assert!(kernel.accepted);
    assert_eq!(kernel.arguments.cr2, 0);
    assert_eq!(kernel.arguments.error, 0);
    assert_eq!(kernel.arguments.rip, 0xffff_ffff_8000_2000);
    assert_eq!(kernel.arguments.rflags, 0x202);
    assert_eq!(kernel.arguments.user_rsp, 0);
    assert_eq!(kernel.arguments.metadata, 0x0000_0008_0000_00e0);
    assert_eq!(kernel.frame_words_read, 6);

    let invalid = [
        MachineState {
            prefix_readable: false,
            ..state(false)
        },
        MachineState {
            user_tail_readable: false,
            ..state(true)
        },
        MachineState {
            scalar_registered: false,
            ..state(false)
        },
        MachineState {
            scalar_return_address_readable: false,
            ..state(false)
        },
        MachineState {
            return_address: 0xffff_ffff_8001_1039,
            ..state(false)
        },
        MachineState {
            rdi: 0xffff_e000_0000_3000,
            ..state(false)
        },
        MachineState {
            direction_flag: true,
            ..state(false)
        },
        MachineState {
            frame: SavedFrameMemory {
                vector: 256,
                ..state(false).frame
            },
            ..state(false)
        },
        MachineState {
            frame: SavedFrameMemory {
                user_ss: 0x1_0000,
                ..state(true).frame
            },
            ..state(true)
        },
        MachineState {
            rsp: 0xffff_e000_0000_2e70,
            ..state(false)
        },
        MachineState {
            rsp: 0xffff_e000_0000_2e88,
            ..state(false)
        },
    ];
    for candidate in invalid {
        let rejected = decode_execute(registered_image(), candidate);
        assert!(!rejected.accepted);
        assert_eq!(rejected.scalar_address, 0);
        assert_eq!(rejected.frame_words_read, 0);
    }

    println!(
        "M1_EXCEPTION_DISPATCHER_FRONT_OK bytes=93 user_words={} kernel_words={} metadata={:016x} scalar_entry_mod16={} tail=1",
        user.frame_words_read,
        kernel.frame_words_read,
        user.arguments.metadata,
        user.scalar_entry_rsp & 15,
    );
}
