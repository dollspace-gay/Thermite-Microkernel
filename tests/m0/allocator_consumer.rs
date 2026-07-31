extern crate tmk_allocator;

use tmk_allocator::{allocate, allocate_pair, BumpState};

fn main() {
    let initial = BumpState { next: 8, end: 16 };
    let (after, first, second) = allocate_pair(initial, 3, 5);
    assert!(first.ok && second.ok);
    assert_eq!((first.start, first.units), (8, 3));
    assert_eq!((second.start, second.units), (11, 5));
    assert_eq!((after.next, after.end), (16, 16));

    let (unchanged, exhausted) = allocate(after, 1);
    assert!(!exhausted.ok);
    assert_eq!((unchanged.next, unchanged.end), (16, 16));

    let (still_unchanged, zero) = allocate(unchanged, 0);
    assert!(!zero.ok);
    assert_eq!((still_unchanged.next, still_unchanged.end), (16, 16));

    let invalid = BumpState { next: 17, end: 16 };
    let (invalid_unchanged, rejected) = allocate(invalid, 1);
    assert!(!rejected.ok);
    assert_eq!((invalid_unchanged.next, invalid_unchanged.end), (17, 16));
    println!("M0_ALLOCATOR_OK:8:11:16");
}
