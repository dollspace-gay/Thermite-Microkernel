#include "kernel_abi.h"

#include <assert.h>
#include <inttypes.h>
#include <stdio.h>

int main(void) {
    assert(TMK_ABI_MAJOR == 1);
    assert(TMK_ABI_MINOR == 0);
    assert(TMK_UTCB_MAGIC == UINT32_C(0x314b4d54));
    assert(TMK_LIMIT_UTCB_BYTES == UINT64_C(1024));
    assert(sizeof(TmkSendCapV1) == 24);
    assert(_Alignof(TmkSendCapV1) == 8);
    assert(sizeof(TmkUtcbV1) == 1024);
    assert(_Alignof(TmkUtcbV1) == 8);
    assert(offsetof(TmkUtcbV1, words) == 24);
    assert(offsetof(TmkUtcbV1, send_caps) == 536);
    assert(offsetof(TmkUtcbV1, reserved_extension) == 680);

    uint64_t tag = (UINT64_C(1) << TMK_MESSAGE_TAG_V1_PROTOCOL_MAJOR_SHIFT) |
                   (UINT64_C(0x1234) << TMK_MESSAGE_TAG_V1_PROTOCOL_ID_SHIFT) |
                   (UINT64_C(0x56) << TMK_MESSAGE_TAG_V1_OPERATION_SHIFT) |
                   (UINT64_C(2) << TMK_MESSAGE_TAG_V1_CAP_COUNT_SHIFT) |
                   UINT64_C(4);
    assert(tmk_message_tag_v1_protocol_major(tag) == 1);
    assert(tmk_message_tag_v1_protocol_id(tag) == UINT64_C(0x1234));
    assert(tmk_message_tag_v1_operation(tag) == UINT64_C(0x56));
    assert(tmk_message_tag_v1_cap_count(tag) == 2);
    assert(tmk_message_tag_v1_word_count(tag) == 4);
    assert(tmk_message_tag_v1_reserved_zero(tag));

    uint64_t cap = (UINT64_C(0xabcd) << TMK_CAP_PTR_V1_ROOT_GUARD_SHIFT) |
                   (UINT64_C(0x1234) << TMK_CAP_PTR_V1_LEVEL_1_SLOT_SHIFT) |
                   (UINT64_C(0x5678) << TMK_CAP_PTR_V1_LEVEL_2_SLOT_SHIFT);
    assert(tmk_cap_ptr_v1_root_guard(cap) == UINT64_C(0xabcd));
    assert(tmk_cap_ptr_v1_level_1_slot(cap) == UINT64_C(0x1234));
    assert(tmk_cap_ptr_v1_level_2_slot(cap) == UINT64_C(0x5678));
    assert(tmk_cap_ptr_v1_reserved_zero(cap));
    assert(!tmk_cap_ptr_v1_reserved_zero(cap | UINT64_C(1)));

    assert(TMK_SYSCALL_ABI_QUERY == UINT64_C(7));
    assert(TMK_K_E_HARDWARE == UINT64_C(13));
    assert(TMK_OP_THREAD_DESTROY == UINT64_C(8));
    assert(TMK_OP_IOMMU_QUERY_FAULT == UINT64_C(5));

    printf("M0_IDL_C_OK:%zu:%zu:%zu:%016" PRIx64 "\n",
           sizeof(TmkUtcbV1), offsetof(TmkUtcbV1, send_caps),
           offsetof(TmkUtcbV1, reserved_extension), tag);
    return 0;
}
