#![no_std]
#![allow(dead_code)]

mod abi {
    include!(env!("TMK_IDL_RS"));
}

#[no_mangle]
pub extern "C" fn tmk_idl_nostd_probe(tag: u64, cap: u64) -> u64 {
    abi::tmk_message_tag_v1_protocol_id(tag)
        ^ abi::tmk_cap_ptr_v1_root_guard(cap)
        ^ abi::TMK_SYSCALL_ABI_QUERY
        ^ core::mem::size_of::<abi::TmkUtcbV1>() as u64
}
