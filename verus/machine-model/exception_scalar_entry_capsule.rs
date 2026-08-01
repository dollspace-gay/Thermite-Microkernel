#![no_std]
#![crate_type = "rlib"]

use verus_builtin::*;
use verus_builtin_macros::*;

verus! {

pub const SCALAR_ENTRY_VIRTUAL: u64 = 0xffff_ffff_8001_1200;
pub const SCALAR_CORE_VIRTUAL: u64 = 0xffff_ffff_8001_1300;
pub const COMMON_CONTINUATION: u64 = 0xffff_ffff_8001_1038;
pub const CONTROL_RETURN: u8 = 0;
pub const CONTROL_SCHEDULE: u8 = 1;
pub const CONTROL_FAIL_STOP: u8 = 2;
pub const REGISTERED_QWORD: u64 = 0x0000_00f8_e9df_8948;

pub struct CapsuleImage {
    pub qword: u64,
}

pub struct MachineState {
    pub cpl: u8,
    pub interrupts_enabled: bool,
    pub direction_flag: bool,
    pub rdi_cr2: u64,
    pub rsi_error: u64,
    pub rdx_rip: u64,
    pub rcx_rflags: u64,
    pub r8_user_rsp: u64,
    pub r9_metadata: u64,
    pub rbx_frame: u64,
    pub r10_frame: u64,
    pub rsp: u64,
    pub return_address: u64,
    pub frame_registered: bool,
    pub return_address_readable: bool,
    pub core_registered: bool,
    pub core_control: u8,
    pub core_preserves_rbx: bool,
    pub core_preserves_frame: bool,
}

pub struct CoreArguments {
    pub frame: u64,
    pub error: u64,
    pub rip: u64,
    pub rflags: u64,
    pub user_rsp: u64,
    pub metadata: u64,
}

pub struct ScalarEntryStep {
    pub accepted: bool,
    pub arguments: CoreArguments,
    pub discarded_redundant_cr2: u64,
    pub core_address: u64,
    pub core_entry_rsp: u64,
    pub stack_neutral_tail_jump: bool,
    pub frame_unchanged: bool,
    pub rbx_preserved: bool,
    pub returns_to_common: bool,
    pub schedules: bool,
    pub fail_stops: bool,
    pub post_rsp: u64,
    pub post_rip: u64,
}

pub open spec fn image_registered(image: &CapsuleImage) -> bool {
    image.qword == REGISTERED_QWORD
}

pub open spec fn scalar_entry_precondition(state: &MachineState) -> bool {
    state.cpl == 0
        && !state.interrupts_enabled
        && !state.direction_flag
        && state.rbx_frame == state.r10_frame
        && state.rbx_frame >= 0xffff_8000_0000_0000
        && state.rsp >= 0xffff_8000_0000_0000
        && state.rsp & 15 == 8
        && state.rsp <= state.rbx_frame - 8
        && state.return_address == COMMON_CONTINUATION
        && state.frame_registered
        && state.return_address_readable
        && state.core_registered
        && state.core_control <= CONTROL_FAIL_STOP
        && (state.core_control != CONTROL_RETURN
            || state.core_preserves_rbx && state.core_preserves_frame)
}

pub fn registered_image() -> (result: CapsuleImage)
    ensures image_registered(&result),
{
    CapsuleImage { qword: REGISTERED_QWORD }
}

pub fn decode_execute(image: CapsuleImage, state: MachineState) -> (result: ScalarEntryStep)
    ensures
        result.accepted <==> image_registered(&image) && scalar_entry_precondition(&state),
        result.accepted ==> result.arguments.frame == state.rbx_frame,
        result.accepted ==> result.arguments.error == state.rsi_error,
        result.accepted ==> result.arguments.rip == state.rdx_rip,
        result.accepted ==> result.arguments.rflags == state.rcx_rflags,
        result.accepted ==> result.arguments.user_rsp == state.r8_user_rsp,
        result.accepted ==> result.arguments.metadata == state.r9_metadata,
        result.accepted ==> result.discarded_redundant_cr2 == state.rdi_cr2,
        result.accepted ==> result.core_address == SCALAR_CORE_VIRTUAL,
        result.accepted ==> result.core_entry_rsp == state.rsp,
        result.accepted ==> result.core_entry_rsp & 15 == 8,
        result.accepted ==> result.stack_neutral_tail_jump,
        result.accepted ==> result.frame_unchanged,
        result.accepted ==> result.rbx_preserved,
        result.accepted ==> result.returns_to_common == (state.core_control == CONTROL_RETURN),
        result.accepted ==> result.schedules == (state.core_control == CONTROL_SCHEDULE),
        result.accepted ==> result.fail_stops == (state.core_control == CONTROL_FAIL_STOP),
        result.accepted && result.returns_to_common ==> result.post_rsp == state.rsp + 8,
        result.accepted && result.returns_to_common ==> result.post_rip == COMMON_CONTINUATION,
        result.accepted && !result.returns_to_common ==> result.post_rsp == 0,
        result.accepted && !result.returns_to_common ==> result.post_rip == 0,
        !result.accepted ==> result.arguments.frame == 0,
        !result.accepted ==> result.arguments.error == 0,
        !result.accepted ==> result.arguments.rip == 0,
        !result.accepted ==> result.arguments.rflags == 0,
        !result.accepted ==> result.arguments.user_rsp == 0,
        !result.accepted ==> result.arguments.metadata == 0,
        !result.accepted ==> result.discarded_redundant_cr2 == 0,
        !result.accepted ==> result.core_address == 0,
        !result.accepted ==> result.core_entry_rsp == 0,
        !result.accepted ==> !result.stack_neutral_tail_jump,
        !result.accepted ==> !result.frame_unchanged,
        !result.accepted ==> !result.rbx_preserved,
        !result.accepted ==> !result.returns_to_common,
        !result.accepted ==> !result.schedules,
        !result.accepted ==> !result.fail_stops,
        !result.accepted ==> result.post_rsp == 0,
        !result.accepted ==> result.post_rip == 0,
{
    if image.qword == REGISTERED_QWORD
        && state.cpl == 0
        && !state.interrupts_enabled
        && !state.direction_flag
        && state.rbx_frame == state.r10_frame
        && state.rbx_frame >= 0xffff_8000_0000_0000
        && state.rsp >= 0xffff_8000_0000_0000
        && state.rsp & 15 == 8
        && state.rsp <= state.rbx_frame - 8
        && state.return_address == COMMON_CONTINUATION
        && state.frame_registered
        && state.return_address_readable
        && state.core_registered
        && state.core_control <= CONTROL_FAIL_STOP
        && (state.core_control != CONTROL_RETURN
            || state.core_preserves_rbx && state.core_preserves_frame)
    {
        let returns = state.core_control == CONTROL_RETURN;
        ScalarEntryStep {
            accepted: true,
            arguments: CoreArguments {
                frame: state.rbx_frame,
                error: state.rsi_error,
                rip: state.rdx_rip,
                rflags: state.rcx_rflags,
                user_rsp: state.r8_user_rsp,
                metadata: state.r9_metadata,
            },
            discarded_redundant_cr2: state.rdi_cr2,
            core_address: SCALAR_CORE_VIRTUAL,
            core_entry_rsp: state.rsp,
            stack_neutral_tail_jump: true,
            frame_unchanged: true,
            rbx_preserved: true,
            returns_to_common: returns,
            schedules: state.core_control == CONTROL_SCHEDULE,
            fail_stops: state.core_control == CONTROL_FAIL_STOP,
            post_rsp: if returns { state.rsp + 8 } else { 0 },
            post_rip: if returns { COMMON_CONTINUATION } else { 0 },
        }
    } else {
        ScalarEntryStep {
            accepted: false,
            arguments: CoreArguments {
                frame: 0, error: 0, rip: 0, rflags: 0, user_rsp: 0, metadata: 0,
            },
            discarded_redundant_cr2: 0,
            core_address: 0,
            core_entry_rsp: 0,
            stack_neutral_tail_jump: false,
            frame_unchanged: false,
            rbx_preserved: false,
            returns_to_common: false,
            schedules: false,
            fail_stops: false,
            post_rsp: 0,
            post_rip: 0,
        }
    }
}

pub fn scalar_entry_observation() -> (result: u64)
    ensures result == 511,
{
    let state = MachineState {
        cpl: 0,
        interrupts_enabled: false,
        direction_flag: false,
        rdi_cr2: 0x1234_5000,
        rsi_error: 6,
        rdx_rip: 0x0040_1000,
        rcx_rflags: 0x202,
        r8_user_rsp: 0x0000_7fff_ffff_e000,
        r9_metadata: 0x001b_0023_0000_000e,
        rbx_frame: 0xffff_e000_0000_2e80,
        r10_frame: 0xffff_e000_0000_2e80,
        rsp: 0xffff_e000_0000_2e78,
        return_address: COMMON_CONTINUATION,
        frame_registered: true,
        return_address_readable: true,
        core_registered: true,
        core_control: CONTROL_RETURN,
        core_preserves_rbx: true,
        core_preserves_frame: true,
    };
    assert(0xffff_e000_0000_2e78u64 & 15 == 8) by(bit_vector);
    assert(scalar_entry_precondition(&state));
    let step = decode_execute(registered_image(), state);
    assert(step.accepted);
    assert(step.arguments.frame == 0xffff_e000_0000_2e80);
    assert(step.arguments.metadata == 0x001b_0023_0000_000e);
    assert(step.discarded_redundant_cr2 == 0x1234_5000);
    assert(step.core_address == SCALAR_CORE_VIRTUAL);
    assert(step.stack_neutral_tail_jump);
    assert(step.returns_to_common && !step.schedules && !step.fail_stops);
    assert(step.post_rsp == 0xffff_e000_0000_2e80);
    assert(step.post_rip == COMMON_CONTINUATION);
    511
}

}
