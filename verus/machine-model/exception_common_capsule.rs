#![no_std]
#![crate_type = "rlib"]

use verus_builtin::*;
use verus_builtin_macros::*;

verus! {

pub const COMMON_ENTRY_VIRTUAL: u64 = 0xffff_ffff_8001_1000;
pub const DISPATCHER_VIRTUAL: u64 = 0xffff_ffff_8001_1100;
pub const KERNEL_CODE_SELECTOR: u16 = 0x08;
pub const KERNEL_DATA_SELECTOR: u16 = 0x10;
pub const USER_DATA_SELECTOR: u16 = 0x1b;
pub const USER_CODE_SELECTOR: u16 = 0x23;
pub const RETURN_RFLAGS_ALLOWED: u64 = 0x0025_0fd7;

pub const QWORD0: u64 = 0x5251_5350_d020_0f50;
pub const QWORD1: u64 = 0x4151_4150_4155_5756;
pub const QWORD2: u64 = 0x4155_4154_4153_4152;
pub const QWORD3: u64 = 0x0098_2484_f657_4156;
pub const QWORD4: u64 = 0xf801_0f03_7403_0000;
pub const QWORD5: u64 = 0x48e3_8948_e789_48fc;
pub const QWORD6: u64 = 0x0000_00c8_e8f0_e483;
pub const QWORD7: u64 = 0x0098_2484_f6dc_8948;
pub const QWORD8: u64 = 0xf801_0f03_7403_0000;
pub const QWORD9: u64 = 0x5c41_5d41_5e41_5f41;
pub const QWORD10: u64 = 0x5841_5941_5a41_5b41;
pub const QWORD11: u64 = 0x8348_5b59_5a5e_5f5d;
pub const QWORD12: u64 = 0x4810_c483_4858_08c4;
pub const TAIL: u8 = 0xcf;

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
    pub qword11: u64,
    pub qword12: u64,
    pub tail: u8,
}

pub struct MachineState {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rsp: u64,
    pub rip: u64,
    pub rflags: u64,
    pub cr2: u64,
    pub cs: u16,
    pub ss: u16,
    pub cpl: u8,
    pub interrupts_enabled: bool,
    pub direction_flag: bool,
    pub gs_kernel_active: bool,
    pub vector: u16,
    pub error_code: u64,
    pub resume_rip: u64,
    pub resume_cs: u16,
    pub resume_rflags: u64,
    pub resume_rsp: u64,
    pub resume_ss: u16,
    pub stack_switch: bool,
    pub normalized_frame_registered: bool,
    pub entry_stack_writable: bool,
    pub entry_stack_readable: bool,
    pub dispatcher_registered: bool,
    pub dispatcher_returns: bool,
    pub dispatcher_preserves_rbx: bool,
    pub dispatcher_preserves_frame: bool,
}

pub struct CommonStep {
    pub accepted: bool,
    pub state: MachineState,
    pub captured_cr2: u64,
    pub dispatcher_frame: u64,
    pub dispatcher_vector: u16,
    pub dispatcher_error: u64,
    pub dispatcher_df_clear: bool,
    pub swapgs_count: u8,
}

pub open spec fn canonical_address(address: u64) -> bool {
    address <= 0x0000_7fff_ffff_ffff
        || address >= 0xffff_8000_0000_0000
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
        && image.qword11 == QWORD11
        && image.qword12 == QWORD12
        && image.tail == TAIL
}

pub open spec fn from_user(state: &MachineState) -> bool {
    state.resume_cs & 3 == 3
}

pub open spec fn return_rflags_valid(flags: u64) -> bool {
    flags & 2 == 2 && flags & !RETURN_RFLAGS_ALLOWED == 0
}

pub open spec fn common_precondition(state: &MachineState) -> bool {
    state.cpl == 0
        && !state.interrupts_enabled
        && state.vector < 256
        && state.normalized_frame_registered
        && state.entry_stack_writable
        && state.entry_stack_readable
        && state.dispatcher_registered
        && state.dispatcher_returns
        && state.dispatcher_preserves_rbx
        && state.dispatcher_preserves_frame
        && state.rsp >= 151
        && canonical_address(state.rsp)
        && (state.rsp as int - 151 <= 0x0000_7fff_ffff_ffff
            || state.rsp as int - 151 >= 0xffff_8000_0000_0000)
        && canonical_address(state.resume_rip)
        && canonical_address(state.resume_rsp)
        && return_rflags_valid(state.resume_rflags)
        && (state.resume_cs == KERNEL_CODE_SELECTOR || state.resume_cs == USER_CODE_SELECTOR)
        && (from_user(state) ==> state.stack_switch)
        && (state.stack_switch ==>
            state.resume_ss == if from_user(state) { USER_DATA_SELECTOR } else { KERNEL_DATA_SELECTOR })
        && state.gs_kernel_active == !from_user(state)
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
        qword11: QWORD11,
        qword12: QWORD12,
        tail: TAIL,
    }
}

pub fn decode_execute(image: CapsuleImage, state: MachineState) -> (result: CommonStep)
    ensures
        result.accepted <==> image_registered(&image) && common_precondition(&state),
        result.accepted ==> result.captured_cr2 == state.cr2,
        result.accepted ==> result.dispatcher_frame == state.rsp - 128,
        result.accepted ==> result.dispatcher_vector == state.vector,
        result.accepted ==> result.dispatcher_error == state.error_code,
        result.accepted ==> result.dispatcher_df_clear,
        result.accepted ==> result.swapgs_count == if from_user(&state) { 2u8 } else { 0u8 },
        result.accepted ==> result.state.rax == state.rax,
        result.accepted ==> result.state.rbx == state.rbx,
        result.accepted ==> result.state.rcx == state.rcx,
        result.accepted ==> result.state.rdx == state.rdx,
        result.accepted ==> result.state.rsi == state.rsi,
        result.accepted ==> result.state.rdi == state.rdi,
        result.accepted ==> result.state.rbp == state.rbp,
        result.accepted ==> result.state.r8 == state.r8,
        result.accepted ==> result.state.r9 == state.r9,
        result.accepted ==> result.state.r10 == state.r10,
        result.accepted ==> result.state.r11 == state.r11,
        result.accepted ==> result.state.r12 == state.r12,
        result.accepted ==> result.state.r13 == state.r13,
        result.accepted ==> result.state.r14 == state.r14,
        result.accepted ==> result.state.r15 == state.r15,
        result.accepted ==> result.state.rsp == state.resume_rsp,
        result.accepted ==> result.state.rip == state.resume_rip,
        result.accepted ==> result.state.rflags == state.resume_rflags,
        result.accepted ==> result.state.cr2 == state.cr2,
        result.accepted ==> result.state.cs == state.resume_cs,
        result.accepted ==> result.state.ss == if state.stack_switch { state.resume_ss } else { state.ss },
        result.accepted ==> result.state.cpl == (state.resume_cs & 3) as u8,
        result.accepted ==> result.state.interrupts_enabled == (state.resume_rflags & 0x200 != 0),
        result.accepted ==> result.state.direction_flag == (state.resume_rflags & 0x400 != 0),
        result.accepted ==> result.state.gs_kernel_active == state.gs_kernel_active,
        !result.accepted ==> result.state.rax == state.rax,
        !result.accepted ==> result.state.rbx == state.rbx,
        !result.accepted ==> result.state.rsp == state.rsp,
        !result.accepted ==> result.state.rip == state.rip,
        !result.accepted ==> result.state.rflags == state.rflags,
        !result.accepted ==> result.state.cr2 == state.cr2,
        !result.accepted ==> result.state.cs == state.cs,
        !result.accepted ==> result.state.ss == state.ss,
        !result.accepted ==> result.state.cpl == state.cpl,
        !result.accepted ==> result.state.gs_kernel_active == state.gs_kernel_active,
        !result.accepted ==> result.captured_cr2 == 0,
        !result.accepted ==> result.dispatcher_frame == 0,
        !result.accepted ==> !result.dispatcher_df_clear,
        !result.accepted ==> result.swapgs_count == 0,
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
        && image.qword11 == QWORD11
        && image.qword12 == QWORD12
        && image.tail == TAIL
        && state.cpl == 0
        && !state.interrupts_enabled
        && state.vector < 256
        && state.normalized_frame_registered
        && state.entry_stack_writable
        && state.entry_stack_readable
        && state.dispatcher_registered
        && state.dispatcher_returns
        && state.dispatcher_preserves_rbx
        && state.dispatcher_preserves_frame
        && state.rsp >= 151
        && (state.rsp <= 0x0000_7fff_ffff_ffff || state.rsp >= 0xffff_8000_0000_0000)
        && (state.rsp - 151 <= 0x0000_7fff_ffff_ffff
            || state.rsp - 151 >= 0xffff_8000_0000_0000)
        && (state.resume_rip <= 0x0000_7fff_ffff_ffff
            || state.resume_rip >= 0xffff_8000_0000_0000)
        && (state.resume_rsp <= 0x0000_7fff_ffff_ffff
            || state.resume_rsp >= 0xffff_8000_0000_0000)
        && state.resume_rflags & 2 == 2
        && state.resume_rflags & !RETURN_RFLAGS_ALLOWED == 0
        && (state.resume_cs == KERNEL_CODE_SELECTOR || state.resume_cs == USER_CODE_SELECTOR)
        && (!(state.resume_cs & 3 == 3) || state.stack_switch)
        && (!state.stack_switch
            || state.resume_ss == if state.resume_cs & 3 == 3 {
                USER_DATA_SELECTOR
            } else {
                KERNEL_DATA_SELECTOR
            })
        && state.gs_kernel_active == !(state.resume_cs & 3 == 3)
    {
        let returning_user = state.resume_cs & 3 == 3;
        CommonStep {
            accepted: true,
            captured_cr2: state.cr2,
            dispatcher_frame: state.rsp - 128,
            dispatcher_vector: state.vector,
            dispatcher_error: state.error_code,
            dispatcher_df_clear: true,
            swapgs_count: if returning_user { 2 } else { 0 },
            state: MachineState {
                rax: state.rax,
                rbx: state.rbx,
                rcx: state.rcx,
                rdx: state.rdx,
                rsi: state.rsi,
                rdi: state.rdi,
                rbp: state.rbp,
                r8: state.r8,
                r9: state.r9,
                r10: state.r10,
                r11: state.r11,
                r12: state.r12,
                r13: state.r13,
                r14: state.r14,
                r15: state.r15,
                rsp: state.resume_rsp,
                rip: state.resume_rip,
                rflags: state.resume_rflags,
                cr2: state.cr2,
                cs: state.resume_cs,
                ss: if state.stack_switch { state.resume_ss } else { state.ss },
                cpl: (state.resume_cs & 3) as u8,
                interrupts_enabled: state.resume_rflags & 0x200 != 0,
                direction_flag: state.resume_rflags & 0x400 != 0,
                gs_kernel_active: state.gs_kernel_active,
                vector: state.vector,
                error_code: state.error_code,
                resume_rip: state.resume_rip,
                resume_cs: state.resume_cs,
                resume_rflags: state.resume_rflags,
                resume_rsp: state.resume_rsp,
                resume_ss: state.resume_ss,
                stack_switch: state.stack_switch,
                normalized_frame_registered: state.normalized_frame_registered,
                entry_stack_writable: state.entry_stack_writable,
                entry_stack_readable: state.entry_stack_readable,
                dispatcher_registered: state.dispatcher_registered,
                dispatcher_returns: state.dispatcher_returns,
                dispatcher_preserves_rbx: state.dispatcher_preserves_rbx,
                dispatcher_preserves_frame: state.dispatcher_preserves_frame,
            },
        }
    } else {
        CommonStep {
            accepted: false,
            state,
            captured_cr2: 0,
            dispatcher_frame: 0,
            dispatcher_vector: 0,
            dispatcher_error: 0,
            dispatcher_df_clear: false,
            swapgs_count: 0,
        }
    }
}

pub fn common_entry_observation() -> (result: u64)
    ensures result == 255,
{
    let state = MachineState {
        rax: 1, rbx: 2, rcx: 3, rdx: 4, rsi: 5, rdi: 6, rbp: 7,
        r8: 8, r9: 9, r10: 10, r11: 11, r12: 12, r13: 13, r14: 14, r15: 15,
        rsp: 0xffff_e000_0000_2f00,
        rip: COMMON_ENTRY_VIRTUAL,
        rflags: 0x402,
        cr2: 0x0000_1234_5000,
        cs: KERNEL_CODE_SELECTOR,
        ss: 0x10,
        cpl: 0,
        interrupts_enabled: false,
        direction_flag: true,
        gs_kernel_active: false,
        vector: 14,
        error_code: 7,
        resume_rip: 0x0000_0000_0040_1000,
        resume_cs: USER_CODE_SELECTOR,
        resume_rflags: 0x602,
        resume_rsp: 0x0000_7fff_ffff_e000,
        resume_ss: USER_DATA_SELECTOR,
        stack_switch: true,
        normalized_frame_registered: true,
        entry_stack_writable: true,
        entry_stack_readable: true,
        dispatcher_registered: true,
        dispatcher_returns: true,
        dispatcher_preserves_rbx: true,
        dispatcher_preserves_frame: true,
    };
    assert(canonical_address(state.rsp));
    assert(state.rsp as int - 151 >= 0xffff_8000_0000_0000);
    assert(canonical_address(state.resume_rip));
    assert(canonical_address(state.resume_rsp));
    assert(USER_CODE_SELECTOR & 3 == 3) by(bit_vector);
    assert(0x602u64 & 2 == 2 && 0x602u64 & !RETURN_RFLAGS_ALLOWED == 0) by(bit_vector);
    assert(from_user(&state));
    assert(common_precondition(&state));
    let step = decode_execute(registered_image(), state);
    assert(step.accepted);
    assert(step.captured_cr2 == 0x0000_1234_5000);
    assert(step.dispatcher_frame == 0xffff_e000_0000_2e80);
    assert(step.dispatcher_vector == 14);
    assert(step.dispatcher_error == 7);
    assert(step.dispatcher_df_clear);
    assert(step.swapgs_count == 2);
    assert(step.state.rax == 1 && step.state.r15 == 15);
    assert(step.state.rip == 0x0000_0000_0040_1000);
    assert(step.state.rsp == 0x0000_7fff_ffff_e000);
    let mut observation = 0u64;
    if step.captured_cr2 == 0x0000_1234_5000 { observation = observation | 1; }
    if step.dispatcher_frame == 0xffff_e000_0000_2e80 { observation = observation | 2; }
    if step.dispatcher_vector == 14 { observation = observation | 4; }
    if step.dispatcher_error == 7 { observation = observation | 8; }
    if step.dispatcher_df_clear { observation = observation | 16; }
    if step.swapgs_count == 2 { observation = observation | 32; }
    if step.state.rax == 1 && step.state.r15 == 15 { observation = observation | 64; }
    if step.state.rip == 0x0000_0000_0040_1000
        && step.state.rsp == 0x0000_7fff_ffff_e000 { observation = observation | 128; }
    assert((0u64 | 1u64) == 1u64
        && (1u64 | 2u64) == 3u64
        && (3u64 | 4u64) == 7u64
        && (7u64 | 8u64) == 15u64
        && (15u64 | 16u64) == 31u64
        && (31u64 | 32u64) == 63u64
        && (63u64 | 64u64) == 127u64
        && (127u64 | 128u64) == 255u64) by(bit_vector);
    assert(observation == 255);
    observation
}

}
