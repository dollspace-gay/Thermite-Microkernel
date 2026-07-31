#![no_std]
#![crate_type = "rlib"]

use verus_builtin::*;
use verus_builtin_macros::*;

verus! {

pub const BOOT_ARENA_BYTES: u64 = 65_536;
pub const BOOT_ARENA_CURSOR_OFFSET: u64 = 65_536;
pub const BOOT_ARENA_SEALED_OFFSET: u64 = 65_544;

pub struct BootArenaState {
    pub base: u64,
    pub cursor: u64,
    pub sealed: bool,
}

pub struct PointerAllocation {
    pub ok: bool,
    pub address: u64,
    pub size: u64,
    pub align: u64,
}

pub struct AllocatorCapsuleStep {
    pub state: BootArenaState,
    pub allocation: PointerAllocation,
}

pub struct AllocCapsuleImage {
    pub word00: u64,
    pub word01: u64,
    pub word02: u64,
    pub word03: u64,
    pub word04: u64,
    pub word05: u64,
    pub word06: u64,
    pub word07: u64,
    pub word08: u64,
    pub word09: u64,
    pub word10: u64,
    pub word11: u64,
    pub word12: u64,
    pub word13: u64,
}

pub struct PrimitiveCapsuleImage {
    pub word0: u64,
    pub word1: u64,
}

pub struct GlobalAllocShimImage {
    pub rust_alloc: u64,
    pub rust_null: u64,
    pub rust_dealloc: u64,
    pub method_word0: u64,
    pub method_word1: u64,
    pub method_word2: u64,
    pub seal_word0: u64,
    pub seal_word1: u64,
}

pub struct GlobalAllocRelocationPlan {
    pub rust_alloc_dispatch_offset: u64,
    pub method_arena_offset: u64,
    pub method_capsule_offset: u64,
    pub seal_arena_offset: u64,
    pub seal_capsule_offset: u64,
}

pub struct AllocCapsuleExecution {
    pub accepted: bool,
    pub step: AllocatorCapsuleStep,
}

pub struct SealCapsuleExecution {
    pub accepted: bool,
    pub state: BootArenaState,
}

pub struct MemoryCapsuleExecution {
    pub accepted: bool,
    pub observation: MemoryObservation,
}

pub fn is_registered_alloc_image(image: AllocCapsuleImage) -> (result: bool)
    ensures
        result <==> (
            image.word00 == 0xd285_4867_74f6_8548
                && image.word01 == 0x8548_ff4a_8d48_6274
                && image.word02 == 0x7640_fa83_4859_75ca
                && image.word03 == 0x0000_1000_fa81_4809
                && image.word04 == 0x0100_08bf_8348_4a75
                && image.word05 == 0x0087_8b48_4075_0000
                && image.word06 == 0x0100_003d_4800_0100
                && image.word07 == 0x2c72_c101_4831_7700
                && image.word08 == 0x8148_d121_48da_f748
                && image.word09 == 0x411d_7700_0100_00f9
                && image.word10 == 0xc829_4900_0100_00b8
                && image.word11 == 0xce01_480f_77c6_394c
                && image.word12 == 0x4800_0100_00b7_8948
                && image.word13 == 0x00c3_c031_c30f_048d
        ),
{
    image.word00 == 0xd285_4867_74f6_8548
        && image.word01 == 0x8548_ff4a_8d48_6274
        && image.word02 == 0x7640_fa83_4859_75ca
        && image.word03 == 0x0000_1000_fa81_4809
        && image.word04 == 0x0100_08bf_8348_4a75
        && image.word05 == 0x0087_8b48_4075_0000
        && image.word06 == 0x0100_003d_4800_0100
        && image.word07 == 0x2c72_c101_4831_7700
        && image.word08 == 0x8148_d121_48da_f748
        && image.word09 == 0x411d_7700_0100_00f9
        && image.word10 == 0xc829_4900_0100_00b8
        && image.word11 == 0xce01_480f_77c6_394c
        && image.word12 == 0x4800_0100_00b7_8948
        && image.word13 == 0x00c3_c031_c30f_048d
}

pub fn is_registered_seal_image(image: PrimitiveCapsuleImage) -> (result: bool)
    ensures
        result <==> (
            image.word0 == 0x0100_0100_0887_c748
                && image.word1 == 0x0000_0000_c300_0000
        ),
{
    image.word0 == 0x0100_0100_0887_c748
        && image.word1 == 0x0000_0000_c300_0000
}

pub fn is_registered_memcpy_image(image: PrimitiveCapsuleImage) -> (result: bool)
    ensures
        result <==> (
            image.word0 == 0xa4f3_d189_48f8_8948
                && image.word1 == 0x0000_0000_0000_00c3
        ),
{
    image.word0 == 0xa4f3_d189_48f8_8948
        && image.word1 == 0x0000_0000_0000_00c3
}

pub fn is_registered_memset_image(image: PrimitiveCapsuleImage) -> (result: bool)
    ensures
        result <==> (
            image.word0 == 0xf089_d189_48f8_8949
                && image.word1 == 0x0000_c3c0_894c_aaf3
        ),
{
    image.word0 == 0xf089_d189_48f8_8949
        && image.word1 == 0x0000_c3c0_894c_aaf3
}

pub fn is_registered_global_alloc_shim(
    image: GlobalAllocShimImage,
    relocations: GlobalAllocRelocationPlan,
) -> (result: bool)
    ensures
        result <==> (
            image.rust_alloc == 0x0000_0000_e9fa_8948
                && image.rust_null == 0x0000_0000_00c3_c031
                && image.rust_dealloc == 0x0000_0000_0000_00c3
                && image.method_word0 == 0x0000_c7c7_48f0_8948
                && image.method_word1 == 0xc289_48d6_8948_0000
                && image.method_word2 == 0x0000_0000_0000_00e9
                && image.seal_word0 == 0xe900_0000_00c7_c748
                && image.seal_word1 == 0x0000_0000_0000_0000
                && relocations.rust_alloc_dispatch_offset == 4
                && relocations.method_arena_offset == 6
                && relocations.method_capsule_offset == 17
                && relocations.seal_arena_offset == 3
                && relocations.seal_capsule_offset == 8
        ),
{
    image.rust_alloc == 0x0000_0000_e9fa_8948
        && image.rust_null == 0x0000_0000_00c3_c031
        && image.rust_dealloc == 0x0000_0000_0000_00c3
        && image.method_word0 == 0x0000_c7c7_48f0_8948
        && image.method_word1 == 0xc289_48d6_8948_0000
        && image.method_word2 == 0x0000_0000_0000_00e9
        && image.seal_word0 == 0xe900_0000_00c7_c748
        && image.seal_word1 == 0x0000_0000_0000_0000
        && relocations.rust_alloc_dispatch_offset == 4
        && relocations.method_arena_offset == 6
        && relocations.method_capsule_offset == 17
        && relocations.seal_arena_offset == 3
        && relocations.seal_capsule_offset == 8
}

pub fn registered_alloc_image() -> (result: AllocCapsuleImage)
    ensures
        result.word00 == 0xd285_4867_74f6_8548,
        result.word01 == 0x8548_ff4a_8d48_6274,
        result.word02 == 0x7640_fa83_4859_75ca,
        result.word03 == 0x0000_1000_fa81_4809,
        result.word04 == 0x0100_08bf_8348_4a75,
        result.word05 == 0x0087_8b48_4075_0000,
        result.word06 == 0x0100_003d_4800_0100,
        result.word07 == 0x2c72_c101_4831_7700,
        result.word08 == 0x8148_d121_48da_f748,
        result.word09 == 0x411d_7700_0100_00f9,
        result.word10 == 0xc829_4900_0100_00b8,
        result.word11 == 0xce01_480f_77c6_394c,
        result.word12 == 0x4800_0100_00b7_8948,
        result.word13 == 0x00c3_c031_c30f_048d,
{
    AllocCapsuleImage {
        word00: 0xd285_4867_74f6_8548,
        word01: 0x8548_ff4a_8d48_6274,
        word02: 0x7640_fa83_4859_75ca,
        word03: 0x0000_1000_fa81_4809,
        word04: 0x0100_08bf_8348_4a75,
        word05: 0x0087_8b48_4075_0000,
        word06: 0x0100_003d_4800_0100,
        word07: 0x2c72_c101_4831_7700,
        word08: 0x8148_d121_48da_f748,
        word09: 0x411d_7700_0100_00f9,
        word10: 0xc829_4900_0100_00b8,
        word11: 0xce01_480f_77c6_394c,
        word12: 0x4800_0100_00b7_8948,
        word13: 0x00c3_c031_c30f_048d,
    }
}

pub fn registered_seal_image() -> (result: PrimitiveCapsuleImage)
    ensures
        result.word0 == 0x0100_0100_0887_c748,
        result.word1 == 0x0000_0000_c300_0000,
{
    PrimitiveCapsuleImage {
        word0: 0x0100_0100_0887_c748,
        word1: 0x0000_0000_c300_0000,
    }
}

pub fn registered_memcpy_image() -> (result: PrimitiveCapsuleImage)
    ensures
        result.word0 == 0xa4f3_d189_48f8_8948,
        result.word1 == 0x0000_0000_0000_00c3,
{
    PrimitiveCapsuleImage {
        word0: 0xa4f3_d189_48f8_8948,
        word1: 0x0000_0000_0000_00c3,
    }
}

pub fn registered_memset_image() -> (result: PrimitiveCapsuleImage)
    ensures
        result.word0 == 0xf089_d189_48f8_8949,
        result.word1 == 0x0000_c3c0_894c_aaf3,
{
    PrimitiveCapsuleImage {
        word0: 0xf089_d189_48f8_8949,
        word1: 0x0000_c3c0_894c_aaf3,
    }
}

pub fn registered_global_alloc_shim() -> (result: GlobalAllocShimImage)
    ensures
        result.rust_alloc == 0x0000_0000_e9fa_8948,
        result.rust_null == 0x0000_0000_00c3_c031,
        result.rust_dealloc == 0x0000_0000_0000_00c3,
        result.method_word0 == 0x0000_c7c7_48f0_8948,
        result.method_word1 == 0xc289_48d6_8948_0000,
        result.method_word2 == 0x0000_0000_0000_00e9,
        result.seal_word0 == 0xe900_0000_00c7_c748,
        result.seal_word1 == 0x0000_0000_0000_0000,
{
    GlobalAllocShimImage {
        rust_alloc: 0x0000_0000_e9fa_8948,
        rust_null: 0x0000_0000_00c3_c031,
        rust_dealloc: 0x0000_0000_0000_00c3,
        method_word0: 0x0000_c7c7_48f0_8948,
        method_word1: 0xc289_48d6_8948_0000,
        method_word2: 0x0000_0000_0000_00e9,
        seal_word0: 0xe900_0000_00c7_c748,
        seal_word1: 0x0000_0000_0000_0000,
    }
}

pub fn registered_global_alloc_relocations() -> (result: GlobalAllocRelocationPlan)
    ensures
        result.rust_alloc_dispatch_offset == 4,
        result.method_arena_offset == 6,
        result.method_capsule_offset == 17,
        result.seal_arena_offset == 3,
        result.seal_capsule_offset == 8,
{
    GlobalAllocRelocationPlan {
        rust_alloc_dispatch_offset: 4,
        method_arena_offset: 6,
        method_capsule_offset: 17,
        seal_arena_offset: 3,
        seal_capsule_offset: 8,
    }
}

fn permitted_alignment(align: u64) -> (result: bool)
    ensures
        result <==> (
            align == 1
                || align == 2
                || align == 4
                || align == 8
                || align == 16
                || align == 32
                || align == 64
                || align == 4096
        ),
{
    align == 1
        || align == 2
        || align == 4
        || align == 8
        || align == 16
        || align == 32
        || align == 64
        || align == 4096
}

pub open spec fn arena_well_formed(state: BootArenaState) -> bool {
    state.base != 0
        && state.base % 4096 == 0
        && state.base <= 0xffff_ffff_ffff_ffffu64 - BOOT_ARENA_SEALED_OFFSET - 8
        && state.cursor <= BOOT_ARENA_BYTES
}

pub open spec fn allocation_capsule_semantics(
    state: BootArenaState,
    size: u64,
    align: u64,
    result: AllocatorCapsuleStep,
) -> bool {
    arena_well_formed(result.state)
        && result.state.base == state.base
        && result.state.sealed == state.sealed
        && (result.allocation.ok <==> (
            !state.sealed
                && 0 < size
                && (
                    align == 1
                        || align == 2
                        || align == 4
                        || align == 8
                        || align == 16
                        || align == 32
                        || align == 64
                        || align == 4096
                )
                && {
                    let remainder = state.cursor as int % align as int;
                    let padding = if remainder == 0 { 0 } else { align as int - remainder };
                    padding <= (BOOT_ARENA_BYTES - state.cursor) as int
                        && size as int
                            <= (BOOT_ARENA_BYTES - state.cursor) as int - padding
                }
        ))
        && (result.allocation.ok ==> result.allocation.address != 0)
        && (result.allocation.ok ==> result.allocation.address % align == 0)
        && (result.allocation.ok ==> result.allocation.address >= state.base)
        && (result.allocation.ok ==>
            result.allocation.address + size
                == result.state.base + result.state.cursor)
        && (result.allocation.ok ==> result.state.cursor <= BOOT_ARENA_BYTES)
        && (result.allocation.ok ==> result.allocation.size == size)
        && (result.allocation.ok ==> result.allocation.align == align)
        && (!result.allocation.ok ==> result.state.cursor == state.cursor)
        && (!result.allocation.ok ==> result.allocation.address == 0)
        && (!result.allocation.ok ==> result.allocation.size == 0)
        && (!result.allocation.ok ==> result.allocation.align == 0)
}

pub fn execute_alloc_capsule(
    state: BootArenaState,
    size: u64,
    align: u64,
) -> (result: AllocatorCapsuleStep)
    requires
        arena_well_formed(state),
    ensures
        allocation_capsule_semantics(state, size, align, result),
{
    if state.sealed || size == 0 || !permitted_alignment(align) {
        return AllocatorCapsuleStep {
            state,
            allocation: PointerAllocation { ok: false, address: 0, size: 0, align: 0 },
        };
    }

    let cursor = state.cursor;
    let mask = align - 1;
    let remainder = cursor & mask;
    assert(remainder == cursor % align) by {
        if align == 1 {
            assert((cursor & 0) == cursor % 1) by (bit_vector);
        } else if align == 2 {
            assert((cursor & 1) == cursor % 2) by (bit_vector);
        } else if align == 4 {
            assert((cursor & 3) == cursor % 4) by (bit_vector);
        } else if align == 8 {
            assert((cursor & 7) == cursor % 8) by (bit_vector);
        } else if align == 16 {
            assert((cursor & 15) == cursor % 16) by (bit_vector);
        } else if align == 32 {
            assert((cursor & 31) == cursor % 32) by (bit_vector);
        } else if align == 64 {
            assert((cursor & 63) == cursor % 64) by (bit_vector);
        } else {
            assert(align == 4096);
            assert((cursor & 4095) == cursor % 4096) by (bit_vector);
        }
    }
    let padding = if remainder == 0 { 0 } else { align - remainder };
    let available = BOOT_ARENA_BYTES - cursor;
    if padding > available || size > available - padding {
        return AllocatorCapsuleStep {
            state,
            allocation: PointerAllocation { ok: false, address: 0, size: 0, align: 0 },
        };
    }

    let offset = cursor + padding;
    let next = offset + size;
    AllocatorCapsuleStep {
        state: BootArenaState { base: state.base, cursor: next, sealed: state.sealed },
        allocation: PointerAllocation {
            ok: true,
            address: state.base + offset,
            size,
            align,
        },
    }
}

pub fn decode_execute_alloc_capsule(
    image: AllocCapsuleImage,
    state: BootArenaState,
    size: u64,
    align: u64,
) -> (result: AllocCapsuleExecution)
    requires
        arena_well_formed(state),
    ensures
        result.accepted <==> (
            image.word00 == 0xd285_4867_74f6_8548
                && image.word01 == 0x8548_ff4a_8d48_6274
                && image.word02 == 0x7640_fa83_4859_75ca
                && image.word03 == 0x0000_1000_fa81_4809
                && image.word04 == 0x0100_08bf_8348_4a75
                && image.word05 == 0x0087_8b48_4075_0000
                && image.word06 == 0x0100_003d_4800_0100
                && image.word07 == 0x2c72_c101_4831_7700
                && image.word08 == 0x8148_d121_48da_f748
                && image.word09 == 0x411d_7700_0100_00f9
                && image.word10 == 0xc829_4900_0100_00b8
                && image.word11 == 0xce01_480f_77c6_394c
                && image.word12 == 0x4800_0100_00b7_8948
                && image.word13 == 0x00c3_c031_c30f_048d
        ),
        result.accepted ==> allocation_capsule_semantics(state, size, align, result.step),
        !result.accepted ==> result.step.state.base == state.base,
        !result.accepted ==> result.step.state.cursor == state.cursor,
        !result.accepted ==> result.step.state.sealed == state.sealed,
        !result.accepted ==> !result.step.allocation.ok,
        !result.accepted ==> result.step.allocation.address == 0,
{
    if is_registered_alloc_image(image) {
        AllocCapsuleExecution {
            accepted: true,
            step: execute_alloc_capsule(state, size, align),
        }
    } else {
        AllocCapsuleExecution {
            accepted: false,
            step: AllocatorCapsuleStep {
                state,
                allocation: PointerAllocation { ok: false, address: 0, size: 0, align: 0 },
            },
        }
    }
}

pub fn execute_registered_alloc_capsule(
    state: BootArenaState,
    size: u64,
    align: u64,
) -> (result: AllocCapsuleExecution)
    requires
        arena_well_formed(state),
    ensures
        result.accepted,
        allocation_capsule_semantics(state, size, align, result.step),
{
    decode_execute_alloc_capsule(registered_alloc_image(), state, size, align)
}

pub fn execute_seal_capsule(state: BootArenaState) -> (result: BootArenaState)
    requires
        arena_well_formed(state),
    ensures
        arena_well_formed(result),
        result.base == state.base,
        result.cursor == state.cursor,
        result.sealed,
{
    BootArenaState { base: state.base, cursor: state.cursor, sealed: true }
}

pub fn decode_execute_seal_capsule(
    image: PrimitiveCapsuleImage,
    state: BootArenaState,
) -> (result: SealCapsuleExecution)
    requires
        arena_well_formed(state),
    ensures
        result.accepted <==> (
            image.word0 == 0x0100_0100_0887_c748
                && image.word1 == 0x0000_0000_c300_0000
        ),
        result.accepted ==> arena_well_formed(result.state),
        result.accepted ==> result.state.base == state.base,
        result.accepted ==> result.state.cursor == state.cursor,
        result.accepted ==> result.state.sealed,
        !result.accepted ==> result.state.base == state.base,
        !result.accepted ==> result.state.cursor == state.cursor,
        !result.accepted ==> result.state.sealed == state.sealed,
{
    if is_registered_seal_image(image) {
        SealCapsuleExecution { accepted: true, state: execute_seal_capsule(state) }
    } else {
        SealCapsuleExecution { accepted: false, state }
    }
}

pub fn execute_registered_seal_capsule(
    state: BootArenaState,
) -> (result: SealCapsuleExecution)
    requires
        arena_well_formed(state),
    ensures
        result.accepted,
        arena_well_formed(result.state),
        result.state.base == state.base,
        result.state.cursor == state.cursor,
        result.state.sealed,
{
    decode_execute_seal_capsule(registered_seal_image(), state)
}

pub fn execute_global_alloc_adapter(
    state: BootArenaState,
    rust_alloc_size: u64,
    rust_alloc_align: u64,
) -> (result: AllocatorCapsuleStep)
    requires
        arena_well_formed(state),
    ensures
        allocation_capsule_semantics(state, rust_alloc_size, rust_alloc_align, result),
{
    // The registered `__rust_alloc` shim moves SysV RDI (size) to RDX. The
    // registered trait method then places the arena in RDI, size in RSI, and
    // alignment in RDX before tail-calling the allocation capsule.
    execute_alloc_capsule(state, rust_alloc_size, rust_alloc_align)
}

pub fn decode_execute_global_alloc_adapter(
    image: GlobalAllocShimImage,
    relocations: GlobalAllocRelocationPlan,
    state: BootArenaState,
    rust_alloc_size: u64,
    rust_alloc_align: u64,
) -> (result: AllocCapsuleExecution)
    requires
        arena_well_formed(state),
    ensures
        result.accepted <==> (
            image.rust_alloc == 0x0000_0000_e9fa_8948
                && image.rust_null == 0x0000_0000_00c3_c031
                && image.rust_dealloc == 0x0000_0000_0000_00c3
                && image.method_word0 == 0x0000_c7c7_48f0_8948
                && image.method_word1 == 0xc289_48d6_8948_0000
                && image.method_word2 == 0x0000_0000_0000_00e9
                && image.seal_word0 == 0xe900_0000_00c7_c748
                && image.seal_word1 == 0x0000_0000_0000_0000
                && relocations.rust_alloc_dispatch_offset == 4
                && relocations.method_arena_offset == 6
                && relocations.method_capsule_offset == 17
                && relocations.seal_arena_offset == 3
                && relocations.seal_capsule_offset == 8
        ),
        result.accepted ==>
            allocation_capsule_semantics(
                state,
                rust_alloc_size,
                rust_alloc_align,
                result.step,
            ),
        !result.accepted ==> result.step.state.base == state.base,
        !result.accepted ==> result.step.state.cursor == state.cursor,
        !result.accepted ==> result.step.state.sealed == state.sealed,
        !result.accepted ==> !result.step.allocation.ok,
        !result.accepted ==> result.step.allocation.address == 0,
{
    if is_registered_global_alloc_shim(image, relocations) {
        AllocCapsuleExecution {
            accepted: true,
            step: execute_global_alloc_adapter(state, rust_alloc_size, rust_alloc_align),
        }
    } else {
        AllocCapsuleExecution {
            accepted: false,
            step: AllocatorCapsuleStep {
                state,
                allocation: PointerAllocation { ok: false, address: 0, size: 0, align: 0 },
            },
        }
    }
}

pub fn execute_registered_global_alloc_adapter(
    state: BootArenaState,
    rust_alloc_size: u64,
    rust_alloc_align: u64,
) -> (result: AllocCapsuleExecution)
    requires
        arena_well_formed(state),
    ensures
        result.accepted,
        allocation_capsule_semantics(
            state,
            rust_alloc_size,
            rust_alloc_align,
            result.step,
        ),
{
    decode_execute_global_alloc_adapter(
        registered_global_alloc_shim(),
        registered_global_alloc_relocations(),
        state,
        rust_alloc_size,
        rust_alloc_align,
    )
}

pub fn execute_global_alloc_null_path() -> (result: u64)
    ensures
        result == 0,
{
    0
}

pub struct MemoryObservation {
    pub return_address: u64,
    pub observed_address: u64,
    pub byte: u8,
}

pub fn execute_memcpy_observation(
    destination: u64,
    source: u64,
    length: u64,
    observed_address: u64,
    destination_byte_before: u8,
    corresponding_source_byte_before: u8,
    direction_flag_clear: bool,
) -> (result: MemoryObservation)
    requires
        direction_flag_clear,
        destination <= 0xffff_ffff_ffff_ffffu64 - length,
        source <= 0xffff_ffff_ffff_ffffu64 - length,
        destination + length <= source || source + length <= destination,
    ensures
        result.return_address == destination,
        result.observed_address == observed_address,
        destination <= observed_address && observed_address < destination + length ==>
            result.byte == corresponding_source_byte_before,
        !(destination <= observed_address && observed_address < destination + length) ==>
            result.byte == destination_byte_before,
{
    MemoryObservation {
        return_address: destination,
        observed_address,
        byte: if destination <= observed_address && observed_address < destination + length {
            corresponding_source_byte_before
        } else {
            destination_byte_before
        },
    }
}

pub fn decode_execute_memcpy_observation(
    image: PrimitiveCapsuleImage,
    destination: u64,
    source: u64,
    length: u64,
    observed_address: u64,
    destination_byte_before: u8,
    corresponding_source_byte_before: u8,
    direction_flag_clear: bool,
) -> (result: MemoryCapsuleExecution)
    requires
        direction_flag_clear,
        destination <= 0xffff_ffff_ffff_ffffu64 - length,
        source <= 0xffff_ffff_ffff_ffffu64 - length,
        destination + length <= source || source + length <= destination,
    ensures
        result.accepted <==> (
            image.word0 == 0xa4f3_d189_48f8_8948
                && image.word1 == 0x0000_0000_0000_00c3
        ),
        result.accepted ==> result.observation.return_address == destination,
        result.accepted ==> result.observation.observed_address == observed_address,
        result.accepted ==>
            destination <= observed_address && observed_address < destination + length ==>
                result.observation.byte == corresponding_source_byte_before,
        result.accepted ==>
            !(destination <= observed_address && observed_address < destination + length) ==>
                result.observation.byte == destination_byte_before,
        !result.accepted ==> result.observation.return_address == destination,
        !result.accepted ==> result.observation.observed_address == observed_address,
        !result.accepted ==> result.observation.byte == destination_byte_before,
{
    if is_registered_memcpy_image(image) {
        MemoryCapsuleExecution {
            accepted: true,
            observation: execute_memcpy_observation(
                destination,
                source,
                length,
                observed_address,
                destination_byte_before,
                corresponding_source_byte_before,
                direction_flag_clear,
            ),
        }
    } else {
        MemoryCapsuleExecution {
            accepted: false,
            observation: MemoryObservation {
                return_address: destination,
                observed_address,
                byte: destination_byte_before,
            },
        }
    }
}

pub fn execute_registered_memcpy_observation(
    destination: u64,
    source: u64,
    length: u64,
    observed_address: u64,
    destination_byte_before: u8,
    corresponding_source_byte_before: u8,
    direction_flag_clear: bool,
) -> (result: MemoryCapsuleExecution)
    requires
        direction_flag_clear,
        destination <= 0xffff_ffff_ffff_ffffu64 - length,
        source <= 0xffff_ffff_ffff_ffffu64 - length,
        destination + length <= source || source + length <= destination,
    ensures
        result.accepted,
        result.observation.return_address == destination,
        result.observation.observed_address == observed_address,
        destination <= observed_address && observed_address < destination + length ==>
            result.observation.byte == corresponding_source_byte_before,
        !(destination <= observed_address && observed_address < destination + length) ==>
            result.observation.byte == destination_byte_before,
{
    decode_execute_memcpy_observation(
        registered_memcpy_image(),
        destination,
        source,
        length,
        observed_address,
        destination_byte_before,
        corresponding_source_byte_before,
        direction_flag_clear,
    )
}

pub fn execute_memset_observation(
    destination: u64,
    value: u8,
    length: u64,
    observed_address: u64,
    destination_byte_before: u8,
    direction_flag_clear: bool,
) -> (result: MemoryObservation)
    requires
        direction_flag_clear,
        destination <= 0xffff_ffff_ffff_ffffu64 - length,
    ensures
        result.return_address == destination,
        result.observed_address == observed_address,
        destination <= observed_address && observed_address < destination + length ==>
            result.byte == value,
        !(destination <= observed_address && observed_address < destination + length) ==>
            result.byte == destination_byte_before,
{
    MemoryObservation {
        return_address: destination,
        observed_address,
        byte: if destination <= observed_address && observed_address < destination + length {
            value
        } else {
            destination_byte_before
        },
    }
}

pub fn decode_execute_memset_observation(
    image: PrimitiveCapsuleImage,
    destination: u64,
    value: u8,
    length: u64,
    observed_address: u64,
    destination_byte_before: u8,
    direction_flag_clear: bool,
) -> (result: MemoryCapsuleExecution)
    requires
        direction_flag_clear,
        destination <= 0xffff_ffff_ffff_ffffu64 - length,
    ensures
        result.accepted <==> (
            image.word0 == 0xf089_d189_48f8_8949
                && image.word1 == 0x0000_c3c0_894c_aaf3
        ),
        result.accepted ==> result.observation.return_address == destination,
        result.accepted ==> result.observation.observed_address == observed_address,
        result.accepted ==>
            destination <= observed_address && observed_address < destination + length ==>
                result.observation.byte == value,
        result.accepted ==>
            !(destination <= observed_address && observed_address < destination + length) ==>
                result.observation.byte == destination_byte_before,
        !result.accepted ==> result.observation.return_address == destination,
        !result.accepted ==> result.observation.observed_address == observed_address,
        !result.accepted ==> result.observation.byte == destination_byte_before,
{
    if is_registered_memset_image(image) {
        MemoryCapsuleExecution {
            accepted: true,
            observation: execute_memset_observation(
                destination,
                value,
                length,
                observed_address,
                destination_byte_before,
                direction_flag_clear,
            ),
        }
    } else {
        MemoryCapsuleExecution {
            accepted: false,
            observation: MemoryObservation {
                return_address: destination,
                observed_address,
                byte: destination_byte_before,
            },
        }
    }
}

pub fn execute_registered_memset_observation(
    destination: u64,
    value: u8,
    length: u64,
    observed_address: u64,
    destination_byte_before: u8,
    direction_flag_clear: bool,
) -> (result: MemoryCapsuleExecution)
    requires
        direction_flag_clear,
        destination <= 0xffff_ffff_ffff_ffffu64 - length,
    ensures
        result.accepted,
        result.observation.return_address == destination,
        result.observation.observed_address == observed_address,
        destination <= observed_address && observed_address < destination + length ==>
            result.observation.byte == value,
        !(destination <= observed_address && observed_address < destination + length) ==>
            result.observation.byte == destination_byte_before,
{
    decode_execute_memset_observation(
        registered_memset_image(),
        destination,
        value,
        length,
        observed_address,
        destination_byte_before,
        direction_flag_clear,
    )
}

}
