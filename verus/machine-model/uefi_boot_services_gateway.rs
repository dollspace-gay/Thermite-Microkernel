#![no_std]
#![crate_type = "rlib"]

use verus_builtin::*;
use verus_builtin_macros::*;

verus! {

pub const EFI_SYSTEM_TABLE_SIGNATURE: u64 = 0x5453_5953_2049_4249;
pub const EFI_BOOT_SERVICES_SIGNATURE: u64 = 0x5652_4553_544f_4f42;
pub const EFI_BUFFER_TOO_SMALL: u64 = 0x8000_0000_0000_0005;
pub const EFI_LOAD_ERROR: u64 = 0x8000_0000_0000_0001;
pub const EFI_SYSTEM_TABLE_BOOT_SERVICES_OFFSET: u64 = 96;
pub const EFI_BOOT_SERVICES_GET_MEMORY_MAP_OFFSET: u64 = 56;
pub const EFI_SYSTEM_TABLE_REQUIRED_BYTES: u32 = 104;
pub const EFI_BOOT_SERVICES_REQUIRED_BYTES: u32 = 64;
pub const GATEWAY_STACK_FRAME_BYTES: u64 = 104;
pub const GATEWAY_SHADOW_BYTES: u64 = 32;
pub const MEMORY_MAP_SIZE_LIMIT: u64 = 1024 * 1024;

pub struct GatewayImage {
    pub q00: u64,
    pub q01: u64,
    pub q02: u64,
    pub q03: u64,
    pub q04: u64,
    pub q05: u64,
    pub q06: u64,
    pub q07: u64,
    pub q08: u64,
    pub q09: u64,
    pub q10: u64,
    pub q11: u64,
    pub q12: u64,
    pub q13: u64,
    pub q14: u64,
    pub q15: u64,
    pub q16: u64,
    pub q17: u64,
    pub q18: u64,
    pub q19: u64,
    pub q20: u64,
    pub q21: u64,
    pub q22: u64,
    pub q23: u64,
    pub q24: u64,
    pub q25: u64,
    pub q26: u64,
    pub q27: u64,
    pub q28: u64,
    pub q29: u64,
    pub q30: u64,
    pub q31: u64,
    pub q32: u64,
    pub q33: u64,
    pub q34: u64,
    pub q35: u64,
    pub q36: u64,
    pub q37: u64,
    pub tail: u32,
}

pub struct GatewayState {
    pub long_mode: bool,
    pub identity_mapped: bool,
    pub direction_flag: bool,
    pub entry_rsp: u64,
    pub return_address: u64,
    pub image_handle: u64,
    pub system_table: u64,
    pub system_table_registered: bool,
    pub system_signature: u64,
    pub system_header_size: u32,
    pub boot_services: u64,
    pub boot_services_registered: bool,
    pub boot_signature: u64,
    pub boot_header_size: u32,
    pub get_memory_map_target: u64,
    pub target_registered: bool,
    pub return_stack_registered: bool,
    pub stack_registered: bool,
    pub stack_writable_bytes: u64,
    pub firmware_returns: bool,
    pub firmware_preserves_nonvolatile: bool,
    pub firmware_status: u64,
    pub returned_required_size: u64,
    pub rbx: u64,
    pub rbp: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
}

pub struct GatewayStep {
    pub accepted: bool,
    pub system_header_valid: bool,
    pub boot_header_valid: bool,
    pub target_valid: bool,
    pub call_invoked: bool,
    pub call_site_rsp: u64,
    pub stack_aligned: bool,
    pub shadow_bytes: u64,
    pub arg_memory_map_size: u64,
    pub arg_memory_map: u64,
    pub arg_map_key: u64,
    pub arg_descriptor_size: u64,
    pub fifth_arg_slot: u64,
    pub arg_descriptor_version: u64,
    pub observed_status: u64,
    pub observed_required_size: u64,
    pub marker0: u64,
    pub marker1: u64,
    pub marker2: u32,
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

pub open spec fn gateway_image_registered(image: &GatewayImage) -> bool {
    image.q00 == 0x0001_2084_0fd2_8548
        && image.q01 == 0x0000_0007_c2f7_4800
        && image.q02 == 0xba49_0000_0113_850f
        && image.q03 == 0x5453_5953_2049_4249
        && image.q04 == 0x0001_0085_0f12_394c
        && image.q05 == 0xf682_0f68_0c7a_8300
        && image.q06 == 0x4d60_5a8b_4c00_0000
        && image.q07 == 0x0000_00e9_840f_db85
        && image.q08 == 0x0f00_0000_07c3_f749
        && image.q09 == 0x42ba_4900_0000_dc85
        && image.q10 == 0x4d56_5245_5354_4f4f
        && image.q11 == 0x0000_00c9_850f_1339
        && image.q12 == 0xbe82_0f40_0c7b_8341
        && image.q13 == 0x4d38_538b_4d00_0000
        && image.q14 == 0x0000_00b1_840f_d285
        && image.q15 == 0x2444_c748_68ec_8348
        && image.q16 == 0x44c7_4800_0000_0028
        && image.q17 == 0xc748_0000_0000_3024
        && image.q18 == 0xc700_0000_0038_2444
        && image.q19 == 0x4800_0000_0040_2444
        && image.q20 == 0x8d4c_d231_2824_4c8d
        && image.q21 == 0x3824_4c8d_4c30_2444
        && image.q22 == 0x4489_4840_2444_8d48
        && image.q23 == 0x05ba_49d2_ff41_2024
        && image.q24 == 0x4c80_0000_0000_0000
        && image.q25 == 0x2444_8b48_5975_d039
        && image.q26 == 0x3d48_4f74_c085_4828
        && image.q27 == 0x8348_4777_0010_0000
        && image.q28 == 0x54b0_00e9_ba66_68c4
        && image.q29 == 0xb0ee_4bb0_ee4d_b0ee
        && image.q30 == 0xee31_b0ee_4db0_ee5f
        && image.q31 == 0x45b0_ee55_b0ee_5fb0
        && image.q32 == 0xb0ee_49b0_ee46_b0ee
        && image.q33 == 0xee41_b0ee_47b0_ee5f
        && image.q34 == 0x5fb0_ee45_b0ee_54b0
        && image.q35 == 0xb0ee_4bb0_ee4f_b0ee
        && image.q36 == 0xc483_48c3_c031_ee0a
        && image.q37 == 0x0000_0000_01b8_4868
        && image.tail == 0xc380_0000
}

pub open spec fn gateway_environment(state: &GatewayState) -> bool {
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
        && (!target_is_valid(state)
            || (state.entry_rsp >= GATEWAY_STACK_FRAME_BYTES
                && state.target_registered
                && state.stack_registered
                && state.stack_writable_bytes >= GATEWAY_STACK_FRAME_BYTES
                && state.firmware_returns
                && state.firmware_preserves_nonvolatile))
}

pub open spec fn system_header_is_valid(state: &GatewayState) -> bool {
    state.system_table != 0
        && state.system_table % 8 == 0
        && state.system_signature == EFI_SYSTEM_TABLE_SIGNATURE
        && state.system_header_size >= EFI_SYSTEM_TABLE_REQUIRED_BYTES
}

pub open spec fn boot_header_is_valid(state: &GatewayState) -> bool {
    system_header_is_valid(state)
        && state.boot_services != 0
        && state.boot_services % 8 == 0
        && state.boot_signature == EFI_BOOT_SERVICES_SIGNATURE
        && state.boot_header_size >= EFI_BOOT_SERVICES_REQUIRED_BYTES
}

pub open spec fn target_is_valid(state: &GatewayState) -> bool {
    boot_header_is_valid(state) && state.get_memory_map_target != 0
}

pub open spec fn probe_succeeds(state: &GatewayState) -> bool {
    target_is_valid(state)
        && state.firmware_status == EFI_BUFFER_TOO_SMALL
        && state.returned_required_size > 0
        && state.returned_required_size <= MEMORY_MAP_SIZE_LIMIT
}

pub fn image_is_registered(image: &GatewayImage) -> (result: bool)
    ensures result <==> gateway_image_registered(image)
{
    image.q00 == 0x0001_2084_0fd2_8548
        && image.q01 == 0x0000_0007_c2f7_4800
        && image.q02 == 0xba49_0000_0113_850f
        && image.q03 == 0x5453_5953_2049_4249
        && image.q04 == 0x0001_0085_0f12_394c
        && image.q05 == 0xf682_0f68_0c7a_8300
        && image.q06 == 0x4d60_5a8b_4c00_0000
        && image.q07 == 0x0000_00e9_840f_db85
        && image.q08 == 0x0f00_0000_07c3_f749
        && image.q09 == 0x42ba_4900_0000_dc85
        && image.q10 == 0x4d56_5245_5354_4f4f
        && image.q11 == 0x0000_00c9_850f_1339
        && image.q12 == 0xbe82_0f40_0c7b_8341
        && image.q13 == 0x4d38_538b_4d00_0000
        && image.q14 == 0x0000_00b1_840f_d285
        && image.q15 == 0x2444_c748_68ec_8348
        && image.q16 == 0x44c7_4800_0000_0028
        && image.q17 == 0xc748_0000_0000_3024
        && image.q18 == 0xc700_0000_0038_2444
        && image.q19 == 0x4800_0000_0040_2444
        && image.q20 == 0x8d4c_d231_2824_4c8d
        && image.q21 == 0x3824_4c8d_4c30_2444
        && image.q22 == 0x4489_4840_2444_8d48
        && image.q23 == 0x05ba_49d2_ff41_2024
        && image.q24 == 0x4c80_0000_0000_0000
        && image.q25 == 0x2444_8b48_5975_d039
        && image.q26 == 0x3d48_4f74_c085_4828
        && image.q27 == 0x8348_4777_0010_0000
        && image.q28 == 0x54b0_00e9_ba66_68c4
        && image.q29 == 0xb0ee_4bb0_ee4d_b0ee
        && image.q30 == 0xee31_b0ee_4db0_ee5f
        && image.q31 == 0x45b0_ee55_b0ee_5fb0
        && image.q32 == 0xb0ee_49b0_ee46_b0ee
        && image.q33 == 0xee41_b0ee_47b0_ee5f
        && image.q34 == 0x5fb0_ee45_b0ee_54b0
        && image.q35 == 0xb0ee_4bb0_ee4f_b0ee
        && image.q36 == 0xc483_48c3_c031_ee0a
        && image.q37 == 0x0000_0000_01b8_4868
        && image.tail == 0xc380_0000
}

pub fn environment_is_registered(state: &GatewayState) -> (result: bool)
    ensures result <==> gateway_environment(state)
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
    let target_valid = boot_valid && state.get_memory_map_target != 0;
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
        && (!target_valid
            || (state.entry_rsp >= GATEWAY_STACK_FRAME_BYTES
                && state.target_registered
                && state.stack_registered
                && state.stack_writable_bytes >= GATEWAY_STACK_FRAME_BYTES
                && state.firmware_returns
                && state.firmware_preserves_nonvolatile))
}

pub fn registered_gateway_image() -> (result: GatewayImage)
    ensures gateway_image_registered(&result)
{
    GatewayImage {
        q00: 0x0001_2084_0fd2_8548,
        q01: 0x0000_0007_c2f7_4800,
        q02: 0xba49_0000_0113_850f,
        q03: 0x5453_5953_2049_4249,
        q04: 0x0001_0085_0f12_394c,
        q05: 0xf682_0f68_0c7a_8300,
        q06: 0x4d60_5a8b_4c00_0000,
        q07: 0x0000_00e9_840f_db85,
        q08: 0x0f00_0000_07c3_f749,
        q09: 0x42ba_4900_0000_dc85,
        q10: 0x4d56_5245_5354_4f4f,
        q11: 0x0000_00c9_850f_1339,
        q12: 0xbe82_0f40_0c7b_8341,
        q13: 0x4d38_538b_4d00_0000,
        q14: 0x0000_00b1_840f_d285,
        q15: 0x2444_c748_68ec_8348,
        q16: 0x44c7_4800_0000_0028,
        q17: 0xc748_0000_0000_3024,
        q18: 0xc700_0000_0038_2444,
        q19: 0x4800_0000_0040_2444,
        q20: 0x8d4c_d231_2824_4c8d,
        q21: 0x3824_4c8d_4c30_2444,
        q22: 0x4489_4840_2444_8d48,
        q23: 0x05ba_49d2_ff41_2024,
        q24: 0x4c80_0000_0000_0000,
        q25: 0x2444_8b48_5975_d039,
        q26: 0x3d48_4f74_c085_4828,
        q27: 0x8348_4777_0010_0000,
        q28: 0x54b0_00e9_ba66_68c4,
        q29: 0xb0ee_4bb0_ee4d_b0ee,
        q30: 0xee31_b0ee_4db0_ee5f,
        q31: 0x45b0_ee55_b0ee_5fb0,
        q32: 0xb0ee_49b0_ee46_b0ee,
        q33: 0xee41_b0ee_47b0_ee5f,
        q34: 0x5fb0_ee45_b0ee_54b0,
        q35: 0xb0ee_4bb0_ee4f_b0ee,
        q36: 0xc483_48c3_c031_ee0a,
        q37: 0x0000_0000_01b8_4868,
        tail: 0xc380_0000,
    }
}

pub fn decode_execute_gateway(image: GatewayImage, state: GatewayState) -> (result: GatewayStep)
    ensures
        result.accepted <==> gateway_image_registered(&image) && gateway_environment(&state),
        result.accepted ==> result.system_header_valid == system_header_is_valid(&state),
        result.accepted ==> result.boot_header_valid == boot_header_is_valid(&state),
        result.accepted ==> result.target_valid == target_is_valid(&state),
        result.accepted ==> result.call_invoked == target_is_valid(&state),
        result.accepted && result.call_invoked ==> result.call_site_rsp == state.entry_rsp - GATEWAY_STACK_FRAME_BYTES,
        result.accepted && result.call_invoked ==> result.stack_aligned,
        result.accepted && result.call_invoked ==> result.shadow_bytes == GATEWAY_SHADOW_BYTES,
        result.accepted && result.call_invoked ==> result.arg_memory_map_size == state.entry_rsp - 64,
        result.accepted && result.call_invoked ==> result.arg_memory_map == 0,
        result.accepted && result.call_invoked ==> result.arg_map_key == state.entry_rsp - 56,
        result.accepted && result.call_invoked ==> result.arg_descriptor_size == state.entry_rsp - 48,
        result.accepted && result.call_invoked ==> result.fifth_arg_slot == state.entry_rsp - 72,
        result.accepted && result.call_invoked ==> result.arg_descriptor_version == state.entry_rsp - 40,
        result.accepted && result.call_invoked ==> result.observed_status == state.firmware_status,
        result.accepted && result.call_invoked ==> result.observed_required_size == state.returned_required_size,
        result.accepted ==> (result.marker_bytes == 20 <==> probe_succeeds(&state)),
        result.accepted && result.marker_bytes == 20 ==> result.marker0 == 0x555f_314d_5f4b_4d54,
        result.accepted && result.marker_bytes == 20 ==> result.marker1 == 0x4554_4147_5f49_4645,
        result.accepted && result.marker_bytes == 20 ==> result.marker2 == 0x0a4b_4f5f,
        result.accepted ==> (result.rax == 0 <==> probe_succeeds(&state)),
        result.accepted && !probe_succeeds(&state) ==> result.rax == EFI_LOAD_ERROR,
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
        result.accepted && !system_header_is_valid(&state) ==> !result.call_invoked,
        result.accepted && !boot_header_is_valid(&state) ==> !result.call_invoked,
        result.accepted && state.get_memory_map_target == 0 ==> !result.call_invoked,
{
    if image_is_registered(&image) && environment_is_registered(&state) {
        let system_valid = state.system_table != 0
            && state.system_table % 8 == 0
            && state.system_signature == EFI_SYSTEM_TABLE_SIGNATURE
            && state.system_header_size >= EFI_SYSTEM_TABLE_REQUIRED_BYTES;
        let boot_valid = system_valid
            && state.boot_services != 0
            && state.boot_services % 8 == 0
            && state.boot_signature == EFI_BOOT_SERVICES_SIGNATURE
            && state.boot_header_size >= EFI_BOOT_SERVICES_REQUIRED_BYTES;
        let target_valid = boot_valid && state.get_memory_map_target != 0;
        let success = target_valid
            && state.firmware_status == EFI_BUFFER_TOO_SMALL
            && state.returned_required_size > 0
            && state.returned_required_size <= MEMORY_MAP_SIZE_LIMIT;
        GatewayStep {
            accepted: true,
            system_header_valid: system_valid,
            boot_header_valid: boot_valid,
            target_valid,
            call_invoked: target_valid,
            call_site_rsp: if target_valid { state.entry_rsp - GATEWAY_STACK_FRAME_BYTES } else { 0 },
            stack_aligned: target_valid && (state.entry_rsp - GATEWAY_STACK_FRAME_BYTES) % 16 == 0,
            shadow_bytes: if target_valid { GATEWAY_SHADOW_BYTES } else { 0 },
            arg_memory_map_size: if target_valid { state.entry_rsp - 64 } else { 0 },
            arg_memory_map: 0,
            arg_map_key: if target_valid { state.entry_rsp - 56 } else { 0 },
            arg_descriptor_size: if target_valid { state.entry_rsp - 48 } else { 0 },
            fifth_arg_slot: if target_valid { state.entry_rsp - 72 } else { 0 },
            arg_descriptor_version: if target_valid { state.entry_rsp - 40 } else { 0 },
            observed_status: if target_valid { state.firmware_status } else { 0 },
            observed_required_size: if target_valid { state.returned_required_size } else { 0 },
            marker0: if success { 0x555f_314d_5f4b_4d54 } else { 0 },
            marker1: if success { 0x4554_4147_5f49_4645 } else { 0 },
            marker2: if success { 0x0a4b_4f5f } else { 0 },
            marker_bytes: if success { 20 } else { 0 },
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
        GatewayStep {
            accepted: false,
            system_header_valid: false,
            boot_header_valid: false,
            target_valid: false,
            call_invoked: false,
            call_site_rsp: 0,
            stack_aligned: false,
            shadow_bytes: 0,
            arg_memory_map_size: 0,
            arg_memory_map: 0,
            arg_map_key: 0,
            arg_descriptor_size: 0,
            fifth_arg_slot: 0,
            arg_descriptor_version: 0,
            observed_status: 0,
            observed_required_size: 0,
            marker0: 0,
            marker1: 0,
            marker2: 0,
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

pub fn execute_registered_gateway(state: GatewayState) -> (result: GatewayStep)
    ensures
        result.accepted <==> gateway_environment(&state),
        result.accepted ==> result.call_invoked == target_is_valid(&state),
        result.accepted ==> (result.marker_bytes == 20 <==> probe_succeeds(&state)),
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
    decode_execute_gateway(registered_gateway_image(), state)
}

}
