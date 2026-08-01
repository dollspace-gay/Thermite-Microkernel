#![no_std]
#![crate_type = "rlib"]

use verus_builtin::*;
use verus_builtin_macros::*;
use vstd::prelude::*;

verus! {

pub const EFI_SYSTEM_TABLE_SIGNATURE: u64 = 0x5453_5953_2049_4249;
pub const EFI_BOOT_SERVICES_SIGNATURE: u64 = 0x5652_4553_544f_4f42;
pub const EFI_BUFFER_TOO_SMALL: u64 = 0x8000_0000_0000_0005;
pub const EFI_LOAD_ERROR: u64 = 0x8000_0000_0000_0001;
pub const EFI_SYSTEM_TABLE_REQUIRED_BYTES: u32 = 104;
pub const EFI_BOOT_SERVICES_REQUIRED_BYTES: u32 = 80;
pub const RAW_MAP_STACK_FRAME_BYTES: u64 = 168;
pub const EFI_SHADOW_SPACE_BYTES: u64 = 32;
pub const ALLOCATION_MARGIN: u64 = 512;
pub const PROBE_SIZE_LIMIT: u64 = 1_048_064;
pub const RAW_MAP_SIZE_LIMIT: u64 = 1_048_576;
pub const DESCRIPTOR_LIMIT: u64 = 4_096;
pub const PAGE_LIMIT: u64 = 1_099_511_627_776;
pub const RAW_MAP_IMAGE_WORD_COUNT: usize = 127;

pub const RAW_MAP_IMAGE_WORDS: [u64; RAW_MAP_IMAGE_WORD_COUNT] = [
    0x0003e4840fd28548, 0x03db850f07c2f600,
    0x49ba491a8b4c0000, 0x4d54535953204942,
    0x000003c5850fd339, 0x03bb820f680c7a83,
    0x854d605a8b4c0000, 0x41000003ae840fdb,
    0x0003a4850f07c3f6, 0x4f42b848138b4d00,
    0x394956524553544f, 0x410000038e850fc2,
    0x0383820f500c7b83, 0x0f00387b83490000,
    0x7b83490000037884, 0x0000036d840f0040,
    0x62840f00487b8349, 0x00a8ec8148000003,
    0x4828245c894c0000, 0x00000000302444c7,
    0x000000382444c748, 0x0000402444c74800,
    0x00482444c7480000, 0x502444c748000000,
    0x244c8d4800000000, 0x3824448d4cd23130,
    0x448d4840244c8d4c, 0x4120244489484824,
    0x000005ba493853ff, 0xd0394c8000000000,
    0x8b48000002f0850f, 0x840fc08548302444,
    0xfe003d48000002e2, 0x000002d6870f000f,
    0x8948000002000548, 0x0002c1c748582444,
    0x448d4cc289480000, 0x4128245c8b4c5024,
    0x850fc085484053ff, 0x24448b48000002aa,
    0x029c840fc0854850, 0x0294850f07a80000,
    0x00802484c7480000, 0x8b48000000000000,
    0x3024448948582444, 0x000000382444c748,
    0x0000402444c74800, 0x00482444c7480000,
    0x30244c8d48000000, 0x448d4c5024548b48,
    0x4840244c8d4c3824, 0x244489484824448d,
    0xff4128245c8b4c20, 0xe6850fc085483853,
    0x3024448b48000001, 0x0001d8840fc08548,
    0x870f5824443b4800, 0x247c8348000001cd,
    0x000001c1840f0038, 0xf9834840244c8b48,
    0x48000001b2820f28, 0x870f00000100f981,
    0x0f07c1f6000001a5, 0x247c830000019c85,
    0x00000191850f0148, 0xd28548f1f748d231,
    0x854800000183850f, 0x480000017a840fc0,
    0x6e870f000010003d, 0x7024448948000001,
    0x4489485024448b48, 0x00602444c7487824,
    0x682444c748000000, 0x24448b4c00000000,
    0x0f0ff983088b4178, 0x508b490000013c87,
    0x850f0fffc2f76608, 0x24543b480000012d,
    0x4900000122820f60, 0x840fc0854818408b,
    0x0000b94900000115, 0x394c000001000000,
    0x4900000102870fc8, 0xba490ce1c149c189,
    0x0010000000000000, 0x870fd2394cca294d,
    0x48ca014c000000e5, 0x10508b4960245489,
    0xce850f0fffc2f766, 0xffffc2c749000000,
    0xd2394cca294dffff, 0x8b4d000000bb870f,
    0xffe00fe0ba492058, 0x0fd3854d30000fff,
    0x00ba49000000a485, 0x4d0ffff000000000,
    0xe3ba0f490b74d385, 0x4d0000008a830f3e,
    0xf9833feac149da89, 0x117406f983167405,
    0x0cf98311740bf983, 0xeb6a75d2854d0c74,
    0xf9836374d2854d05, 0x05740bf9830a7407,
    0xc3f64106750cf983, 0x1f7507f9834e741f,
    0x00ba496824548b48, 0x4900000100000000,
    0x483277d2394cc229, 0x486824548948c201,
    0x244401484024448b, 0x0f0170246c834878,
    0x7c8348fffffec785, 0x84c7480c74006824,
    0x0000010000008024, 0x8b4c50244c8b4800,
    0x484853ff4128245c, 0x24bc83483a75c085,
    0x662f750100000080, 0x4db0ee54b000e9ba,
    0xb0ee5fb0ee4bb0ee, 0xee50b0ee41b0ee4d,
    0x4bb0ee4fb0ee5fb0, 0x8148c031ee0ab0ee,
    0x8148c3000000a8c4, 0x01b848000000a8c4,
    0xc380000000000000,
];

pub struct RawMapImage {
    pub words: [u64; RAW_MAP_IMAGE_WORD_COUNT],
}

pub struct RawMapMachineState {
    pub long_mode: bool,
    pub identity_mapped: bool,
    pub direction_flag: bool,
    pub entry_rsp: u64,
    pub return_address: u64,
    pub system_table: u64,
    pub system_table_registered: bool,
    pub system_signature: u64,
    pub system_header_size: u32,
    pub boot_services: u64,
    pub boot_services_registered: bool,
    pub boot_signature: u64,
    pub boot_header_size: u32,
    pub get_memory_map_target: u64,
    pub allocate_pool_target: u64,
    pub free_pool_target: u64,
    pub targets_registered: bool,
    pub return_stack_registered: bool,
    pub stack_registered: bool,
    pub stack_writable_bytes: u64,
    pub firmware_returns: bool,
    pub firmware_preserves_nonvolatile: bool,
    pub probe_status: u64,
    pub probe_required_size: u64,
    pub allocate_status: u64,
    pub allocated_buffer: u64,
    pub allocated_buffer_registered: bool,
    pub second_status: u64,
    pub returned_size: u64,
    pub map_key: u64,
    pub descriptor_size: u64,
    pub descriptor_version: u32,
    pub descriptor_count: u64,
    pub map_bytes_registered: bool,
    pub descriptors_valid: bool,
    pub usable_pages: u64,
    pub free_status: u64,
    pub rbx: u64,
    pub rbp: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
}

pub struct RawMapMachineStep {
    pub accepted: bool,
    pub system_header_valid: bool,
    pub boot_header_valid: bool,
    pub targets_valid: bool,
    pub probe_called: bool,
    pub allocate_called: bool,
    pub second_map_called: bool,
    pub descriptors_scanned: u64,
    pub free_called: bool,
    pub buffer_owned_at_return: bool,
    pub call_site_rsp: u64,
    pub stack_aligned: bool,
    pub shadow_bytes: u64,
    pub pool_type: u32,
    pub allocation_size: u64,
    pub observed_map_key: u64,
    pub observed_descriptor_size: u64,
    pub observed_descriptor_version: u32,
    pub observed_usable_pages: u64,
    pub marker0: u64,
    pub marker1: u32,
    pub marker_bytes: u8,
    pub rax: u64,
    pub rsp: u64,
    pub post_rip: u64,
    pub rbx: u64,
    pub rbp: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub returned: bool,
    pub nonvolatile_preserved: bool,
}

pub open spec fn raw_map_image_registered(image: &RawMapImage) -> bool {
    image.words@ == RAW_MAP_IMAGE_WORDS@
}

pub open spec fn system_header_is_valid(state: &RawMapMachineState) -> bool {
    state.system_table != 0
        && state.system_table % 8 == 0
        && state.system_signature == EFI_SYSTEM_TABLE_SIGNATURE
        && state.system_header_size >= EFI_SYSTEM_TABLE_REQUIRED_BYTES
}

pub open spec fn boot_header_is_valid(state: &RawMapMachineState) -> bool {
    system_header_is_valid(state)
        && state.boot_services != 0
        && state.boot_services % 8 == 0
        && state.boot_signature == EFI_BOOT_SERVICES_SIGNATURE
        && state.boot_header_size >= EFI_BOOT_SERVICES_REQUIRED_BYTES
}

pub open spec fn targets_are_valid(state: &RawMapMachineState) -> bool {
    boot_header_is_valid(state)
        && state.get_memory_map_target != 0
        && state.allocate_pool_target != 0
        && state.free_pool_target != 0
}

pub open spec fn probe_succeeds(state: &RawMapMachineState) -> bool {
    targets_are_valid(state)
        && state.probe_status == EFI_BUFFER_TOO_SMALL
        && state.probe_required_size > 0
        && state.probe_required_size <= PROBE_SIZE_LIMIT
}

pub open spec fn allocation_succeeds(state: &RawMapMachineState) -> bool {
    probe_succeeds(state)
        && state.allocate_status == 0
        && state.allocated_buffer != 0
        && state.allocated_buffer % 8 == 0
}

pub open spec fn raw_map_shape_valid(state: &RawMapMachineState) -> bool {
    allocation_succeeds(state)
        && state.second_status == 0
        && state.returned_size > 0
        && state.returned_size <= state.probe_required_size + ALLOCATION_MARGIN
        && state.map_key != 0
        && state.descriptor_size >= 40
        && state.descriptor_size <= 256
        && state.descriptor_size % 8 == 0
        && state.descriptor_version == 1
        && state.descriptor_count >= 1
        && state.descriptor_count <= DESCRIPTOR_LIMIT
        && state.returned_size % state.descriptor_size == 0
        && state.returned_size / state.descriptor_size == state.descriptor_count
}

pub open spec fn raw_map_capsule_succeeds(state: &RawMapMachineState) -> bool {
    raw_map_shape_valid(state)
        && state.descriptors_valid
        && state.usable_pages > 0
        && state.usable_pages <= PAGE_LIMIT
        && state.free_status == 0
}

pub open spec fn raw_map_machine_environment(state: &RawMapMachineState) -> bool {
    state.long_mode
        && state.identity_mapped
        && !state.direction_flag
        && state.entry_rsp % 16 == 8
        && state.return_stack_registered
        && (state.system_table == 0
            || state.system_table % 8 != 0
            || state.system_table_registered)
        && (!system_header_is_valid(state)
            || state.boot_services == 0
            || state.boot_services % 8 != 0
            || state.boot_services_registered)
        && (!targets_are_valid(state)
            || (state.entry_rsp >= RAW_MAP_STACK_FRAME_BYTES
                && state.targets_registered
                && state.stack_registered
                && state.stack_writable_bytes >= RAW_MAP_STACK_FRAME_BYTES
                && state.firmware_returns
                && state.firmware_preserves_nonvolatile))
        && (!allocation_succeeds(state) || state.allocated_buffer_registered)
        && (!raw_map_shape_valid(state) || state.map_bytes_registered)
}

pub fn raw_map_image_is_registered(image: &RawMapImage) -> (result: bool)
    ensures result <==> raw_map_image_registered(image),
{
    let mut index: usize = 0;
    while index < RAW_MAP_IMAGE_WORD_COUNT
        invariant
            index <= RAW_MAP_IMAGE_WORD_COUNT,
            forall|prior: int| 0 <= prior < index
                ==> image.words@[prior] == RAW_MAP_IMAGE_WORDS@[prior],
        decreases RAW_MAP_IMAGE_WORD_COUNT - index,
    {
        if image.words[index] != RAW_MAP_IMAGE_WORDS[index] {
            return false;
        }
        index = index + 1;
    }
    assert(image.words@ =~= RAW_MAP_IMAGE_WORDS@);
    true
}

pub fn registered_raw_map_image() -> (result: RawMapImage)
    ensures raw_map_image_registered(&result),
{
    RawMapImage { words: RAW_MAP_IMAGE_WORDS }
}

pub fn environment_is_registered(state: &RawMapMachineState) -> (result: bool)
    ensures result <==> raw_map_machine_environment(state),
{
    let system_valid = state.system_table != 0
        && state.system_table % 8 == 0
        && state.system_signature == EFI_SYSTEM_TABLE_SIGNATURE
        && state.system_header_size >= EFI_SYSTEM_TABLE_REQUIRED_BYTES;
    let boot_valid = system_valid
        && state.boot_services != 0
        && state.boot_services % 8 == 0
        && state.boot_signature == EFI_BOOT_SERVICES_SIGNATURE
        && state.boot_header_size >= EFI_BOOT_SERVICES_REQUIRED_BYTES;
    let targets_valid = boot_valid
        && state.get_memory_map_target != 0
        && state.allocate_pool_target != 0
        && state.free_pool_target != 0;
    let probe_ok = targets_valid
        && state.probe_status == EFI_BUFFER_TOO_SMALL
        && state.probe_required_size > 0
        && state.probe_required_size <= PROBE_SIZE_LIMIT;
    let allocation_ok = probe_ok
        && state.allocate_status == 0
        && state.allocated_buffer != 0
        && state.allocated_buffer % 8 == 0;
    let shape_ok = allocation_ok
        && state.second_status == 0
        && state.returned_size > 0
        && state.returned_size <= state.probe_required_size + ALLOCATION_MARGIN
        && state.map_key != 0
        && state.descriptor_size >= 40
        && state.descriptor_size <= 256
        && state.descriptor_size % 8 == 0
        && state.descriptor_version == 1
        && state.descriptor_count >= 1
        && state.descriptor_count <= DESCRIPTOR_LIMIT
        && state.returned_size % state.descriptor_size == 0
        && state.returned_size / state.descriptor_size == state.descriptor_count;
    state.long_mode
        && state.identity_mapped
        && !state.direction_flag
        && state.entry_rsp % 16 == 8
        && state.return_stack_registered
        && (state.system_table == 0
            || state.system_table % 8 != 0
            || state.system_table_registered)
        && (!system_valid
            || state.boot_services == 0
            || state.boot_services % 8 != 0
            || state.boot_services_registered)
        && (!targets_valid
            || (state.entry_rsp >= RAW_MAP_STACK_FRAME_BYTES
                && state.targets_registered
                && state.stack_registered
                && state.stack_writable_bytes >= RAW_MAP_STACK_FRAME_BYTES
                && state.firmware_returns
                && state.firmware_preserves_nonvolatile))
        && (!allocation_ok || state.allocated_buffer_registered)
        && (!shape_ok || state.map_bytes_registered)
}

pub fn decode_execute_raw_map_capsule(
    image: RawMapImage,
    state: RawMapMachineState,
) -> (result: RawMapMachineStep)
    ensures
        result.accepted <==> raw_map_image_registered(&image)
            && raw_map_machine_environment(&state),
        result.accepted ==> result.system_header_valid == system_header_is_valid(&state),
        result.accepted ==> result.boot_header_valid == boot_header_is_valid(&state),
        result.accepted ==> result.targets_valid == targets_are_valid(&state),
        result.accepted ==> result.probe_called == targets_are_valid(&state),
        result.accepted ==> result.allocate_called == probe_succeeds(&state),
        result.accepted ==> result.second_map_called == allocation_succeeds(&state),
        result.accepted ==> result.free_called == allocation_succeeds(&state),
        result.accepted ==> result.descriptors_scanned
            == if raw_map_shape_valid(&state) { state.descriptor_count } else { 0 },
        result.accepted ==> !result.buffer_owned_at_return
            == (allocation_succeeds(&state) ==> state.free_status == 0),
        result.accepted && result.probe_called ==>
            result.call_site_rsp == state.entry_rsp - RAW_MAP_STACK_FRAME_BYTES,
        result.accepted && result.probe_called ==> result.stack_aligned,
        result.accepted && result.probe_called ==>
            result.shadow_bytes == EFI_SHADOW_SPACE_BYTES,
        result.accepted && result.allocate_called ==> result.pool_type == 2,
        result.accepted && result.allocate_called ==>
            result.allocation_size == state.probe_required_size + ALLOCATION_MARGIN,
        result.accepted && result.second_map_called ==> result.observed_map_key == state.map_key,
        result.accepted && result.second_map_called ==>
            result.observed_descriptor_size == state.descriptor_size,
        result.accepted && result.second_map_called ==>
            result.observed_descriptor_version == state.descriptor_version,
        result.accepted && raw_map_shape_valid(&state) ==>
            result.observed_usable_pages == state.usable_pages,
        result.accepted ==> (result.marker_bytes == 11 <==> raw_map_capsule_succeeds(&state)),
        result.accepted && result.marker_bytes == 11 ==>
            result.marker0 == 0x5f50_414d_5f4b_4d54,
        result.accepted && result.marker_bytes == 11 ==> result.marker1 == 0x000a_4b4f,
        result.accepted ==> (result.rax == 0 <==> raw_map_capsule_succeeds(&state)),
        result.accepted && !raw_map_capsule_succeeds(&state) ==> result.rax == EFI_LOAD_ERROR,
        result.accepted ==> result.rsp == state.entry_rsp,
        result.accepted ==> result.post_rip == state.return_address,
        result.accepted ==> result.rbx == state.rbx,
        result.accepted ==> result.rbp == state.rbp,
        result.accepted ==> result.rdi == state.rdi,
        result.accepted ==> result.rsi == state.rsi,
        result.accepted ==> result.r12 == state.r12,
        result.accepted ==> result.r13 == state.r13,
        result.accepted ==> result.r14 == state.r14,
        result.accepted ==> result.r15 == state.r15,
        result.accepted ==> result.returned,
        result.accepted ==> result.nonvolatile_preserved,
        result.accepted && !targets_are_valid(&state) ==> !result.probe_called,
        result.accepted && !probe_succeeds(&state) ==> !result.allocate_called,
        result.accepted && !allocation_succeeds(&state) ==> !result.free_called,
        result.accepted && raw_map_capsule_succeeds(&state) ==> !result.buffer_owned_at_return,
{
    if raw_map_image_is_registered(&image) && environment_is_registered(&state) {
        let system_valid = state.system_table != 0
            && state.system_table % 8 == 0
            && state.system_signature == EFI_SYSTEM_TABLE_SIGNATURE
            && state.system_header_size >= EFI_SYSTEM_TABLE_REQUIRED_BYTES;
        let boot_valid = system_valid
            && state.boot_services != 0
            && state.boot_services % 8 == 0
            && state.boot_signature == EFI_BOOT_SERVICES_SIGNATURE
            && state.boot_header_size >= EFI_BOOT_SERVICES_REQUIRED_BYTES;
        let targets_valid = boot_valid
            && state.get_memory_map_target != 0
            && state.allocate_pool_target != 0
            && state.free_pool_target != 0;
        let probe_ok = targets_valid
            && state.probe_status == EFI_BUFFER_TOO_SMALL
            && state.probe_required_size > 0
            && state.probe_required_size <= PROBE_SIZE_LIMIT;
        let allocation_ok = probe_ok
            && state.allocate_status == 0
            && state.allocated_buffer != 0
            && state.allocated_buffer % 8 == 0;
        let shape_ok = allocation_ok
            && state.second_status == 0
            && state.returned_size > 0
            && state.returned_size <= state.probe_required_size + ALLOCATION_MARGIN
            && state.map_key != 0
            && state.descriptor_size >= 40
            && state.descriptor_size <= 256
            && state.descriptor_size % 8 == 0
            && state.descriptor_version == 1
            && state.descriptor_count >= 1
            && state.descriptor_count <= DESCRIPTOR_LIMIT
            && state.returned_size % state.descriptor_size == 0
            && state.returned_size / state.descriptor_size == state.descriptor_count;
        let success = shape_ok
            && state.descriptors_valid
            && state.usable_pages > 0
            && state.usable_pages <= PAGE_LIMIT
            && state.free_status == 0;
        RawMapMachineStep {
            accepted: true,
            system_header_valid: system_valid,
            boot_header_valid: boot_valid,
            targets_valid,
            probe_called: targets_valid,
            allocate_called: probe_ok,
            second_map_called: allocation_ok,
            descriptors_scanned: if shape_ok { state.descriptor_count } else { 0 },
            free_called: allocation_ok,
            buffer_owned_at_return: allocation_ok && state.free_status != 0,
            call_site_rsp: if targets_valid {
                state.entry_rsp - RAW_MAP_STACK_FRAME_BYTES
            } else { 0 },
            stack_aligned: targets_valid
                && (state.entry_rsp - RAW_MAP_STACK_FRAME_BYTES) % 16 == 0,
            shadow_bytes: if targets_valid { EFI_SHADOW_SPACE_BYTES } else { 0 },
            pool_type: if probe_ok { 2 } else { 0 },
            allocation_size: if probe_ok {
                state.probe_required_size + ALLOCATION_MARGIN
            } else { 0 },
            observed_map_key: if allocation_ok { state.map_key } else { 0 },
            observed_descriptor_size: if allocation_ok { state.descriptor_size } else { 0 },
            observed_descriptor_version: if allocation_ok {
                state.descriptor_version
            } else { 0 },
            observed_usable_pages: if shape_ok { state.usable_pages } else { 0 },
            marker0: if success { 0x5f50_414d_5f4b_4d54 } else { 0 },
            marker1: if success { 0x000a_4b4f } else { 0 },
            marker_bytes: if success { 11 } else { 0 },
            rax: if success { 0 } else { EFI_LOAD_ERROR },
            rsp: state.entry_rsp,
            post_rip: state.return_address,
            rbx: state.rbx,
            rbp: state.rbp,
            rdi: state.rdi,
            rsi: state.rsi,
            r12: state.r12,
            r13: state.r13,
            r14: state.r14,
            r15: state.r15,
            returned: true,
            nonvolatile_preserved: true,
        }
    } else {
        RawMapMachineStep {
            accepted: false,
            system_header_valid: false,
            boot_header_valid: false,
            targets_valid: false,
            probe_called: false,
            allocate_called: false,
            second_map_called: false,
            descriptors_scanned: 0,
            free_called: false,
            buffer_owned_at_return: false,
            call_site_rsp: 0,
            stack_aligned: false,
            shadow_bytes: 0,
            pool_type: 0,
            allocation_size: 0,
            observed_map_key: 0,
            observed_descriptor_size: 0,
            observed_descriptor_version: 0,
            observed_usable_pages: 0,
            marker0: 0,
            marker1: 0,
            marker_bytes: 0,
            rax: 0,
            rsp: state.entry_rsp,
            post_rip: 0,
            rbx: state.rbx,
            rbp: state.rbp,
            rdi: state.rdi,
            rsi: state.rsi,
            r12: state.r12,
            r13: state.r13,
            r14: state.r14,
            r15: state.r15,
            returned: false,
            nonvolatile_preserved: false,
        }
    }
}

pub fn execute_registered_raw_map_capsule(
    state: RawMapMachineState,
) -> (result: RawMapMachineStep)
    ensures
        result.accepted <==> raw_map_machine_environment(&state),
        result.accepted ==> result.probe_called == targets_are_valid(&state),
        result.accepted ==> result.allocate_called == probe_succeeds(&state),
        result.accepted ==> result.second_map_called == allocation_succeeds(&state),
        result.accepted ==> result.free_called == allocation_succeeds(&state),
        result.accepted ==> (result.marker_bytes == 11 <==> raw_map_capsule_succeeds(&state)),
        result.accepted ==> result.rsp == state.entry_rsp,
        result.accepted ==> result.post_rip == state.return_address,
        result.accepted ==> result.rbx == state.rbx,
        result.accepted ==> result.rbp == state.rbp,
        result.accepted ==> result.rdi == state.rdi,
        result.accepted ==> result.rsi == state.rsi,
        result.accepted ==> result.r12 == state.r12,
        result.accepted ==> result.r13 == state.r13,
        result.accepted ==> result.r14 == state.r14,
        result.accepted ==> result.r15 == state.r15,
        result.accepted ==> result.returned,
        result.accepted ==> result.nonvolatile_preserved,
{
    decode_execute_raw_map_capsule(registered_raw_map_image(), state)
}

}
