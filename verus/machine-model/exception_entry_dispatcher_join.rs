#![no_std]
#![crate_type = "rlib"]

use verus_builtin::*;
use verus_builtin_macros::*;

verus! {

pub const COMMON_ENTRY_VIRTUAL: u64 = 0xffff_ffff_8001_1000;
pub const DISPATCHER_VIRTUAL: u64 = 0xffff_ffff_8001_1100;
pub const SCALAR_SEAM_VIRTUAL: u64 = 0xffff_ffff_8001_1200;
pub const COMMON_CONTINUATION: u64 = 0xffff_ffff_8001_1038;
pub const KERNEL_CODE_SELECTOR: u64 = 0x08;
pub const KERNEL_DATA_SELECTOR: u64 = 0x10;
pub const USER_DATA_SELECTOR: u64 = 0x1b;
pub const USER_CODE_SELECTOR: u64 = 0x23;
pub const RETURN_RFLAGS_ALLOWED: u64 = 0x0025_0fd7;

pub const COMMON_QWORD0: u64 = 0x5251_5350_d020_0f50;
pub const COMMON_QWORD12: u64 = 0x4810_c483_4858_08c4;
pub const COMMON_TAIL: u8 = 0xcf;
pub const DISPATCHER_QWORD0: u64 = 0x4970_7a8b_49fa_8949;
pub const DISPATCHER_QWORD10: u64 = 0xd909_4d20_e3c1_49c0;
pub const DISPATCHER_TAIL: u64 = 0x0000_0000_00a3_e9;

pub struct JoinedImageRegistration {
    pub common_first: u64,
    pub common_last: u64,
    pub common_tail: u8,
    pub common_bytes: u16,
    pub dispatcher_first: u64,
    pub dispatcher_last: u64,
    pub dispatcher_tail: u64,
    pub dispatcher_bytes: u16,
    pub common_address: u64,
    pub dispatcher_address: u64,
    pub scalar_address: u64,
}

pub struct EntryState {
    pub cpl: u8,
    pub interrupts_enabled: bool,
    pub direction_flag: bool,
    pub gs_kernel_active: bool,
    pub rsp: u64,
    pub rbx: u64,
    pub cr2: u64,
    pub vector: u64,
    pub error: u64,
    pub resume_rip: u64,
    pub resume_cs: u64,
    pub resume_rflags: u64,
    pub resume_rsp: u64,
    pub resume_ss: u64,
    pub stack_switch: bool,
    pub normalized_frame_registered: bool,
    pub stack_low: u64,
    pub stack_high: u64,
    pub stack_readable: bool,
    pub stack_writable: bool,
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

pub struct JoinedStep {
    pub accepted: bool,
    pub frame_base: u64,
    pub dispatcher_rsp: u64,
    pub return_address: u64,
    pub arguments: ScalarArguments,
    pub frame_words_read: u8,
    pub prefix_readable: bool,
    pub user_tail_readable: bool,
    pub return_address_readable: bool,
    pub dispatcher_precondition_established: bool,
    pub dispatcher_df_clear: bool,
    pub scalar_entry_aligned: bool,
    pub scalar_tail_transfer: bool,
    pub frame_unchanged: bool,
    pub rbx_preserved: bool,
    pub dispatcher_return_rsp: u64,
    pub final_rip: u64,
    pub final_rsp: u64,
    pub final_rflags: u64,
    pub swapgs_count: u8,
}

pub open spec fn image_registered(image: &JoinedImageRegistration) -> bool {
    image.common_first == COMMON_QWORD0
        && image.common_last == COMMON_QWORD12
        && image.common_tail == COMMON_TAIL
        && image.common_bytes == 105
        && image.dispatcher_first == DISPATCHER_QWORD0
        && image.dispatcher_last == DISPATCHER_QWORD10
        && image.dispatcher_tail == DISPATCHER_TAIL
        && image.dispatcher_bytes == 93
        && image.common_address == COMMON_ENTRY_VIRTUAL
        && image.dispatcher_address == DISPATCHER_VIRTUAL
        && image.scalar_address == SCALAR_SEAM_VIRTUAL
}

pub open spec fn canonical(address: u64) -> bool {
    address <= 0x0000_7fff_ffff_ffff || address >= 0xffff_8000_0000_0000
}

pub open spec fn from_user(state: &EntryState) -> bool {
    state.resume_cs & 3 == 3
}

pub open spec fn return_rflags_valid(flags: u64) -> bool {
    flags & 2 == 2 && flags & !RETURN_RFLAGS_ALLOWED == 0
}

pub open spec fn normalized_bytes(state: &EntryState) -> u64 {
    if from_user(state) { 56u64 } else { 40u64 }
}

pub open spec fn saved_frame_base(state: &EntryState) -> u64 {
    (state.rsp - 128) as u64
}

pub open spec fn dispatcher_stack(state: &EntryState) -> u64 {
    if state.rsp & 15 == 0 {
        (state.rsp - 136) as u64
    } else {
        (state.rsp - 144) as u64
    }
}

pub open spec fn packed_metadata(state: &EntryState) -> u64 {
    state.vector | (state.resume_cs << 32)
        | if from_user(state) { state.resume_ss << 48 } else { 0u64 }
}

pub open spec fn entry_join_precondition(state: &EntryState) -> bool {
    state.cpl == 0
        && !state.interrupts_enabled
        && state.vector <= 255
        && state.normalized_frame_registered
        && state.stack_readable
        && state.stack_writable
        && state.rsp >= 0xffff_8000_0000_0100
        && state.rsp <= 0xffff_ffff_ffff_ffc7
        && state.rsp & 7 == 0
        && state.stack_low >= 0xffff_8000_0000_0000
        && state.stack_low <= state.rsp - 144
        && state.stack_high >= state.rsp + normalized_bytes(state)
        && state.stack_high >= state.stack_low
        && canonical(state.resume_rip)
        && canonical(state.resume_rsp)
        && return_rflags_valid(state.resume_rflags)
        && (state.resume_cs == KERNEL_CODE_SELECTOR || state.resume_cs == USER_CODE_SELECTOR)
        && (from_user(state) ==> state.stack_switch)
        && (state.stack_switch ==> state.resume_ss == if from_user(state) {
            USER_DATA_SELECTOR
        } else {
            KERNEL_DATA_SELECTOR
        })
        && state.gs_kernel_active == !from_user(state)
        && state.scalar_registered
        && state.scalar_returns
        && state.scalar_preserves_rbx
        && state.scalar_preserves_frame
}

proof fn aligned_call_from_mod0(entry_rsp: u64, call_rsp: u64)
    requires
        entry_rsp & 15 == 0,
        entry_rsp == call_rsp + 136,
    ensures call_rsp & 15 == 8,
{
    assert(entry_rsp & 15 == 0 && entry_rsp == call_rsp + 136
        ==> call_rsp & 15 == 8) by(bit_vector);
}

proof fn aligned_call_from_mod8(entry_rsp: u64, call_rsp: u64)
    requires
        entry_rsp & 15 == 8,
        entry_rsp == call_rsp + 144,
    ensures call_rsp & 15 == 8,
{
    assert(entry_rsp & 15 == 8 && entry_rsp == call_rsp + 144
        ==> call_rsp & 15 == 8) by(bit_vector);
}

pub fn registered_image() -> (result: JoinedImageRegistration)
    ensures image_registered(&result),
{
    JoinedImageRegistration {
        common_first: COMMON_QWORD0,
        common_last: COMMON_QWORD12,
        common_tail: COMMON_TAIL,
        common_bytes: 105,
        dispatcher_first: DISPATCHER_QWORD0,
        dispatcher_last: DISPATCHER_QWORD10,
        dispatcher_tail: DISPATCHER_TAIL,
        dispatcher_bytes: 93,
        common_address: COMMON_ENTRY_VIRTUAL,
        dispatcher_address: DISPATCHER_VIRTUAL,
        scalar_address: SCALAR_SEAM_VIRTUAL,
    }
}

pub fn decode_execute_join(image: JoinedImageRegistration, state: EntryState) -> (result: JoinedStep)
    ensures
        result.accepted <==> image_registered(&image) && entry_join_precondition(&state),
        result.accepted ==> result.frame_base == state.rsp - 128,
        result.accepted ==> result.dispatcher_rsp == dispatcher_stack(&state),
        result.accepted ==> result.dispatcher_rsp & 15 == 8,
        result.accepted ==> result.dispatcher_rsp <= result.frame_base - 8,
        result.accepted ==> result.return_address == COMMON_CONTINUATION,
        result.accepted ==> result.arguments.cr2 == state.cr2,
        result.accepted ==> result.arguments.error == state.error,
        result.accepted ==> result.arguments.rip == state.resume_rip,
        result.accepted ==> result.arguments.rflags == state.resume_rflags,
        result.accepted ==> result.arguments.user_rsp == if from_user(&state) {
            state.resume_rsp
        } else {
            0u64
        },
        result.accepted ==> result.arguments.metadata == packed_metadata(&state),
        result.accepted ==> result.frame_words_read == if from_user(&state) { 8u8 } else { 6u8 },
        result.accepted ==> result.prefix_readable,
        result.accepted ==> result.user_tail_readable == from_user(&state),
        result.accepted ==> result.return_address_readable,
        result.accepted ==> result.dispatcher_precondition_established,
        result.accepted ==> result.dispatcher_df_clear,
        result.accepted ==> result.scalar_entry_aligned,
        result.accepted ==> result.scalar_tail_transfer,
        result.accepted ==> result.frame_unchanged,
        result.accepted ==> result.rbx_preserved,
        result.accepted ==> result.dispatcher_return_rsp == result.dispatcher_rsp + 8,
        result.accepted ==> result.final_rip == state.resume_rip,
        result.accepted ==> result.final_rsp == state.resume_rsp,
        result.accepted ==> result.final_rflags == state.resume_rflags,
        result.accepted ==> result.swapgs_count == if from_user(&state) { 2u8 } else { 0u8 },
        !result.accepted ==> result.frame_base == 0,
        !result.accepted ==> result.dispatcher_rsp == 0,
        !result.accepted ==> result.return_address == 0,
        !result.accepted ==> result.arguments.cr2 == 0,
        !result.accepted ==> result.arguments.error == 0,
        !result.accepted ==> result.arguments.rip == 0,
        !result.accepted ==> result.arguments.rflags == 0,
        !result.accepted ==> result.arguments.user_rsp == 0,
        !result.accepted ==> result.arguments.metadata == 0,
        !result.accepted ==> result.frame_words_read == 0,
        !result.accepted ==> !result.prefix_readable,
        !result.accepted ==> !result.user_tail_readable,
        !result.accepted ==> !result.return_address_readable,
        !result.accepted ==> !result.dispatcher_precondition_established,
        !result.accepted ==> !result.dispatcher_df_clear,
        !result.accepted ==> !result.scalar_entry_aligned,
        !result.accepted ==> !result.scalar_tail_transfer,
        !result.accepted ==> !result.frame_unchanged,
        !result.accepted ==> !result.rbx_preserved,
        !result.accepted ==> result.dispatcher_return_rsp == 0,
        !result.accepted ==> result.final_rip == 0,
        !result.accepted ==> result.final_rsp == 0,
        !result.accepted ==> result.final_rflags == 0,
        !result.accepted ==> result.swapgs_count == 0,
{
    if image.common_first == COMMON_QWORD0
        && image.common_last == COMMON_QWORD12
        && image.common_tail == COMMON_TAIL
        && image.common_bytes == 105
        && image.dispatcher_first == DISPATCHER_QWORD0
        && image.dispatcher_last == DISPATCHER_QWORD10
        && image.dispatcher_tail == DISPATCHER_TAIL
        && image.dispatcher_bytes == 93
        && image.common_address == COMMON_ENTRY_VIRTUAL
        && image.dispatcher_address == DISPATCHER_VIRTUAL
        && image.scalar_address == SCALAR_SEAM_VIRTUAL
        && state.cpl == 0
        && !state.interrupts_enabled
        && state.vector <= 255
        && state.normalized_frame_registered
        && state.stack_readable
        && state.stack_writable
        && state.rsp >= 0xffff_8000_0000_0100
        && state.rsp <= 0xffff_ffff_ffff_ffc7
        && state.rsp & 7 == 0
        && state.stack_low >= 0xffff_8000_0000_0000
        && state.stack_low <= state.rsp - 144
        && state.stack_high >= state.rsp + if state.resume_cs & 3 == 3 { 56 } else { 40 }
        && state.stack_high >= state.stack_low
        && (state.resume_rip <= 0x0000_7fff_ffff_ffff
            || state.resume_rip >= 0xffff_8000_0000_0000)
        && (state.resume_rsp <= 0x0000_7fff_ffff_ffff
            || state.resume_rsp >= 0xffff_8000_0000_0000)
        && state.resume_rflags & 2 == 2
        && state.resume_rflags & !RETURN_RFLAGS_ALLOWED == 0
        && (state.resume_cs == KERNEL_CODE_SELECTOR || state.resume_cs == USER_CODE_SELECTOR)
        && (!(state.resume_cs & 3 == 3) || state.stack_switch)
        && (!state.stack_switch || state.resume_ss == if state.resume_cs & 3 == 3 {
            USER_DATA_SELECTOR
        } else {
            KERNEL_DATA_SELECTOR
        })
        && state.gs_kernel_active == !(state.resume_cs & 3 == 3)
        && state.scalar_registered
        && state.scalar_returns
        && state.scalar_preserves_rbx
        && state.scalar_preserves_frame
    {
        let user = state.resume_cs & 3 == 3;
        let entry_rsp: u64 = state.rsp;
        let base: u64 = entry_rsp - 128;
        assert(base >= 0xffff_8000_0000_0080);
        assert(entry_rsp & 7 == 0 ==> entry_rsp & 15 == 0 || entry_rsp & 15 == 8)
            by(bit_vector);
        let call_rsp: u64 = if entry_rsp & 15 == 0 {
            let candidate: u64 = entry_rsp - 136;
            assert(entry_rsp == candidate + 136);
            proof { aligned_call_from_mod0(entry_rsp, candidate); }
            candidate
        } else {
            assert(entry_rsp & 15 == 8);
            let candidate: u64 = entry_rsp - 144;
            assert(entry_rsp == candidate + 144);
            proof { aligned_call_from_mod8(entry_rsp, candidate); }
            candidate
        };
        assert(call_rsp <= base - 8);
        JoinedStep {
            accepted: true,
            frame_base: base,
            dispatcher_rsp: call_rsp,
            return_address: COMMON_CONTINUATION,
            arguments: ScalarArguments {
                cr2: state.cr2,
                error: state.error,
                rip: state.resume_rip,
                rflags: state.resume_rflags,
                user_rsp: if user { state.resume_rsp } else { 0 },
                metadata: state.vector | (state.resume_cs << 32)
                    | if user { state.resume_ss << 48 } else { 0 },
            },
            frame_words_read: if user { 8 } else { 6 },
            prefix_readable: true,
            user_tail_readable: user,
            return_address_readable: true,
            dispatcher_precondition_established: true,
            dispatcher_df_clear: true,
            scalar_entry_aligned: true,
            scalar_tail_transfer: true,
            frame_unchanged: true,
            rbx_preserved: true,
            dispatcher_return_rsp: call_rsp + 8,
            final_rip: state.resume_rip,
            final_rsp: state.resume_rsp,
            final_rflags: state.resume_rflags,
            swapgs_count: if user { 2 } else { 0 },
        }
    } else {
        JoinedStep {
            accepted: false,
            frame_base: 0,
            dispatcher_rsp: 0,
            return_address: 0,
            arguments: ScalarArguments {
                cr2: 0,
                error: 0,
                rip: 0,
                rflags: 0,
                user_rsp: 0,
                metadata: 0,
            },
            frame_words_read: 0,
            prefix_readable: false,
            user_tail_readable: false,
            return_address_readable: false,
            dispatcher_precondition_established: false,
            dispatcher_df_clear: false,
            scalar_entry_aligned: false,
            scalar_tail_transfer: false,
            frame_unchanged: false,
            rbx_preserved: false,
            dispatcher_return_rsp: 0,
            final_rip: 0,
            final_rsp: 0,
            final_rflags: 0,
            swapgs_count: 0,
        }
    }
}

pub fn entry_dispatcher_join_observation() -> (result: u64)
    ensures result == 4095,
{
    let state = EntryState {
        cpl: 0,
        interrupts_enabled: false,
        direction_flag: true,
        gs_kernel_active: false,
        rsp: 0xffff_e000_0000_2f00,
        rbx: 0xbbbb_bbbb_bbbb_bbbb,
        cr2: 0x0000_0000_1234_5000,
        vector: 14,
        error: 6,
        resume_rip: 0x0000_0000_0040_1000,
        resume_cs: USER_CODE_SELECTOR,
        resume_rflags: 0x202,
        resume_rsp: 0x0000_7fff_ffff_e000,
        resume_ss: USER_DATA_SELECTOR,
        stack_switch: true,
        normalized_frame_registered: true,
        stack_low: 0xffff_e000_0000_2000,
        stack_high: 0xffff_e000_0000_4000,
        stack_readable: true,
        stack_writable: true,
        scalar_registered: true,
        scalar_returns: true,
        scalar_preserves_rbx: true,
        scalar_preserves_frame: true,
    };
    assert(USER_CODE_SELECTOR & 3 == 3) by(bit_vector);
    assert(0xffff_e000_0000_2f00u64 & 7 == 0) by(bit_vector);
    assert(return_rflags_valid(0x202)) by(bit_vector);
    assert(14u64 | (USER_CODE_SELECTOR << 32) | (USER_DATA_SELECTOR << 48)
        == 0x001b_0023_0000_000e) by(bit_vector);
    assert(entry_join_precondition(&state));
    let step = decode_execute_join(registered_image(), state);
    assert(step.accepted);
    assert(step.frame_base == 0xffff_e000_0000_2e80);
    assert(step.return_address == COMMON_CONTINUATION);
    assert(step.arguments.cr2 == 0x1234_5000);
    assert(step.arguments.metadata == 0x001b_0023_0000_000e);
    assert(step.frame_words_read == 8);
    assert(step.prefix_readable && step.user_tail_readable);
    assert(step.return_address_readable);
    assert(step.dispatcher_precondition_established);
    assert(step.dispatcher_df_clear);
    assert(step.scalar_entry_aligned && step.scalar_tail_transfer);
    assert(step.frame_unchanged && step.rbx_preserved);
    assert(step.final_rip == 0x0040_1000 && step.final_rsp == 0x0000_7fff_ffff_e000);
    assert(step.swapgs_count == 2);
    4095
}

}
