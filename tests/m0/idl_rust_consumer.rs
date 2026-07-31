#![allow(dead_code)]

mod abi {
    include!(env!("TMK_IDL_RS"));
}

use abi::*;
use core::mem::{align_of, offset_of, size_of};

fn main() {
    assert_eq!(TMK_ABI_MAJOR, 1);
    assert_eq!(TMK_ABI_MINOR, 0);
    assert_eq!(TMK_UTCB_MAGIC, 0x314b4d54);
    assert_eq!(TMK_LIMIT_UTCB_BYTES, 1024);
    assert_eq!(size_of::<TmkSendCapV1>(), 24);
    assert_eq!(align_of::<TmkSendCapV1>(), 8);
    assert_eq!(size_of::<TmkUtcbV1>(), 1024);
    assert_eq!(align_of::<TmkUtcbV1>(), 8);
    assert_eq!(offset_of!(TmkUtcbV1, words), 24);
    assert_eq!(offset_of!(TmkUtcbV1, send_caps), 536);
    assert_eq!(offset_of!(TmkUtcbV1, reserved_extension), 680);

    let tag = (1u64 << TMK_MESSAGE_TAG_V1_PROTOCOL_MAJOR_SHIFT)
        | (0x1234u64 << TMK_MESSAGE_TAG_V1_PROTOCOL_ID_SHIFT)
        | (0x56u64 << TMK_MESSAGE_TAG_V1_OPERATION_SHIFT)
        | (2u64 << TMK_MESSAGE_TAG_V1_CAP_COUNT_SHIFT)
        | 4;
    assert_eq!(tmk_message_tag_v1_protocol_major(tag), 1);
    assert_eq!(tmk_message_tag_v1_protocol_id(tag), 0x1234);
    assert_eq!(tmk_message_tag_v1_operation(tag), 0x56);
    assert_eq!(tmk_message_tag_v1_cap_count(tag), 2);
    assert_eq!(tmk_message_tag_v1_word_count(tag), 4);
    assert!(tmk_message_tag_v1_reserved_zero(tag));

    let cap = (0xabcdu64 << TMK_CAP_PTR_V1_ROOT_GUARD_SHIFT)
        | (0x1234u64 << TMK_CAP_PTR_V1_LEVEL_1_SLOT_SHIFT)
        | (0x5678u64 << TMK_CAP_PTR_V1_LEVEL_2_SLOT_SHIFT);
    assert_eq!(tmk_cap_ptr_v1_root_guard(cap), 0xabcd);
    assert_eq!(tmk_cap_ptr_v1_level_1_slot(cap), 0x1234);
    assert_eq!(tmk_cap_ptr_v1_level_2_slot(cap), 0x5678);
    assert!(tmk_cap_ptr_v1_reserved_zero(cap));
    assert!(!tmk_cap_ptr_v1_reserved_zero(cap | 1));

    assert_eq!(TMK_SYSCALL_ABI_QUERY, 7);
    assert_eq!(TMK_K_E_HARDWARE, 13);
    assert_eq!(TMK_OP_THREAD_DESTROY, 8);
    assert_eq!(TMK_OP_IOMMU_QUERY_FAULT, 5);

    println!(
        "M0_IDL_RUST_OK:{}:{}:{}:{tag:016x}",
        size_of::<TmkUtcbV1>(),
        offset_of!(TmkUtcbV1, send_caps),
        offset_of!(TmkUtcbV1, reserved_extension)
    );
}
