extern crate tmk_bootinfo;

use tmk_bootinfo::bootinfo_shell::validate_bootinfo;

const BOOTINFO_LEN: usize = 320;

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap())
}

fn seal_header(bytes: &mut [u8]) {
    put_u64(bytes, 24, 0);
    let checksum = (0..32).fold(0u64, |sum, slot| sum ^ read_u64(bytes, slot * 8));
    put_u64(bytes, 24, checksum);
    assert_eq!(
        (0..32).fold(0u64, |sum, slot| sum ^ read_u64(bytes, slot * 8)),
        0
    );
}

fn valid_bootinfo() -> Vec<u8> {
    let mut bytes = vec![0u8; BOOTINFO_LEN];
    put_u64(&mut bytes, 0, 0x3154_4f4f_424b_4d54);
    put_u32(&mut bytes, 8, 1);
    put_u32(&mut bytes, 12, 0);
    put_u32(&mut bytes, 16, 256);
    put_u32(&mut bytes, 20, BOOTINFO_LEN as u32);
    put_u64(&mut bytes, 32, 1);
    put_u32(&mut bytes, 40, 256);
    put_u32(&mut bytes, 44, 32);
    put_u32(&mut bytes, 48, 2);
    put_u64(&mut bytes, 56, 0x0020_0000);
    put_u64(&mut bytes, 64, 0x0040_0000);
    put_u64(&mut bytes, 72, 0xffff_ffff_8000_0000);
    put_u64(&mut bytes, 80, 0xffff_ffff_8020_0000);
    put_u64(&mut bytes, 88, 0x0080_0000);
    put_u64(&mut bytes, 96, 0x1000);
    bytes[104] = 0xa5;
    put_u64(&mut bytes, 136, 0x0090_0000);
    put_u64(&mut bytes, 144, 0x1000);
    bytes[152] = 0x5a;
    put_u64(&mut bytes, 184, 0x000f_0000);
    put_u32(&mut bytes, 192, BOOTINFO_LEN as u32);
    put_u32(&mut bytes, 196, 0);
    put_u32(&mut bytes, 200, BOOTINFO_LEN as u32);
    put_u32(&mut bytes, 204, 0);
    put_u32(&mut bytes, 208, 7);

    put_u64(&mut bytes, 256, 0x0000_1000);
    put_u64(&mut bytes, 264, 0x0080_0000);
    put_u32(&mut bytes, 272, 1);
    put_u64(&mut bytes, 288, 0x0080_0000);
    put_u64(&mut bytes, 296, 0x00a0_0000);
    put_u32(&mut bytes, 304, 2);
    seal_header(&mut bytes);
    bytes
}

fn expect_code(bytes: &[u8], expected: u32) {
    let result = validate_bootinfo(bytes);
    assert_eq!(result.code, expected);
}

fn main() {
    let valid = valid_bootinfo();
    let result = validate_bootinfo(&valid);
    assert_eq!(result.code, 0);
    assert_eq!(result.range_count, 2);
    assert_eq!(result.last_end, 0x00a0_0000);
    assert_eq!(result.bsp_apic_id, 7);

    expect_code(&valid[..255], 100);

    let mut bad_magic = valid.clone();
    put_u64(&mut bad_magic, 0, 0);
    seal_header(&mut bad_magic);
    expect_code(&bad_magic, 2);

    let mut bad_checksum = valid.clone();
    bad_checksum[24] ^= 1;
    expect_code(&bad_checksum, 4);

    let mut bad_header_reserved = valid.clone();
    put_u32(&mut bad_header_reserved, 52, 1);
    seal_header(&mut bad_header_reserved);
    expect_code(&bad_header_reserved, 4);

    let mut missing_service_digest = valid.clone();
    missing_service_digest[104] = 0;
    seal_header(&mut missing_service_digest);
    expect_code(&missing_service_digest, 4);

    let mut invalid_framebuffer = valid.clone();
    put_u64(&mut invalid_framebuffer, 32, 5);
    seal_header(&mut invalid_framebuffer);
    expect_code(&invalid_framebuffer, 4);

    let mut bad_kind = valid.clone();
    put_u32(&mut bad_kind, 304, 0);
    expect_code(&bad_kind, 13);

    let mut overlap = valid.clone();
    put_u64(&mut overlap, 288, 0x0070_0000);
    expect_code(&overlap, 13);

    let mut unaligned = valid.clone();
    put_u64(&mut unaligned, 296, 0x00a0_0001);
    expect_code(&unaligned, 13);

    let mut reserved_low = valid.clone();
    put_u32(&mut reserved_low, 308, 1);
    expect_code(&reserved_low, 13);

    let mut reserved_high = valid.clone();
    put_u64(&mut reserved_high, 312, 1);
    expect_code(&reserved_high, 13);

    let mut truncated_map = valid.clone();
    truncated_map.pop();
    expect_code(&truncated_map, 4);

    println!(
        "M1_BOOTINFO_OK ranges={} last={:016x} bsp={} negatives=12",
        result.range_count, result.last_end, result.bsp_apic_id
    );
}
