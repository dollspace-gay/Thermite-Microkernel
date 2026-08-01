#![no_std]
#![crate_type = "rlib"]

use verus_builtin::*;
use verus_builtin_macros::*;
use vstd::array::ArrayAdditionalSpecFns;

verus! {

pub const VECTOR_COUNT: usize = 256;
pub const STUB_BYTES: u64 = 16;
pub const STUB_TABLE_VIRTUAL: u64 = 0xffff_ffff_8001_0000;
pub const COMMON_ENTRY_VIRTUAL: u64 = 0xffff_ffff_8001_1000;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct StubImage {
    pub qword0: u64,
    pub qword1: u64,
}

#[repr(C, align(4096))]
pub struct StubTable {
    pub entries: [StubImage; VECTOR_COUNT],
}

pub open spec fn cpu_pushes_error_code(vector: u16) -> bool {
    vector == 8
        || vector == 10
        || vector == 11
        || vector == 12
        || vector == 13
        || vector == 14
        || vector == 17
        || vector == 21
        || vector == 29
        || vector == 30
}

pub open spec fn expected_stub_address(vector: u16) -> int {
    STUB_TABLE_VIRTUAL as int + vector as int * STUB_BYTES as int
}

pub open spec fn spec_has_synthetic_error(stub: &StubImage) -> bool {
    stub.qword0 & 0xffff == 0x006a
}

pub open spec fn spec_stub_vector(stub: &StubImage) -> u64 {
    if spec_has_synthetic_error(stub) {
        (stub.qword0 >> 24) & 0xffff_ffff
    } else {
        (stub.qword0 >> 8) & 0xffff_ffff
    }
}

pub open spec fn spec_jump_displacement(stub: &StubImage) -> u64 {
    if spec_has_synthetic_error(stub) {
        stub.qword1 & 0xffff_ffff
    } else {
        ((stub.qword0 >> 48) & 0xffff) | ((stub.qword1 & 0xffff) << 16)
    }
}

pub open spec fn spec_stub_jump_target(stub: &StubImage, vector: u16) -> int {
    expected_stub_address(vector)
        + if spec_has_synthetic_error(stub) { 12int } else { 10int }
        + spec_jump_displacement(stub) as int
}

pub open spec fn spec_stub_encoding_valid(stub: &StubImage) -> bool {
    if spec_has_synthetic_error(stub) {
        ((stub.qword0 >> 16) & 0xff) == 0x68
            && ((stub.qword0 >> 56) & 0xff) == 0xe9
    } else {
        (stub.qword0 & 0xff) == 0x68
            && ((stub.qword0 >> 40) & 0xff) == 0xe9
    }
}

pub open spec fn spec_stub_padding_valid(stub: &StubImage) -> bool {
    if spec_has_synthetic_error(stub) {
        stub.qword1 >> 32 == 0x9090_9090
    } else {
        stub.qword1 >> 16 == 0x9090_9090_9090
    }
}

pub open spec fn registered_stub(stub: &StubImage, vector: u16) -> bool {
    vector < 256
        && spec_has_synthetic_error(stub) == !cpu_pushes_error_code(vector)
        && spec_stub_vector(stub) == vector as u64
        && spec_stub_encoding_valid(stub)
        && spec_stub_padding_valid(stub)
        && spec_jump_displacement(stub) <= 4084
        && spec_stub_jump_target(stub, vector) == COMMON_ENTRY_VIRTUAL as int
}

pub open spec fn registered_stub_table_spec(table: &StubTable) -> bool {
    forall|index: int| 0 <= index < VECTOR_COUNT
        ==> registered_stub(&table.entries[index], index as u16)
}

pub fn stub_has_synthetic_error(stub: &StubImage) -> (result: bool)
    ensures result == spec_has_synthetic_error(stub),
{
    stub.qword0 & 0xffff == 0x006a
}

pub fn stub_vector(stub: &StubImage) -> (result: u64)
    ensures result == spec_stub_vector(stub),
{
    if stub.qword0 & 0xffff == 0x006a {
        (stub.qword0 >> 24) & 0xffff_ffff
    } else {
        (stub.qword0 >> 8) & 0xffff_ffff
    }
}

pub fn stub_jump_displacement(stub: &StubImage) -> (result: u64)
    ensures result == spec_jump_displacement(stub),
{
    if stub.qword0 & 0xffff == 0x006a {
        stub.qword1 & 0xffff_ffff
    } else {
        ((stub.qword0 >> 48) & 0xffff) | ((stub.qword1 & 0xffff) << 16)
    }
}

pub fn stub_jump_target(stub: &StubImage, vector: u16) -> (result: u64)
    requires vector < 256,
    ensures
        spec_jump_displacement(stub) <= 4084
            ==> result as int == spec_stub_jump_target(stub, vector),
        spec_jump_displacement(stub) > 4084 ==> result == 0,
{
    let next = STUB_TABLE_VIRTUAL
        + (vector as u64) * STUB_BYTES
        + if stub.qword0 & 0xffff == 0x006a { 12 } else { 10 };
    let displacement = stub_jump_displacement(stub);
    if displacement <= 4084 {
        next + displacement
    } else {
        0
    }
}

pub fn stub_encoding_valid(stub: &StubImage) -> (result: bool)
    ensures result == spec_stub_encoding_valid(stub),
{
    if stub.qword0 & 0xffff == 0x006a {
        ((stub.qword0 >> 16) & 0xff) == 0x68
            && ((stub.qword0 >> 56) & 0xff) == 0xe9
    } else {
        (stub.qword0 & 0xff) == 0x68
            && ((stub.qword0 >> 40) & 0xff) == 0xe9
    }
}

pub fn stub_padding_valid(stub: &StubImage) -> (result: bool)
    ensures result == spec_stub_padding_valid(stub),
{
    if stub.qword0 & 0xffff == 0x006a {
        stub.qword1 >> 32 == 0x9090_9090
    } else {
        stub.qword1 >> 16 == 0x9090_9090_9090
    }
}

pub fn vector_has_error_code(vector: u16) -> (result: bool)
    ensures result == cpu_pushes_error_code(vector),
{
    vector == 8
        || vector == 10
        || vector == 11
        || vector == 12
        || vector == 13
        || vector == 14
        || vector == 17
        || vector == 21
        || vector == 29
        || vector == 30
}

pub fn registered_displacement(vector: u16, cpu_error: bool) -> (result: u32)
    requires vector < 256, cpu_error == cpu_pushes_error_code(vector),
    ensures
        result as int
            == COMMON_ENTRY_VIRTUAL as int - expected_stub_address(vector)
                - if cpu_error { 10int } else { 12int },
        result <= 4084,
{
    let used = if cpu_error { 10u32 } else { 12u32 };
    4096u32 - (vector as u32) * 16u32 - used
}

pub open spec fn synthetic_qword0(vector: u32) -> u64 {
    0xe900_0000_0068_006au64 | ((vector as u64) << 24)
}

pub open spec fn synthetic_qword1(displacement: u32) -> u64 {
    0x9090_9090_0000_0000u64 | displacement as u64
}

pub open spec fn cpu_error_qword0(vector: u32, displacement: u32) -> u64 {
    0x0000_e900_0000_0068u64
        | ((vector as u64) << 8)
        | (((displacement as u64) & 0xffff) << 48)
}

pub open spec fn cpu_error_qword1(displacement: u32) -> u64 {
    0x9090_9090_9090_0000u64
        | (((displacement as u64) >> 16) & 0xffff)
}

#[verifier::bit_vector]
proof fn lemma_synthetic_encoding(vector: u32, displacement: u32)
    requires vector < 256, displacement <= 4084,
    ensures
        synthetic_qword0(vector) & 0xffff == 0x006a,
        ((synthetic_qword0(vector) >> 24) & 0xffff_ffff) == vector as u64,
        ((synthetic_qword0(vector) >> 16) & 0xff) == 0x68,
        ((synthetic_qword0(vector) >> 56) & 0xff) == 0xe9,
        (synthetic_qword1(displacement) & 0xffff_ffff) == displacement as u64,
        synthetic_qword1(displacement) >> 32 == 0x9090_9090,
{}

#[verifier::bit_vector]
proof fn lemma_cpu_error_encoding(vector: u32, displacement: u32)
    requires vector < 256, displacement <= 4084,
    ensures
        cpu_error_qword0(vector, displacement) & 0xffff != 0x006a,
        ((cpu_error_qword0(vector, displacement) >> 8) & 0xffff_ffff) == vector as u64,
        (cpu_error_qword0(vector, displacement) & 0xff) == 0x68,
        ((cpu_error_qword0(vector, displacement) >> 40) & 0xff) == 0xe9,
        (((cpu_error_qword0(vector, displacement) >> 48) & 0xffff)
            | ((cpu_error_qword1(displacement) & 0xffff) << 16)) == displacement as u64,
        cpu_error_qword1(displacement) >> 16 == 0x9090_9090_9090,
{}

pub fn registered_stub_image(vector: u16) -> (result: StubImage)
    requires vector < 256,
    ensures registered_stub(&result, vector),
{
    let cpu_error = vector_has_error_code(vector);
    let displacement = registered_displacement(vector, cpu_error);
    if cpu_error {
        let qword0 = 0x0000_e900_0000_0068u64
            | ((vector as u64) << 8)
            | (((displacement as u64) & 0xffff) << 48);
        let qword1 = 0x9090_9090_9090_0000u64
            | (((displacement as u64) >> 16) & 0xffff);
        assert(qword0 == cpu_error_qword0(vector as u32, displacement));
        assert(qword1 == cpu_error_qword1(displacement));
        proof { lemma_cpu_error_encoding(vector as u32, displacement); }
        let result = StubImage { qword0, qword1 };
        assert(!spec_has_synthetic_error(&result));
        assert(spec_stub_vector(&result) == vector as u64);
        assert(spec_stub_encoding_valid(&result));
        assert(spec_stub_padding_valid(&result));
        assert(spec_jump_displacement(&result) == displacement as u64);
        assert(spec_stub_jump_target(&result, vector) == COMMON_ENTRY_VIRTUAL as int);
        result
    } else {
        let qword0 = 0xe900_0000_0068_006au64 | ((vector as u64) << 24);
        let qword1 = 0x9090_9090_0000_0000u64 | displacement as u64;
        assert(qword0 == synthetic_qword0(vector as u32));
        assert(qword1 == synthetic_qword1(displacement));
        proof { lemma_synthetic_encoding(vector as u32, displacement); }
        let result = StubImage { qword0, qword1 };
        assert(spec_has_synthetic_error(&result));
        assert(spec_stub_vector(&result) == vector as u64);
        assert(spec_stub_encoding_valid(&result));
        assert(spec_stub_padding_valid(&result));
        assert(spec_jump_displacement(&result) == displacement as u64);
        assert(spec_stub_jump_target(&result, vector) == COMMON_ENTRY_VIRTUAL as int);
        result
    }
}

pub fn registered_stub_table() -> (result: StubTable)
    ensures registered_stub_table_spec(&result),
{
    let zero = StubImage { qword0: 0, qword1: 0 };
    let mut entries = [zero; VECTOR_COUNT];
    let mut slot: usize = 0;
    while slot < VECTOR_COUNT
        invariant
            0 <= slot <= VECTOR_COUNT,
            forall|index: int| 0 <= index < slot
                ==> registered_stub(&entries[index], index as u16),
        decreases VECTOR_COUNT - slot,
    {
        entries[slot] = registered_stub_image(slot as u16);
        slot = slot + 1;
    }
    StubTable { entries }
}

pub fn exception_stub_observation() -> (result: u64)
    ensures result == 255,
{
    let table = registered_stub_table();
    let divide = table.entries[0];
    let double_fault = table.entries[8];
    let page_fault = table.entries[14];
    let control_protection = table.entries[21];
    let timer = table.entries[0xe0];
    let spurious = table.entries[0xff];
    assert(registered_stub(&divide, 0));
    assert(registered_stub(&double_fault, 8));
    assert(registered_stub(&page_fault, 14));
    assert(registered_stub(&control_protection, 21));
    assert(registered_stub(&timer, 0xe0));
    assert(registered_stub(&spurious, 0xff));
    let mut observation = 0u64;
    if stub_has_synthetic_error(&divide) { observation = observation | 1; }
    if !stub_has_synthetic_error(&double_fault) { observation = observation | 2; }
    if !stub_has_synthetic_error(&page_fault) { observation = observation | 4; }
    if !stub_has_synthetic_error(&control_protection) { observation = observation | 8; }
    if stub_has_synthetic_error(&timer) { observation = observation | 16; }
    if stub_has_synthetic_error(&spurious) { observation = observation | 32; }
    if stub_jump_target(&divide, 0) == COMMON_ENTRY_VIRTUAL { observation = observation | 64; }
    if stub_jump_target(&spurious, 0xff) == COMMON_ENTRY_VIRTUAL {
        observation = observation | 128;
    }
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
