#![no_std]
#![crate_type = "rlib"]

use verus_builtin::*;
use verus_builtin_macros::*;

verus! {

pub struct ByteArena {
    pub base: usize,
    pub cursor: usize,
    pub limit: usize,
}

pub struct ByteAllocation {
    pub ok: bool,
    pub address: usize,
    pub size: usize,
    pub align: usize,
}

fn is_permitted_alignment(align: usize) -> (result: bool)
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

pub fn allocate_bytes(
    state: ByteArena,
    size: usize,
    align: usize,
) -> (result: (ByteArena, ByteAllocation))
    ensures
        result.0.base == state.base,
        result.0.limit == state.limit,
        (0 < state.base && state.base <= state.cursor && state.cursor <= state.limit) ==>
            (0 < result.0.base && result.0.base <= result.0.cursor && result.0.cursor <= result.0.limit),
        result.1.ok <==> (
            0 < state.base
                && state.base <= state.cursor
                && state.cursor <= state.limit
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
                    padding <= (state.limit - state.cursor) as int
                        && size as int <= (state.limit - state.cursor) as int - padding
                }
        ),
        result.1.ok ==>
            (0 < state.base && state.base <= state.cursor && state.cursor <= state.limit),
        result.1.ok ==> 0 < size,
        result.1.ok ==> (
            align == 1
                || align == 2
                || align == 4
                || align == 8
                || align == 16
                || align == 32
                || align == 64
                || align == 4096
        ),
        result.1.ok ==> result.1.address % align == 0,
        result.1.ok ==> result.1.address != 0,
        result.1.ok ==> state.cursor <= result.1.address,
        result.1.ok ==> result.1.address + size == result.0.cursor,
        result.1.ok ==> result.0.cursor <= state.limit,
        result.1.ok ==> result.1.size == size,
        result.1.ok ==> result.1.align == align,
        !result.1.ok ==> result.0.cursor == state.cursor,
        !result.1.ok ==> result.1.address == 0,
        !result.1.ok ==> result.1.size == 0,
        !result.1.ok ==> result.1.align == 0,
{
    if !(0 < state.base && state.base <= state.cursor && state.cursor <= state.limit) {
        return (
            ByteArena { base: state.base, cursor: state.cursor, limit: state.limit },
            ByteAllocation { ok: false, address: 0, size: 0, align: 0 },
        );
    }
    if size == 0 || !is_permitted_alignment(align) {
        return (
            ByteArena { base: state.base, cursor: state.cursor, limit: state.limit },
            ByteAllocation { ok: false, address: 0, size: 0, align: 0 },
        );
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
    let available = state.limit - cursor;
    if padding > available {
        return (
            ByteArena { base: state.base, cursor: state.cursor, limit: state.limit },
            ByteAllocation { ok: false, address: 0, size: 0, align: 0 },
        );
    }
    let after_padding = available - padding;
    if size > after_padding {
        return (
            ByteArena { base: state.base, cursor: state.cursor, limit: state.limit },
            ByteAllocation { ok: false, address: 0, size: 0, align: 0 },
        );
    }

    let address = cursor + padding;
    let next = address + size;
    (
        ByteArena { base: state.base, cursor: next, limit: state.limit },
        ByteAllocation { ok: true, address, size, align },
    )
}

pub fn allocate_two_layouts(
    state: ByteArena,
    first_size: usize,
    first_align: usize,
    second_size: usize,
    second_align: usize,
) -> (result: (ByteArena, ByteAllocation, ByteAllocation))
    ensures
        (0 < state.base && state.base <= state.cursor && state.cursor <= state.limit) ==>
            (0 < result.0.base && result.0.base <= result.0.cursor && result.0.cursor <= result.0.limit),
        result.0.base == state.base,
        result.0.limit == state.limit,
        result.1.ok && result.2.ok ==>
            result.1.address + result.1.size <= result.2.address,
        result.2.ok ==> result.2.address + result.2.size == result.0.cursor,
{
    let (after_first, first) = allocate_bytes(state, first_size, first_align);
    let (after_second, second) = allocate_bytes(after_first, second_size, second_align);
    (after_second, first, second)
}

}
