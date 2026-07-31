#![no_std]
#![crate_type = "rlib"]

use verus_builtin::*;
use verus_builtin_macros::*;

verus! {

pub struct BumpState {
    pub next: usize,
    pub end: usize,
}

pub struct Allocation {
    pub ok: bool,
    pub start: usize,
    pub units: usize,
}

pub fn allocate(state: BumpState, units: usize) -> (result: (BumpState, Allocation))
    ensures
        state.next <= state.end ==> result.0.next <= result.0.end,
        result.0.end == state.end,
        result.1.ok <==>
            (state.next <= state.end && 0 < units && units <= state.end - state.next),
        result.1.ok ==> result.1.start == state.next,
        result.1.ok ==> result.1.units == units,
        result.1.ok ==> result.0.next == state.next + units,
        !result.1.ok ==> result.1.start == 0,
        !result.1.ok ==> result.1.units == 0,
        !result.1.ok ==> result.0.next == state.next,
{
    if state.next <= state.end {
        if 0 < units && units <= state.end - state.next {
            (
                BumpState { next: state.next + units, end: state.end },
                Allocation { ok: true, start: state.next, units },
            )
        } else {
            (
                BumpState { next: state.next, end: state.end },
                Allocation { ok: false, start: 0, units: 0 },
            )
        }
    } else {
        (
            BumpState { next: state.next, end: state.end },
            Allocation { ok: false, start: 0, units: 0 },
        )
    }
}

pub fn allocate_pair(
    state: BumpState,
    first_units: usize,
    second_units: usize,
) -> (result: (BumpState, Allocation, Allocation))
    ensures
        state.next <= state.end ==> result.0.next <= result.0.end,
        result.0.end == state.end,
        result.1.ok ==> result.1.start == state.next,
        result.1.ok && result.2.ok ==>
            result.1.start + result.1.units <= result.2.start,
        result.2.ok ==> result.2.start + result.2.units == result.0.next,
{
    let (after_first, first) = allocate(state, first_units);
    let (after_second, second) = allocate(after_first, second_units);
    (after_second, first, second)
}

}
