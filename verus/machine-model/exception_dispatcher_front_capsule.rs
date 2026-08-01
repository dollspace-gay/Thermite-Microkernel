#![no_std]
#![crate_type = "rlib"]

use verus_builtin::*;
use verus_builtin_macros::*;

verus! {

pub const DISPATCHER_VIRTUAL: u64 = 0xffff_ffff_8001_1100;
pub const SCALAR_SEAM_VIRTUAL: u64 = 0xffff_ffff_8001_1200;
pub const KERNEL_CODE_SELECTOR: u64 = 0x08;
pub const USER_CODE_SELECTOR: u64 = 0x23;

pub const QWORD0: u64 = 0x4970_7a8b_49fa_8949;
pub const QWORD1: u64 = 0x8b49_0000_0088_b28b;
pub const QWORD2: u64 = 0x8a8b_4900_0000_9092;
pub const QWORD3: u64 = 0x808a_8b4d_0000_00a0;
pub const QWORD4: u64 = 0x0098_9a8b_4d00_0000;
pub const QWORD5: u64 = 0x1e74_03c3_f641_0000;
pub const QWORD6: u64 = 0x4900_0000_a882_8b4d;
pub const QWORD7: u64 = 0xc148_0000_00b0_828b;
pub const QWORD8: u64 = 0x094d_20e3_c149_30e0;
pub const QWORD9: u64 = 0x3145_0aeb_c109_49d9;
pub const QWORD10: u64 = 0xd909_4d20_e3c1_49c0;
pub const TAIL: u64 = 0x0000_0000_00a3_e9;

pub struct CapsuleImage {
    pub qword0: u64,
    pub qword1: u64,
    pub qword2: u64,
    pub qword3: u64,
    pub qword4: u64,
    pub qword5: u64,
    pub qword6: u64,
    pub qword7: u64,
    pub qword8: u64,
    pub qword9: u64,
    pub qword10: u64,
    pub tail: u64,
}

pub struct SavedFrameMemory {
    pub base: u64,
    pub cr2: u64,
    pub vector: u64,
    pub error: u64,
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub user_rsp: u64,
    pub user_ss: u64,
}

pub struct MachineState {
    pub cpl: u8,
    pub interrupts_enabled: bool,
    pub direction_flag: bool,
    pub rdi: u64,
    pub rbx: u64,
    pub rsp: u64,
    pub return_address: u64,
    pub frame: SavedFrameMemory,
    pub prefix_readable: bool,
    pub user_tail_readable: bool,
    pub scalar_return_address_readable: bool,
    pub scalar_registered: bool,
    pub scalar_returns: bool,
    pub scalar_preserves_rbx: bool,
    pub scalar_preserves_frame: bool,
}

pub struct ScalarArguments {
    pub cr2: u64,
    pub error: u64,
    pub rip: u64,
    pub rflags: u64,
    pub user_rsp: u64,
    pub metadata: u64,
}

pub struct DispatcherFrontStep {
    pub accepted: bool,
    pub arguments: ScalarArguments,
    pub scalar_address: u64,
    pub frame_words_read: u8,
    pub frame_memory_unchanged: bool,
    pub post_rbx: u64,
    pub post_rsp: u64,
    pub post_rip: u64,
    pub scalar_entry_rsp: u64,
    pub scalar_stack_aligned: bool,
    pub scalar_tail_transfer: bool,
    pub return_address_consumed: bool,
}

pub open spec fn image_registered(image: &CapsuleImage) -> bool {
    image.qword0 == QWORD0
        && image.qword1 == QWORD1
        && image.qword2 == QWORD2
        && image.qword3 == QWORD3
        && image.qword4 == QWORD4
        && image.qword5 == QWORD5
        && image.qword6 == QWORD6
        && image.qword7 == QWORD7
        && image.qword8 == QWORD8
        && image.qword9 == QWORD9
        && image.qword10 == QWORD10
        && image.tail == TAIL
}

pub open spec fn from_user(frame: &SavedFrameMemory) -> bool {
    frame.cs & 3 == 3
}

pub open spec fn packed_metadata(frame: &SavedFrameMemory) -> u64 {
    frame.vector | (frame.cs << 32)
        | if from_user(frame) { frame.user_ss << 48 } else { 0u64 }
}

pub open spec fn dispatcher_front_precondition(state: &MachineState) -> bool {
    state.cpl == 0
        && !state.interrupts_enabled
        && !state.direction_flag
        && state.rdi == state.frame.base
        && state.frame.base >= 0xffff_8000_0000_0000
        && state.frame.base <= 0xffff_ffff_ffff_ff48
        && state.frame.vector <= 255
        && (state.frame.cs == KERNEL_CODE_SELECTOR || state.frame.cs == USER_CODE_SELECTOR)
        && (from_user(&state.frame) ==> state.frame.user_ss <= 0xffff)
        && state.prefix_readable
        && (from_user(&state.frame) ==> state.user_tail_readable)
        && state.rsp >= 0xffff_8000_0000_0000
        && state.rsp & 15 == 8
        && state.rsp <= state.frame.base - 8
        && state.return_address == 0xffff_ffff_8001_1038
        && state.scalar_return_address_readable
        && state.scalar_registered
        && state.scalar_returns
        && state.scalar_preserves_rbx
        && state.scalar_preserves_frame
}

pub fn registered_image() -> (result: CapsuleImage)
    ensures image_registered(&result),
{
    CapsuleImage {
        qword0: QWORD0,
        qword1: QWORD1,
        qword2: QWORD2,
        qword3: QWORD3,
        qword4: QWORD4,
        qword5: QWORD5,
        qword6: QWORD6,
        qword7: QWORD7,
        qword8: QWORD8,
        qword9: QWORD9,
        qword10: QWORD10,
        tail: TAIL,
    }
}

pub fn decode_execute(image: CapsuleImage, state: MachineState) -> (result: DispatcherFrontStep)
    ensures
        result.accepted <==> image_registered(&image) && dispatcher_front_precondition(&state),
        result.accepted ==> result.arguments.cr2 == state.frame.cr2,
        result.accepted ==> result.arguments.error == state.frame.error,
        result.accepted ==> result.arguments.rip == state.frame.rip,
        result.accepted ==> result.arguments.rflags == state.frame.rflags,
        result.accepted ==> result.arguments.user_rsp == if from_user(&state.frame) {
            state.frame.user_rsp
        } else {
            0u64
        },
        result.accepted ==> result.arguments.metadata == packed_metadata(&state.frame),
        result.accepted ==> result.scalar_address == SCALAR_SEAM_VIRTUAL,
        result.accepted ==> result.frame_words_read == if from_user(&state.frame) { 8u8 } else { 6u8 },
        result.accepted ==> result.frame_memory_unchanged,
        result.accepted ==> result.post_rbx == state.rbx,
        result.accepted ==> result.post_rsp == state.rsp + 8,
        result.accepted ==> result.post_rip == state.return_address,
        result.accepted ==> result.scalar_entry_rsp == state.rsp,
        result.accepted ==> result.scalar_entry_rsp & 15 == 8,
        result.accepted ==> result.scalar_stack_aligned,
        result.accepted ==> result.scalar_tail_transfer,
        result.accepted ==> result.return_address_consumed,
        !result.accepted ==> result.arguments.cr2 == 0,
        !result.accepted ==> result.arguments.error == 0,
        !result.accepted ==> result.arguments.rip == 0,
        !result.accepted ==> result.arguments.rflags == 0,
        !result.accepted ==> result.arguments.user_rsp == 0,
        !result.accepted ==> result.arguments.metadata == 0,
        !result.accepted ==> result.scalar_address == 0,
        !result.accepted ==> result.frame_words_read == 0,
        !result.accepted ==> !result.frame_memory_unchanged,
        !result.accepted ==> result.post_rbx == state.rbx,
        !result.accepted ==> result.post_rsp == state.rsp,
        !result.accepted ==> result.post_rip == 0,
        !result.accepted ==> result.scalar_entry_rsp == 0,
        !result.accepted ==> !result.scalar_stack_aligned,
        !result.accepted ==> !result.scalar_tail_transfer,
        !result.accepted ==> !result.return_address_consumed,
{
    if image.qword0 == QWORD0
        && image.qword1 == QWORD1
        && image.qword2 == QWORD2
        && image.qword3 == QWORD3
        && image.qword4 == QWORD4
        && image.qword5 == QWORD5
        && image.qword6 == QWORD6
        && image.qword7 == QWORD7
        && image.qword8 == QWORD8
        && image.qword9 == QWORD9
        && image.qword10 == QWORD10
        && image.tail == TAIL
        && state.cpl == 0
        && !state.interrupts_enabled
        && !state.direction_flag
        && state.rdi == state.frame.base
        && state.frame.base >= 0xffff_8000_0000_0000
        && state.frame.base <= 0xffff_ffff_ffff_ff48
        && state.frame.vector <= 255
        && (state.frame.cs == KERNEL_CODE_SELECTOR || state.frame.cs == USER_CODE_SELECTOR)
        && (!(state.frame.cs & 3 == 3) || state.frame.user_ss <= 0xffff)
        && state.prefix_readable
        && (!(state.frame.cs & 3 == 3) || state.user_tail_readable)
        && state.rsp >= 0xffff_8000_0000_0000
        && state.rsp & 15 == 8
        && state.rsp <= state.frame.base - 8
        && state.return_address == 0xffff_ffff_8001_1038
        && state.scalar_return_address_readable
        && state.scalar_registered
        && state.scalar_returns
        && state.scalar_preserves_rbx
        && state.scalar_preserves_frame
    {
        let user = state.frame.cs & 3 == 3;
        let metadata = state.frame.vector | (state.frame.cs << 32)
            | if user { state.frame.user_ss << 48 } else { 0 };
        DispatcherFrontStep {
            accepted: true,
            arguments: ScalarArguments {
                cr2: state.frame.cr2,
                error: state.frame.error,
                rip: state.frame.rip,
                rflags: state.frame.rflags,
                user_rsp: if user { state.frame.user_rsp } else { 0 },
                metadata,
            },
            scalar_address: SCALAR_SEAM_VIRTUAL,
            frame_words_read: if user { 8 } else { 6 },
            frame_memory_unchanged: true,
            post_rbx: state.rbx,
            post_rsp: state.rsp + 8,
            post_rip: state.return_address,
            scalar_entry_rsp: state.rsp,
            scalar_stack_aligned: true,
            scalar_tail_transfer: true,
            return_address_consumed: true,
        }
    } else {
        DispatcherFrontStep {
            accepted: false,
            arguments: ScalarArguments {
                cr2: 0,
                error: 0,
                rip: 0,
                rflags: 0,
                user_rsp: 0,
                metadata: 0,
            },
            scalar_address: 0,
            frame_words_read: 0,
            frame_memory_unchanged: false,
            post_rbx: state.rbx,
            post_rsp: state.rsp,
            post_rip: 0,
            scalar_entry_rsp: 0,
            scalar_stack_aligned: false,
            scalar_tail_transfer: false,
            return_address_consumed: false,
        }
    }
}

pub fn dispatcher_front_observation() -> (result: u64)
    ensures result == 1023,
{
    assert((USER_CODE_SELECTOR & 3u64) == 3u64) by(bit_vector);
    assert(14u64 | (USER_CODE_SELECTOR << 32) | (0x1bu64 << 48)
        == 0x001b_0023_0000_000e) by(bit_vector);
    let state = MachineState {
        cpl: 0,
        interrupts_enabled: false,
        direction_flag: false,
        rdi: 0xffff_e000_0000_2e80,
        rbx: 0xbbbb_bbbb_bbbb_bbbb,
        rsp: 0xffff_e000_0000_2e78,
        return_address: 0xffff_ffff_8001_1038,
        frame: SavedFrameMemory {
            base: 0xffff_e000_0000_2e80,
            cr2: 0x1234_5000,
            vector: 14,
            error: 6,
            rip: 0x0040_1000,
            cs: USER_CODE_SELECTOR,
            rflags: 0x202,
            user_rsp: 0x0000_7fff_ffff_e000,
            user_ss: 0x1b,
        },
        prefix_readable: true,
        user_tail_readable: true,
        scalar_return_address_readable: true,
        scalar_registered: true,
        scalar_returns: true,
        scalar_preserves_rbx: true,
        scalar_preserves_frame: true,
    };
    assert(0xffff_e000_0000_2e78u64 & 15 == 8) by(bit_vector);
    assert(dispatcher_front_precondition(&state));
    let step = decode_execute(registered_image(), state);
    assert(step.accepted);
    assert(step.arguments.cr2 == 0x1234_5000);
    assert(step.arguments.error == 6);
    assert(step.arguments.rip == 0x0040_1000);
    assert(step.arguments.rflags == 0x202);
    assert(step.arguments.user_rsp == 0x0000_7fff_ffff_e000);
    assert(step.arguments.metadata == 0x001b_0023_0000_000e);
    assert(step.scalar_address == SCALAR_SEAM_VIRTUAL);
    assert(step.frame_words_read == 8);
    assert(step.frame_memory_unchanged);
    assert(step.post_rbx == 0xbbbb_bbbb_bbbb_bbbb);
    assert(step.post_rsp == 0xffff_e000_0000_2e80);
    assert(step.post_rip == 0xffff_ffff_8001_1038);
    assert(step.scalar_entry_rsp & 15 == 8);
    assert(step.scalar_stack_aligned);
    assert(step.scalar_tail_transfer);
    assert(step.return_address_consumed);
    1023
}

}
