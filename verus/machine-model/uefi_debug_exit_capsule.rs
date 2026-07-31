#![no_std]
#![crate_type = "rlib"]

use verus_builtin::*;
use verus_builtin_macros::*;

verus! {

pub struct UefiEntryCode {
    pub word0: u64,
    pub word1: u64,
    pub word2: u64,
    pub word3: u64,
    pub word4: u64,
    pub word5: u64,
    pub word6: u64,
}

pub struct UefiMachineState {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsp: u64,
    pub write0: u64,
    pub write1: u64,
    pub returned: bool,
}

pub struct UefiEntryStep {
    pub accepted: bool,
    pub state: UefiMachineState,
}

pub fn entry_code() -> (result: UefiEntryCode)
    ensures
        result.word0 == 0xb0ee_54b0_00e9_ba66u64,
        result.word1 == 0xee5f_b0ee_4bb0_ee4du64,
        result.word2 == 0x5fb0_ee30_b0ee_4db0u64,
        result.word3 == 0xb0ee_45b0_ee55_b0eeu64,
        result.word4 == 0xee5f_b0ee_49b0_ee46u64,
        result.word5 == 0x21b0_ee4b_b0ee_4fb0u64,
        result.word6 == 0xccc3_c031_ee0a_b0eeu64,
{
    UefiEntryCode {
        word0: 0xb0ee_54b0_00e9_ba66u64,
        word1: 0xee5f_b0ee_4bb0_ee4du64,
        word2: 0x5fb0_ee30_b0ee_4db0u64,
        word3: 0xb0ee_45b0_ee55_b0eeu64,
        word4: 0xee5f_b0ee_49b0_ee46u64,
        word5: 0x21b0_ee4b_b0ee_4fb0u64,
        word6: 0xccc3_c031_ee0a_b0eeu64,
    }
}

pub fn decode_execute(code: UefiEntryCode, state: UefiMachineState) -> (result: UefiEntryStep)
    ensures
        result.accepted <==>
            code.word0 == 0xb0ee_54b0_00e9_ba66u64
            && code.word1 == 0xee5f_b0ee_4bb0_ee4du64
            && code.word2 == 0x5fb0_ee30_b0ee_4db0u64
            && code.word3 == 0xb0ee_45b0_ee55_b0eeu64
            && code.word4 == 0xee5f_b0ee_49b0_ee46u64
            && code.word5 == 0x21b0_ee4b_b0ee_4fb0u64
            && code.word6 == 0xccc3_c031_ee0a_b0eeu64,
        result.accepted ==> result.state.rax == 0,
        result.accepted ==> result.state.rbx == state.rbx,
        result.accepted ==> result.state.rcx == state.rcx,
        result.accepted ==> result.state.rdx == 0x00e9,
        result.accepted ==> result.state.rsp == state.rsp,
        result.accepted ==> result.state.write0 == 0x555f_304d_5f4b_4d54u64,
        result.accepted ==> result.state.write1 == 0x0a21_4b4f_5f49_4645u64,
        result.accepted ==> result.state.returned,
        !result.accepted ==> result.state.rax == state.rax,
        !result.accepted ==> result.state.rbx == state.rbx,
        !result.accepted ==> result.state.rcx == state.rcx,
        !result.accepted ==> result.state.rdx == state.rdx,
        !result.accepted ==> result.state.rsp == state.rsp,
        !result.accepted ==> result.state.write0 == state.write0,
        !result.accepted ==> result.state.write1 == state.write1,
        !result.accepted ==> result.state.returned == state.returned,
{
    if code.word0 == 0xb0ee_54b0_00e9_ba66u64
        && code.word1 == 0xee5f_b0ee_4bb0_ee4du64
        && code.word2 == 0x5fb0_ee30_b0ee_4db0u64
        && code.word3 == 0xb0ee_45b0_ee55_b0eeu64
        && code.word4 == 0xee5f_b0ee_49b0_ee46u64
        && code.word5 == 0x21b0_ee4b_b0ee_4fb0u64
        && code.word6 == 0xccc3_c031_ee0a_b0eeu64
    {
        UefiEntryStep {
            accepted: true,
            state: UefiMachineState {
                rax: 0,
                rbx: state.rbx,
                rcx: state.rcx,
                rdx: 0x00e9,
                rsp: state.rsp,
                write0: 0x555f_304d_5f4b_4d54u64,
                write1: 0x0a21_4b4f_5f49_4645u64,
                returned: true,
            },
        }
    } else {
        UefiEntryStep { accepted: false, state }
    }
}

pub fn execute_registered(state: UefiMachineState) -> (result: UefiEntryStep)
    ensures
        result.accepted,
        result.state.rax == 0,
        result.state.rbx == state.rbx,
        result.state.rcx == state.rcx,
        result.state.rdx == 0x00e9,
        result.state.rsp == state.rsp,
        result.state.write0 == 0x555f_304d_5f4b_4d54u64,
        result.state.write1 == 0x0a21_4b4f_5f49_4645u64,
        result.state.returned,
{
    decode_execute(entry_code(), state)
}

}
