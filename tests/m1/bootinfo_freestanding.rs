#![no_std]
#![no_main]

extern crate tmk_bootinfo;

use core::panic::PanicInfo;

const BOOTINFO_LEN: usize = 320;

const fn put_u32(mut bytes: [u8; BOOTINFO_LEN], offset: usize, value: u32) -> [u8; BOOTINFO_LEN] {
    let encoded = value.to_le_bytes();
    let mut index = 0;
    while index < 4 {
        bytes[offset + index] = encoded[index];
        index += 1;
    }
    bytes
}

const fn put_u64(mut bytes: [u8; BOOTINFO_LEN], offset: usize, value: u64) -> [u8; BOOTINFO_LEN] {
    let encoded = value.to_le_bytes();
    let mut index = 0;
    while index < 8 {
        bytes[offset + index] = encoded[index];
        index += 1;
    }
    bytes
}

const fn read_u64(bytes: &[u8; BOOTINFO_LEN], offset: usize) -> u64 {
    u64::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
        bytes[offset + 4],
        bytes[offset + 5],
        bytes[offset + 6],
        bytes[offset + 7],
    ])
}

const fn valid_bootinfo() -> [u8; BOOTINFO_LEN] {
    let mut bytes = [0u8; BOOTINFO_LEN];
    bytes = put_u64(bytes, 0, 0x3154_4f4f_424b_4d54);
    bytes = put_u32(bytes, 8, 1);
    bytes = put_u32(bytes, 16, 256);
    bytes = put_u32(bytes, 20, BOOTINFO_LEN as u32);
    bytes = put_u64(bytes, 32, 1);
    bytes = put_u32(bytes, 40, 256);
    bytes = put_u32(bytes, 44, 32);
    bytes = put_u32(bytes, 48, 2);
    bytes = put_u64(bytes, 56, 0x0020_0000);
    bytes = put_u64(bytes, 64, 0x0040_0000);
    bytes = put_u64(bytes, 72, 0xffff_ffff_8000_0000);
    bytes = put_u64(bytes, 80, 0xffff_ffff_8020_0000);
    bytes = put_u64(bytes, 88, 0x0080_0000);
    bytes = put_u64(bytes, 96, 0x1000);
    bytes[104] = 0xa5;
    bytes = put_u64(bytes, 136, 0x0090_0000);
    bytes = put_u64(bytes, 144, 0x1000);
    bytes[152] = 0x5a;
    bytes = put_u64(bytes, 184, 0x000f_0000);
    bytes = put_u32(bytes, 192, BOOTINFO_LEN as u32);
    bytes = put_u32(bytes, 200, BOOTINFO_LEN as u32);
    bytes = put_u32(bytes, 208, 7);
    bytes = put_u64(bytes, 256, 0x0000_1000);
    bytes = put_u64(bytes, 264, 0x0080_0000);
    bytes = put_u32(bytes, 272, 1);
    bytes = put_u64(bytes, 288, 0x0080_0000);
    bytes = put_u64(bytes, 296, 0x00a0_0000);
    bytes = put_u32(bytes, 304, 2);
    let mut checksum = 0u64;
    let mut slot = 0;
    while slot < 32 {
        checksum ^= read_u64(&bytes, slot * 8);
        slot += 1;
    }
    put_u64(bytes, 24, checksum)
}

static BOOTINFO: [u8; BOOTINFO_LEN] = valid_bootinfo();

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[no_mangle]
pub extern "C" fn rust_eh_personality() {}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let result = tmk_bootinfo::bootinfo_shell::validate_bootinfo(&BOOTINFO);
    if result.code != 0 || result.range_count != 2 || result.last_end != 0x00a0_0000 {
        panic!("verified BootInfo rejected");
    }
    loop {
        core::hint::spin_loop();
    }
}
