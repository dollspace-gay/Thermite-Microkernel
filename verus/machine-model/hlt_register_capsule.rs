#![no_std]
#![crate_type = "rlib"]

use verus_builtin::*;
use verus_builtin_macros::*;

verus! {

pub struct MachineState {
    pub rax: u64,
    pub rdi: u64,
    pub rip: u64,
    pub halted: bool,
}

pub struct CapsuleStep {
    pub accepted: bool,
    pub state: MachineState,
}

pub fn registered_word() -> (result: u32)
    ensures
        result == 0xf4f88948u32,
        result & 0xffu32 == 0x48u32,
        (result >> 8) & 0xffu32 == 0x89u32,
        (result >> 16) & 0xffu32 == 0xf8u32,
        (result >> 24) & 0xffu32 == 0xf4u32,
{
    let word: u32 = 0xf4f88948u32;
    assert(0xf4f88948u32 & 0xffu32 == 0x48u32) by (bit_vector);
    assert((0xf4f88948u32 >> 8) & 0xffu32 == 0x89u32) by (bit_vector);
    assert((0xf4f88948u32 >> 16) & 0xffu32 == 0xf8u32) by (bit_vector);
    assert((0xf4f88948u32 >> 24) & 0xffu32 == 0xf4u32) by (bit_vector);
    word
}

pub fn decode_execute(word: u32, state: MachineState) -> (result: CapsuleStep)
    ensures
        result.accepted <==>
            (word == 0xf4f88948u32 && state.rip <= 0xffff_ffff_ffff_fffbu64),
        result.accepted ==> result.state.rax == state.rdi,
        result.accepted ==> result.state.rdi == state.rdi,
        result.accepted ==> result.state.rip == state.rip + 4,
        result.accepted ==> result.state.halted,
        !result.accepted ==> result.state.rax == state.rax,
        !result.accepted ==> result.state.rdi == state.rdi,
        !result.accepted ==> result.state.rip == state.rip,
        !result.accepted ==> result.state.halted == state.halted,
{
    if word == 0xf4f88948u32 && state.rip <= 0xffff_ffff_ffff_fffbu64 {
        CapsuleStep {
            accepted: true,
            state: MachineState {
                rax: state.rdi,
                rdi: state.rdi,
                rip: state.rip + 4,
                halted: true,
            },
        }
    } else {
        CapsuleStep { accepted: false, state }
    }
}

pub fn execute_registered(state: MachineState) -> (result: CapsuleStep)
    ensures
        result.accepted <==> state.rip <= 0xffff_ffff_ffff_fffbu64,
        result.accepted ==> result.state.rax == state.rdi,
        result.accepted ==> result.state.rdi == state.rdi,
        result.accepted ==> result.state.rip == state.rip + 4,
        result.accepted ==> result.state.halted,
        !result.accepted ==> result.state.rax == state.rax,
        !result.accepted ==> result.state.rdi == state.rdi,
        !result.accepted ==> result.state.rip == state.rip,
        !result.accepted ==> result.state.halted == state.halted,
{
    decode_execute(registered_word(), state)
}

}
