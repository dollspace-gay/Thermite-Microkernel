#![no_std]
#![crate_type = "rlib"]

use verus_builtin::*;
use verus_builtin_macros::*;

verus! {

pub const GS_SETUP_VIRTUAL: u64 = 0xffff_ffff_8000_1040;
pub const SCALAR_ENTRY_VIRTUAL: u64 = 0xffff_ffff_8001_1200;
pub const SCALAR_WRAPPER_VIRTUAL: u64 = 0xffff_ffff_8001_1300;
pub const FAIL_STOP_VIRTUAL: u64 = 0xffff_ffff_8001_1500;
pub const SCHEDULE_VIRTUAL: u64 = 0xffff_ffff_8001_1600;
pub const ADAPTER_VIRTUAL: u64 = 0xffff_ffff_8001_2000;
pub const COMMON_CONTINUATION: u64 = 0xffff_ffff_8001_1038;
pub const GS_HEADER_FLAGS: u64 = 0x1ff;
pub const SCALAR_CORE_BLOCK_BYTES: u64 = 640;
pub const CONTROL_RETURN: u8 = 0;
pub const CONTROL_SCHEDULE: u8 = 1;
pub const CONTROL_FAIL_STOP: u8 = 2;
pub const KERNEL_CODE_SELECTOR: u64 = 0x08;
pub const USER_DATA_SELECTOR: u64 = 0x1b;
pub const USER_CODE_SELECTOR: u64 = 0x23;

pub struct GsSetupImage {
    pub q00: u64,
    pub q01: u64,
    pub q02: u64,
    pub q03: u64,
    pub tail: u32,
}

pub struct WrapperImage {
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
    pub q38: u64,
    pub tail: u16,
}

pub struct ControlImages {
    pub fail_stop: u32,
    pub schedule_unavailable: u64,
}

pub struct GsSetupState {
    pub cpl: u8,
    pub interrupts_enabled: bool,
    pub gs_base_operand: u64,
    pub kernel_gs_base_operand: u64,
    pub current_gs_header_registered: bool,
    pub kernel_gs_header_registered: bool,
    pub msr_access_registered: bool,
    pub return_stack_registered: bool,
    pub return_address: u64,
}

pub struct GsSetupStep {
    pub accepted: bool,
    pub gs_base: u64,
    pub kernel_gs_base: u64,
    pub wrmsr_count: u8,
    pub returned: bool,
    pub post_rip: u64,
}

pub struct WrapperState {
    pub cpl: u8,
    pub interrupts_enabled: bool,
    pub direction_flag: bool,
    pub rsp: u64,
    pub return_address: u64,
    pub gs_header_registered: bool,
    pub gs_self: u64,
    pub gs_core_block: u64,
    pub gs_active_frame: u64,
    pub gs_flags: u64,
    pub frame_pointer: u64,
    pub frame_prefix_registered: bool,
    pub frame_user_tail_registered: bool,
    pub frame_cr2: u64,
    pub frame_vector: u64,
    pub frame_error: u64,
    pub frame_rip: u64,
    pub frame_cs: u64,
    pub frame_rflags: u64,
    pub frame_user_rsp: u64,
    pub frame_ss: u64,
    pub transport_cr2: u64,
    pub transport_error: u64,
    pub transport_rip: u64,
    pub transport_rflags: u64,
    pub transport_user_rsp: u64,
    pub transport_metadata: u64,
    pub core_block_registered: bool,
    pub core_block_exclusive: bool,
    pub core_block_writable_bytes: u64,
    pub core_block_layout_80_u64: bool,
    pub adapter_registered: bool,
    pub adapter_receipt_bound: bool,
    pub adapter_stack_registered: bool,
    pub adapter_preserves_rbx: bool,
    pub adapter_control: u8,
    pub adapter_bridge_valid: bool,
    pub adapter_crash_latched: bool,
    pub fail_stop_registered: bool,
    pub schedule_stub_registered: bool,
    pub common_return_registered: bool,
}

pub struct WrapperStep {
    pub accepted: bool,
    pub header_valid: bool,
    pub adapter_invoked: bool,
    pub block_frame_cr2: u64,
    pub block_frame_vector: u64,
    pub block_frame_error: u64,
    pub block_frame_rip: u64,
    pub block_frame_cs: u64,
    pub block_frame_rflags: u64,
    pub block_frame_user_rsp: u64,
    pub block_frame_ss: u64,
    pub block_word_count: u64,
    pub block_arg_cr2: u64,
    pub block_arg_error: u64,
    pub block_arg_rip: u64,
    pub block_arg_rflags: u64,
    pub block_arg_user_rsp: u64,
    pub block_arg_metadata: u64,
    pub cr2_cross_checked: bool,
    pub adapter_bridge_valid: bool,
    pub control: u8,
    pub returned: bool,
    pub schedule_requested: bool,
    pub fail_stopped: bool,
    pub interrupts_disabled_at_stop: bool,
    pub post_rsp: u64,
    pub post_rip: u64,
}

pub open spec fn canonical_kernel_address(address: u64) -> bool {
    address >= 0xffff_8000_0000_0000
}

pub open spec fn gs_setup_image_registered(image: &GsSetupImage) -> bool {
    image.q00 == 0xf889_48c0_0001_01b9
        && image.q01 == 0x0f20_eac1_48fa_8948
        && image.q02 == 0x8948_c000_0102_b930
        && image.q03 == 0x20ea_c148_f289_48f0
        && image.tail == 0x00c3_300f
}

pub open spec fn wrapper_image_registered(image: &WrapperImage) -> bool {
    image.q00 == 0x0000_0025_048b_4865
        && image.q01 == 0x0122_840f_c085_4800
        && image.q02 == 0x0825_1c8b_4c65_0000
        && image.q03 == 0x000f_c3f7_4900_0000
        && image.q04 == 0x0000_010c_850f_0000
        && image.q05 == 0x0000_1025_3c3b_4865
        && image.q06 == 0x6500_0000_fd85_0f00
        && image.q07 == 0x0000_0018_253c_8148
        && image.q08 == 0x00ea_850f_0000_01ff
        && image.q09 == 0x8949_7047_8b48_0000
        && image.q10 == 0x0000_8087_8b48_7043
        && image.q11 == 0x0000_0080_8389_4900
        && image.q12 == 0x4900_0000_8887_8b48
        && image.q13 == 0x8b48_0000_0088_8389
        && image.q14 == 0x8389_4900_0000_9087
        && image.q15 == 0x9887_8b48_0000_0090
        && image.q16 == 0x0098_8389_4900_0000
        && image.q17 == 0x8b48_23f8_8348_0000
        && image.q18 == 0x8389_4900_0000_a087
        && image.q19 == 0x8b48_2975_0000_00a0
        && image.q20 == 0x8389_4900_0000_a887
        && image.q21 == 0xb087_8b48_0000_00a8
        && image.q22 == 0x00b0_8389_4900_0000
        && image.q23 == 0x0000_b883_c749_0000
        && image.q24 == 0x4921_eb00_0000_1700
        && image.q25 == 0x0000_0000_00a8_83c7
        && image.q26 == 0x0000_b083_c749_0000
        && image.q27 == 0x83c7_4900_0000_0000
        && image.q28 == 0x0000_0015_0000_00b8
        && image.q29 == 0x4900_0000_c093_894d
        && image.q30 == 0x8949_0000_00c8_b389
        && image.q31 == 0x8b89_4900_0000_d093
        && image.q32 == 0xe083_894d_0000_00d8
        && image.q33 == 0x00e8_8b89_4d00_0000
        && image.q34 == 0xec83_48df_894c_0000
        && image.q35 == 0x8348_0000_0be2_e808
        && image.q36 == 0xf883_1374_c085_08c4
        && image.q37 == 0xe900_0001_d184_0f01
        && image.q38 == 0x0000_c7e9_0000_00cc
        && image.tail == 0xc300
}

pub open spec fn control_images_registered(image: &ControlImages) -> bool {
    image.fail_stop == 0xfdeb_f4fa
        && image.schedule_unavailable == 0x0000_00ff_fffe_fbe9
}

pub fn wrapper_image_valid(image: &WrapperImage) -> (result: bool)
    ensures result == wrapper_image_registered(image),
{
    image.q00 == 0x0000_0025_048b_4865
        && image.q01 == 0x0122_840f_c085_4800
        && image.q02 == 0x0825_1c8b_4c65_0000
        && image.q03 == 0x000f_c3f7_4900_0000
        && image.q04 == 0x0000_010c_850f_0000
        && image.q05 == 0x0000_1025_3c3b_4865
        && image.q06 == 0x6500_0000_fd85_0f00
        && image.q07 == 0x0000_0018_253c_8148
        && image.q08 == 0x00ea_850f_0000_01ff
        && image.q09 == 0x8949_7047_8b48_0000
        && image.q10 == 0x0000_8087_8b48_7043
        && image.q11 == 0x0000_0080_8389_4900
        && image.q12 == 0x4900_0000_8887_8b48
        && image.q13 == 0x8b48_0000_0088_8389
        && image.q14 == 0x8389_4900_0000_9087
        && image.q15 == 0x9887_8b48_0000_0090
        && image.q16 == 0x0098_8389_4900_0000
        && image.q17 == 0x8b48_23f8_8348_0000
        && image.q18 == 0x8389_4900_0000_a087
        && image.q19 == 0x8b48_2975_0000_00a0
        && image.q20 == 0x8389_4900_0000_a887
        && image.q21 == 0xb087_8b48_0000_00a8
        && image.q22 == 0x00b0_8389_4900_0000
        && image.q23 == 0x0000_b883_c749_0000
        && image.q24 == 0x4921_eb00_0000_1700
        && image.q25 == 0x0000_0000_00a8_83c7
        && image.q26 == 0x0000_b083_c749_0000
        && image.q27 == 0x83c7_4900_0000_0000
        && image.q28 == 0x0000_0015_0000_00b8
        && image.q29 == 0x4900_0000_c093_894d
        && image.q30 == 0x8949_0000_00c8_b389
        && image.q31 == 0x8b89_4900_0000_d093
        && image.q32 == 0xe083_894d_0000_00d8
        && image.q33 == 0x00e8_8b89_4d00_0000
        && image.q34 == 0xec83_48df_894c_0000
        && image.q35 == 0x8348_0000_0be2_e808
        && image.q36 == 0xf883_1374_c085_08c4
        && image.q37 == 0xe900_0001_d184_0f01
        && image.q38 == 0x0000_c7e9_0000_00cc
        && image.tail == 0xc300
}

pub fn control_images_valid(image: &ControlImages) -> (result: bool)
    ensures result == control_images_registered(image),
{
    image.fail_stop == 0xfdeb_f4fa
        && image.schedule_unavailable == 0x0000_00ff_fffe_fbe9
}

pub open spec fn gs_header_valid(state: &WrapperState) -> bool {
    state.gs_self != 0
        && state.gs_core_block & 15 == 0
        && state.frame_pointer == state.gs_active_frame
        && state.gs_flags == GS_HEADER_FLAGS
}

pub open spec fn transport_matches_frame(state: &WrapperState) -> bool {
    state.transport_cr2 == state.frame_cr2
        && state.transport_error == state.frame_error
        && state.transport_rip == state.frame_rip
        && state.transport_rflags == state.frame_rflags
        && (state.transport_metadata & 0xffff_ffff) == state.frame_vector
        && ((state.transport_metadata >> 32) & 0xffff) == state.frame_cs
        && (state.frame_cs == USER_CODE_SELECTOR
            ==> state.transport_user_rsp == state.frame_user_rsp
                && ((state.transport_metadata >> 48) & 0xffff) == state.frame_ss)
        && (state.frame_cs != USER_CODE_SELECTOR
            ==> state.transport_user_rsp == 0
                && ((state.transport_metadata >> 48) & 0xffff) == 0)
}

pub open spec fn adapter_contract(state: &WrapperState) -> bool {
    state.adapter_control <= CONTROL_FAIL_STOP
        && (state.adapter_control == CONTROL_FAIL_STOP ==> state.adapter_crash_latched)
        && (!transport_matches_frame(state)
            ==> state.adapter_control == CONTROL_FAIL_STOP && !state.adapter_bridge_valid)
}

pub open spec fn wrapper_precondition(state: &WrapperState) -> bool {
    state.cpl == 0
        && !state.interrupts_enabled
        && !state.direction_flag
        && canonical_kernel_address(state.rsp)
        && state.rsp <= u64::MAX - 8
        && state.rsp & 15 == 8
        && state.return_address == COMMON_CONTINUATION
        && state.gs_header_registered
        && state.fail_stop_registered
        && state.schedule_stub_registered
        && state.adapter_registered
        && state.adapter_receipt_bound
        && state.adapter_preserves_rbx
        && state.common_return_registered
        && adapter_contract(state)
        && (gs_header_valid(state)
            ==> canonical_kernel_address(state.frame_pointer)
                && canonical_kernel_address(state.gs_core_block)
                && state.frame_prefix_registered
                && (state.frame_cs == USER_CODE_SELECTOR ==> state.frame_user_tail_registered)
                && state.core_block_registered
                && state.core_block_exclusive
                && state.core_block_writable_bytes >= SCALAR_CORE_BLOCK_BYTES
                && state.core_block_layout_80_u64
                && state.adapter_stack_registered)
}

pub fn transport_match(state: &WrapperState) -> (result: bool)
    ensures result == transport_matches_frame(state),
{
    let prefix = state.transport_cr2 == state.frame_cr2
        && state.transport_error == state.frame_error
        && state.transport_rip == state.frame_rip
        && state.transport_rflags == state.frame_rflags
        && (state.transport_metadata & 0xffff_ffff) == state.frame_vector
        && ((state.transport_metadata >> 32) & 0xffff) == state.frame_cs;
    if state.frame_cs == USER_CODE_SELECTOR {
        prefix
            && state.transport_user_rsp == state.frame_user_rsp
            && ((state.transport_metadata >> 48) & 0xffff) == state.frame_ss
    } else {
        prefix
            && state.transport_user_rsp == 0
            && ((state.transport_metadata >> 48) & 0xffff) == 0
    }
}

pub fn wrapper_state_valid(state: &WrapperState) -> (result: bool)
    ensures result == wrapper_precondition(state),
{
    let header = state.gs_self != 0
        && state.gs_core_block & 15 == 0
        && state.frame_pointer == state.gs_active_frame
        && state.gs_flags == GS_HEADER_FLAGS;
    let transport = transport_match(state);
    state.cpl == 0
        && !state.interrupts_enabled
        && !state.direction_flag
        && state.rsp >= 0xffff_8000_0000_0000
        && state.rsp <= u64::MAX - 8
        && state.rsp & 15 == 8
        && state.return_address == COMMON_CONTINUATION
        && state.gs_header_registered
        && state.fail_stop_registered
        && state.schedule_stub_registered
        && state.adapter_registered
        && state.adapter_receipt_bound
        && state.adapter_preserves_rbx
        && state.common_return_registered
        && state.adapter_control <= CONTROL_FAIL_STOP
        && (state.adapter_control != CONTROL_FAIL_STOP || state.adapter_crash_latched)
        && (transport
            || state.adapter_control == CONTROL_FAIL_STOP && !state.adapter_bridge_valid)
        && (!header
            || state.frame_pointer >= 0xffff_8000_0000_0000
                && state.gs_core_block >= 0xffff_8000_0000_0000
                && state.frame_prefix_registered
                && (state.frame_cs != USER_CODE_SELECTOR || state.frame_user_tail_registered)
                && state.core_block_registered
                && state.core_block_exclusive
                && state.core_block_writable_bytes >= SCALAR_CORE_BLOCK_BYTES
                && state.core_block_layout_80_u64
                && state.adapter_stack_registered)
}

pub fn registered_gs_setup_image() -> (result: GsSetupImage)
    ensures gs_setup_image_registered(&result),
{
    GsSetupImage {
        q00: 0xf889_48c0_0001_01b9,
        q01: 0x0f20_eac1_48fa_8948,
        q02: 0x8948_c000_0102_b930,
        q03: 0x20ea_c148_f289_48f0,
        tail: 0x00c3_300f,
    }
}

pub fn registered_wrapper_image() -> (result: WrapperImage)
    ensures wrapper_image_registered(&result),
{
    WrapperImage {
        q00: 0x0000_0025_048b_4865,
        q01: 0x0122_840f_c085_4800,
        q02: 0x0825_1c8b_4c65_0000,
        q03: 0x000f_c3f7_4900_0000,
        q04: 0x0000_010c_850f_0000,
        q05: 0x0000_1025_3c3b_4865,
        q06: 0x6500_0000_fd85_0f00,
        q07: 0x0000_0018_253c_8148,
        q08: 0x00ea_850f_0000_01ff,
        q09: 0x8949_7047_8b48_0000,
        q10: 0x0000_8087_8b48_7043,
        q11: 0x0000_0080_8389_4900,
        q12: 0x4900_0000_8887_8b48,
        q13: 0x8b48_0000_0088_8389,
        q14: 0x8389_4900_0000_9087,
        q15: 0x9887_8b48_0000_0090,
        q16: 0x0098_8389_4900_0000,
        q17: 0x8b48_23f8_8348_0000,
        q18: 0x8389_4900_0000_a087,
        q19: 0x8b48_2975_0000_00a0,
        q20: 0x8389_4900_0000_a887,
        q21: 0xb087_8b48_0000_00a8,
        q22: 0x00b0_8389_4900_0000,
        q23: 0x0000_b883_c749_0000,
        q24: 0x4921_eb00_0000_1700,
        q25: 0x0000_0000_00a8_83c7,
        q26: 0x0000_b083_c749_0000,
        q27: 0x83c7_4900_0000_0000,
        q28: 0x0000_0015_0000_00b8,
        q29: 0x4900_0000_c093_894d,
        q30: 0x8949_0000_00c8_b389,
        q31: 0x8b89_4900_0000_d093,
        q32: 0xe083_894d_0000_00d8,
        q33: 0x00e8_8b89_4d00_0000,
        q34: 0xec83_48df_894c_0000,
        q35: 0x8348_0000_0be2_e808,
        q36: 0xf883_1374_c085_08c4,
        q37: 0xe900_0001_d184_0f01,
        q38: 0x0000_c7e9_0000_00cc,
        tail: 0xc300,
    }
}

pub fn registered_control_images() -> (result: ControlImages)
    ensures control_images_registered(&result),
{
    ControlImages {
        fail_stop: 0xfdeb_f4fa,
        schedule_unavailable: 0x0000_00ff_fffe_fbe9,
    }
}

pub fn install_gs(image: GsSetupImage, state: GsSetupState) -> (result: GsSetupStep)
    ensures
        result.accepted <==> gs_setup_image_registered(&image)
            && state.cpl == 0
            && !state.interrupts_enabled
            && canonical_kernel_address(state.gs_base_operand)
            && canonical_kernel_address(state.kernel_gs_base_operand)
            && state.current_gs_header_registered
            && state.kernel_gs_header_registered
            && state.msr_access_registered
            && state.return_stack_registered,
        result.accepted ==> result.gs_base == state.gs_base_operand,
        result.accepted ==> result.kernel_gs_base == state.kernel_gs_base_operand,
        result.accepted ==> result.wrmsr_count == 2,
        result.accepted ==> result.returned,
        result.accepted ==> result.post_rip == state.return_address,
        !result.accepted ==> result.gs_base == 0,
        !result.accepted ==> result.kernel_gs_base == 0,
        !result.accepted ==> result.wrmsr_count == 0,
        !result.accepted ==> !result.returned,
        !result.accepted ==> result.post_rip == 0,
{
    if image.q00 == 0xf889_48c0_0001_01b9
        && image.q01 == 0x0f20_eac1_48fa_8948
        && image.q02 == 0x8948_c000_0102_b930
        && image.q03 == 0x20ea_c148_f289_48f0
        && image.tail == 0x00c3_300f
        && state.cpl == 0
        && !state.interrupts_enabled
        && state.gs_base_operand >= 0xffff_8000_0000_0000
        && state.kernel_gs_base_operand >= 0xffff_8000_0000_0000
        && state.current_gs_header_registered
        && state.kernel_gs_header_registered
        && state.msr_access_registered
        && state.return_stack_registered
    {
        GsSetupStep {
            accepted: true,
            gs_base: state.gs_base_operand,
            kernel_gs_base: state.kernel_gs_base_operand,
            wrmsr_count: 2,
            returned: true,
            post_rip: state.return_address,
        }
    } else {
        GsSetupStep {
            accepted: false,
            gs_base: 0,
            kernel_gs_base: 0,
            wrmsr_count: 0,
            returned: false,
            post_rip: 0,
        }
    }
}

pub fn decode_execute_wrapper(
    image: WrapperImage,
    controls: ControlImages,
    state: WrapperState,
) -> (result: WrapperStep)
    ensures
        result.accepted <==> wrapper_image_registered(&image)
            && control_images_registered(&controls)
            && wrapper_precondition(&state),
        result.accepted ==> result.header_valid == gs_header_valid(&state),
        result.accepted ==> result.adapter_invoked == result.header_valid,
        result.accepted && result.header_valid ==>
            result.block_frame_cr2 == state.frame_cr2
                && result.block_arg_cr2 == state.transport_cr2,
        result.accepted && result.header_valid ==>
            result.block_frame_error == state.frame_error
                && result.block_arg_error == state.transport_error,
        result.accepted && result.header_valid ==>
            result.block_frame_rip == state.frame_rip
                && result.block_arg_rip == state.transport_rip,
        result.accepted && result.header_valid ==>
            result.block_frame_rflags == state.frame_rflags
                && result.block_arg_rflags == state.transport_rflags,
        result.accepted && result.header_valid ==>
            result.block_frame_vector == state.frame_vector
                && result.block_frame_cs == state.frame_cs,
        result.accepted && result.header_valid && state.frame_cs == USER_CODE_SELECTOR ==>
            result.block_frame_user_rsp == state.frame_user_rsp
                && result.block_frame_ss == state.frame_ss
                && result.block_word_count == 23,
        result.accepted && result.header_valid && state.frame_cs != USER_CODE_SELECTOR ==>
            result.block_frame_user_rsp == 0
                && result.block_frame_ss == 0
                && result.block_word_count == 21,
        result.accepted && result.header_valid ==>
            result.block_arg_user_rsp == state.transport_user_rsp
                && result.block_arg_metadata == state.transport_metadata,
        result.accepted && result.header_valid ==>
            result.cr2_cross_checked == (state.frame_cr2 == state.transport_cr2),
        result.accepted && result.header_valid ==>
            result.adapter_bridge_valid == state.adapter_bridge_valid,
        result.accepted && !result.header_valid ==> !result.adapter_bridge_valid,
        result.accepted && result.header_valid ==> result.control == state.adapter_control,
        result.accepted && !result.header_valid ==> result.control == CONTROL_FAIL_STOP,
        result.accepted ==> result.returned
            == (result.header_valid && result.control == CONTROL_RETURN),
        result.accepted ==> result.schedule_requested
            == (result.header_valid && result.control == CONTROL_SCHEDULE),
        result.accepted ==> result.fail_stopped == !result.returned,
        result.accepted ==> result.interrupts_disabled_at_stop == result.fail_stopped,
        result.accepted && result.returned ==> result.post_rsp == state.rsp + 8,
        result.accepted && result.returned ==> result.post_rip == COMMON_CONTINUATION,
        result.accepted && !result.returned ==> result.post_rsp == 0 && result.post_rip == 0,
        !result.accepted ==> !result.header_valid,
        !result.accepted ==> !result.adapter_invoked,
        !result.accepted ==> !result.returned,
        !result.accepted ==> !result.schedule_requested,
        !result.accepted ==> !result.fail_stopped,
        !result.accepted ==> result.post_rsp == 0 && result.post_rip == 0,
{
    if wrapper_image_valid(&image)
        && control_images_valid(&controls)
        && wrapper_state_valid(&state)
    {
        let header = state.gs_self != 0
            && state.gs_core_block & 15 == 0
            && state.frame_pointer == state.gs_active_frame
            && state.gs_flags == GS_HEADER_FLAGS;
        if header {
            let user = state.frame_cs == USER_CODE_SELECTOR;
            let control = state.adapter_control;
            let returned = control == CONTROL_RETURN;
            WrapperStep {
                accepted: true,
                header_valid: true,
                adapter_invoked: true,
                block_frame_cr2: state.frame_cr2,
                block_frame_vector: state.frame_vector,
                block_frame_error: state.frame_error,
                block_frame_rip: state.frame_rip,
                block_frame_cs: state.frame_cs,
                block_frame_rflags: state.frame_rflags,
                block_frame_user_rsp: if user { state.frame_user_rsp } else { 0 },
                block_frame_ss: if user { state.frame_ss } else { 0 },
                block_word_count: if user { 23 } else { 21 },
                block_arg_cr2: state.transport_cr2,
                block_arg_error: state.transport_error,
                block_arg_rip: state.transport_rip,
                block_arg_rflags: state.transport_rflags,
                block_arg_user_rsp: state.transport_user_rsp,
                block_arg_metadata: state.transport_metadata,
                cr2_cross_checked: state.frame_cr2 == state.transport_cr2,
                adapter_bridge_valid: state.adapter_bridge_valid,
                control,
                returned,
                schedule_requested: control == CONTROL_SCHEDULE,
                fail_stopped: !returned,
                interrupts_disabled_at_stop: !returned,
                post_rsp: if returned { state.rsp + 8 } else { 0 },
                post_rip: if returned { COMMON_CONTINUATION } else { 0 },
            }
        } else {
            WrapperStep {
                accepted: true,
                header_valid: false,
                adapter_invoked: false,
                block_frame_cr2: 0,
                block_frame_vector: 0,
                block_frame_error: 0,
                block_frame_rip: 0,
                block_frame_cs: 0,
                block_frame_rflags: 0,
                block_frame_user_rsp: 0,
                block_frame_ss: 0,
                block_word_count: 0,
                block_arg_cr2: 0,
                block_arg_error: 0,
                block_arg_rip: 0,
                block_arg_rflags: 0,
                block_arg_user_rsp: 0,
                block_arg_metadata: 0,
                cr2_cross_checked: false,
                adapter_bridge_valid: false,
                control: CONTROL_FAIL_STOP,
                returned: false,
                schedule_requested: false,
                fail_stopped: true,
                interrupts_disabled_at_stop: true,
                post_rsp: 0,
                post_rip: 0,
            }
        }
    } else {
        WrapperStep {
            accepted: false,
            header_valid: false,
            adapter_invoked: false,
            block_frame_cr2: 0,
            block_frame_vector: 0,
            block_frame_error: 0,
            block_frame_rip: 0,
            block_frame_cs: 0,
            block_frame_rflags: 0,
            block_frame_user_rsp: 0,
            block_frame_ss: 0,
            block_word_count: 0,
            block_arg_cr2: 0,
            block_arg_error: 0,
            block_arg_rip: 0,
            block_arg_rflags: 0,
            block_arg_user_rsp: 0,
            block_arg_metadata: 0,
            cr2_cross_checked: false,
            adapter_bridge_valid: false,
            control: 0,
            returned: false,
            schedule_requested: false,
            fail_stopped: false,
            interrupts_disabled_at_stop: false,
            post_rsp: 0,
            post_rip: 0,
        }
    }
}

pub fn wrapper_observation() -> (result: u64)
    ensures result == 4095,
{
    let state = WrapperState {
        cpl: 0,
        interrupts_enabled: false,
        direction_flag: false,
        rsp: 0xffff_e000_0000_2e78,
        return_address: COMMON_CONTINUATION,
        gs_header_registered: true,
        gs_self: 0xffff_e000_0000_0000,
        gs_core_block: 0xffff_e000_0000_1000,
        gs_active_frame: 0xffff_e000_0000_2e80,
        gs_flags: GS_HEADER_FLAGS,
        frame_pointer: 0xffff_e000_0000_2e80,
        frame_prefix_registered: true,
        frame_user_tail_registered: true,
        frame_cr2: 0x1234_5000,
        frame_vector: 14,
        frame_error: 6,
        frame_rip: 0x0040_1000,
        frame_cs: USER_CODE_SELECTOR,
        frame_rflags: 0x202,
        frame_user_rsp: 0x0000_7fff_ffff_e000,
        frame_ss: USER_DATA_SELECTOR,
        transport_cr2: 0x1234_5000,
        transport_error: 6,
        transport_rip: 0x0040_1000,
        transport_rflags: 0x202,
        transport_user_rsp: 0x0000_7fff_ffff_e000,
        transport_metadata: 0x001b_0023_0000_000e,
        core_block_registered: true,
        core_block_exclusive: true,
        core_block_writable_bytes: SCALAR_CORE_BLOCK_BYTES,
        core_block_layout_80_u64: true,
        adapter_registered: true,
        adapter_receipt_bound: true,
        adapter_stack_registered: true,
        adapter_preserves_rbx: true,
        adapter_control: CONTROL_RETURN,
        adapter_bridge_valid: true,
        adapter_crash_latched: false,
        fail_stop_registered: true,
        schedule_stub_registered: true,
        common_return_registered: true,
    };
    assert(0xffff_e000_0000_2e78u64 & 15 == 8) by(bit_vector);
    assert(0x001b_0023_0000_000eu64 & 0xffff_ffff == 14) by(bit_vector);
    assert((0x001b_0023_0000_000eu64 >> 32) & 0xffff == USER_CODE_SELECTOR) by(bit_vector);
    assert((0x001b_0023_0000_000eu64 >> 48) & 0xffff == USER_DATA_SELECTOR) by(bit_vector);
    let matched = transport_match(&state);
    assert(matched);
    let state_valid = wrapper_state_valid(&state);
    assert(state_valid);
    assert(0xffff_e000_0000_1000u64 & 15 == 0) by(bit_vector);
    assert(gs_header_valid(&state));
    let image = registered_wrapper_image();
    assert(wrapper_image_registered(&image));
    let controls = registered_control_images();
    assert(control_images_registered(&controls));
    let step = decode_execute_wrapper(image, controls, state);
    assert(step.accepted && step.header_valid && step.adapter_invoked);
    assert(step.block_frame_cr2 == step.block_arg_cr2);
    assert(step.block_word_count == 23);
    assert(step.cr2_cross_checked && step.adapter_bridge_valid);
    assert(step.returned && !step.schedule_requested && !step.fail_stopped);
    assert(step.post_rsp == 0xffff_e000_0000_2e80);
    assert(step.post_rip == COMMON_CONTINUATION);
    4095
}

}
