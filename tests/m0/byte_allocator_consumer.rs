extern crate tmk_byte_allocator;

use tmk_byte_allocator::{allocate_bytes, allocate_two_layouts, ByteArena};

fn main() {
    let initial = ByteArena {
        base: 0x20_0000,
        cursor: 0x20_0003,
        limit: 0x20_1000,
    };
    let (after, first, second) = allocate_two_layouts(initial, 13, 8, 16, 16);
    assert!(first.ok && second.ok);
    assert_eq!((first.address, first.size, first.align), (0x20_0008, 13, 8));
    assert_eq!(
        (second.address, second.size, second.align),
        (0x20_0020, 16, 16)
    );
    assert_eq!((after.base, after.cursor, after.limit), (0x20_0000, 0x20_0030, 0x20_1000));

    for align in [0, 3, 128] {
        let (unchanged, rejected) = allocate_bytes(
            ByteArena { base: after.base, cursor: after.cursor, limit: after.limit },
            8,
            align,
        );
        assert!(!rejected.ok);
        assert_eq!(unchanged.cursor, after.cursor);
    }

    let (unchanged, zero) = allocate_bytes(
        ByteArena { base: after.base, cursor: after.cursor, limit: after.limit },
        0,
        8,
    );
    assert!(!zero.ok);
    assert_eq!(unchanged.cursor, after.cursor);

    let near_end = ByteArena {
        base: 0x20_0000,
        cursor: 0x20_0ff9,
        limit: 0x20_1000,
    };
    let (near_end_unchanged, exhausted) = allocate_bytes(near_end, 8, 8);
    assert!(!exhausted.ok);
    assert_eq!(near_end_unchanged.cursor, 0x20_0ff9);

    let corrupt = ByteArena { base: 9, cursor: 8, limit: 16 };
    let (corrupt_unchanged, rejected) = allocate_bytes(corrupt, 1, 1);
    assert!(!rejected.ok);
    assert_eq!(
        (corrupt_unchanged.base, corrupt_unchanged.cursor, corrupt_unchanged.limit),
        (9, 8, 16)
    );

    let overflow_edge = ByteArena {
        base: usize::MAX - 15,
        cursor: usize::MAX - 7,
        limit: usize::MAX,
    };
    let (overflow_unchanged, rejected) = allocate_bytes(overflow_edge, 8, 8);
    assert!(!rejected.ok);
    assert_eq!(overflow_unchanged.cursor, usize::MAX - 7);

    println!("M0_BYTE_ALLOCATOR_OK:200008:200020:200030");
}
