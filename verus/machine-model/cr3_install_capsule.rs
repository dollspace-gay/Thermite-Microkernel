#![no_std]
#![crate_type = "rlib"]

use verus_builtin::*;
use verus_builtin_macros::*;

verus! {

pub const ROOT_PHYSICAL: u64 = 0x0040_0000;
pub const CR3_ADDRESS_MASK: u64 = 0x000f_ffff_ffff_f000;
pub const REGISTERED_WORD: u32 = 0xc3df_220f;

pub struct MachineState {
    pub rax: u64,
    pub rdi: u64,
    pub rsp: u64,
    pub rip: u64,
    pub rflags: u64,
    pub cr3: u64,
    pub cpl: u8,
    pub cr4_pcide: bool,
    pub interrupts_enabled: bool,
    pub stack_readable: bool,
    pub return_address: u64,
    pub non_global_tlb_valid: bool,
}

pub struct CapsuleStep {
    pub accepted: bool,
    pub state: MachineState,
}

pub open spec fn canonical_address(address: u64) -> bool {
    address <= 0x0000_7fff_ffff_ffff
        || address >= 0xffff_8000_0000_0000
}

pub open spec fn install_precondition(state: &MachineState) -> bool {
    state.cpl == 0
        && !state.cr4_pcide
        && state.rdi & 0xfff == 0
        && state.rdi <= CR3_ADDRESS_MASK
        && state.stack_readable
        && state.rsp <= 0xffff_ffff_ffff_fff7
        && canonical_address(state.return_address)
}

pub fn registered_word() -> (result: u32)
    ensures
        result == REGISTERED_WORD,
        result & 0xff == 0x0f,
        (result >> 8) & 0xff == 0x22,
        (result >> 16) & 0xff == 0xdf,
        (result >> 24) & 0xff == 0xc3,
{
    let word = REGISTERED_WORD;
    assert(REGISTERED_WORD & 0xff == 0x0f) by(bit_vector);
    assert((REGISTERED_WORD >> 8) & 0xff == 0x22) by(bit_vector);
    assert((REGISTERED_WORD >> 16) & 0xff == 0xdf) by(bit_vector);
    assert((REGISTERED_WORD >> 24) & 0xff == 0xc3) by(bit_vector);
    word
}

pub fn decode_execute(word: u32, state: MachineState) -> (result: CapsuleStep)
    ensures
        result.accepted <==> word == REGISTERED_WORD && install_precondition(&state),
        result.accepted ==> result.state.cr3 == state.rdi,
        result.accepted ==> result.state.rsp == state.rsp + 8,
        result.accepted ==> result.state.rip == state.return_address,
        result.accepted ==> !result.state.non_global_tlb_valid,
        result.accepted ==> result.state.rax == state.rax,
        result.accepted ==> result.state.rdi == state.rdi,
        result.accepted ==> result.state.rflags == state.rflags,
        result.accepted ==> result.state.cpl == state.cpl,
        result.accepted ==> result.state.cr4_pcide == state.cr4_pcide,
        result.accepted ==> result.state.interrupts_enabled == state.interrupts_enabled,
        result.accepted ==> result.state.stack_readable == state.stack_readable,
        result.accepted ==> result.state.return_address == state.return_address,
        !result.accepted ==> result.state.rax == state.rax,
        !result.accepted ==> result.state.rdi == state.rdi,
        !result.accepted ==> result.state.rsp == state.rsp,
        !result.accepted ==> result.state.rip == state.rip,
        !result.accepted ==> result.state.rflags == state.rflags,
        !result.accepted ==> result.state.cr3 == state.cr3,
        !result.accepted ==> result.state.cpl == state.cpl,
        !result.accepted ==> result.state.cr4_pcide == state.cr4_pcide,
        !result.accepted ==> result.state.interrupts_enabled == state.interrupts_enabled,
        !result.accepted ==> result.state.stack_readable == state.stack_readable,
        !result.accepted ==> result.state.return_address == state.return_address,
        !result.accepted ==>
            result.state.non_global_tlb_valid == state.non_global_tlb_valid,
{
    if word == REGISTERED_WORD
        && state.cpl == 0
        && !state.cr4_pcide
        && state.rdi & 0xfff == 0
        && state.rdi <= CR3_ADDRESS_MASK
        && state.stack_readable
        && state.rsp <= 0xffff_ffff_ffff_fff7
        && (state.return_address <= 0x0000_7fff_ffff_ffff
            || state.return_address >= 0xffff_8000_0000_0000)
    {
        CapsuleStep {
            accepted: true,
            state: MachineState {
                rax: state.rax,
                rdi: state.rdi,
                rsp: state.rsp + 8,
                rip: state.return_address,
                rflags: state.rflags,
                cr3: state.rdi,
                cpl: state.cpl,
                cr4_pcide: state.cr4_pcide,
                interrupts_enabled: state.interrupts_enabled,
                stack_readable: state.stack_readable,
                return_address: state.return_address,
                non_global_tlb_valid: false,
            },
        }
    } else {
        CapsuleStep { accepted: false, state }
    }
}

pub fn install_registered_root(state: MachineState) -> (result: CapsuleStep)
    requires
        install_precondition(&state),
        state.rdi == ROOT_PHYSICAL,
    ensures
        result.accepted,
        result.state.cr3 == ROOT_PHYSICAL,
        result.state.rsp == state.rsp + 8,
        result.state.rip == state.return_address,
        !result.state.non_global_tlb_valid,
        result.state.rax == state.rax,
        result.state.rdi == state.rdi,
        result.state.rflags == state.rflags,
        result.state.interrupts_enabled == state.interrupts_enabled,
{
    assert(ROOT_PHYSICAL & 0xfff == 0) by(bit_vector);
    assert(ROOT_PHYSICAL <= CR3_ADDRESS_MASK) by(bit_vector);
    decode_execute(registered_word(), state)
}

pub fn registered_install_observation() -> (result: u64)
    ensures result == 0x0040_2028,
{
    let state = MachineState {
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
    };
    assert(ROOT_PHYSICAL & 0xfff == 0) by(bit_vector);
    assert(ROOT_PHYSICAL <= CR3_ADDRESS_MASK) by(bit_vector);
    assert(canonical_address(state.return_address));
    assert(install_precondition(&state));
    let step = install_registered_root(state);
    assert(step.accepted);
    assert(step.state.cr3 == ROOT_PHYSICAL);
    assert(step.state.rsp == 0x2028);
    assert(step.state.rip == 0xffff_ffff_8000_2000);
    assert(!step.state.non_global_tlb_valid);
    assert(ROOT_PHYSICAL + 0x2028 == 0x0040_2028);
    step.state.cr3 + step.state.rsp
}

}
