extern crate tmk_firmware_raw_map;

use tmk_firmware_raw_map::firmware_raw_map_shell::validate_raw_memory_map;

const DESCRIPTOR_SIZE: usize = 48;

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn descriptor(
    bytes: &mut [u8],
    index: usize,
    memory_type: u32,
    physical_start: u64,
    virtual_start: u64,
    pages: u64,
    attributes: u64,
) {
    let offset = index * DESCRIPTOR_SIZE;
    put_u32(bytes, offset, memory_type);
    put_u64(bytes, offset + 8, physical_start);
    put_u64(bytes, offset + 16, virtual_start);
    put_u64(bytes, offset + 24, pages);
    put_u64(bytes, offset + 32, attributes);
    bytes[offset + 40..offset + DESCRIPTOR_SIZE].fill(0xa5);
}

fn valid_map() -> Vec<u8> {
    let mut bytes = vec![0u8; DESCRIPTOR_SIZE * 6];
    descriptor(&mut bytes, 0, 0, 0, 0, 1, 0);
    descriptor(&mut bytes, 1, 2, 0x1000, 0, 1, 8);
    descriptor(&mut bytes, 2, 7, 0x2000, 0, 16, 8);
    descriptor(&mut bytes, 3, 11, 0x12000, 0, 1, 0x8000_0000_0000_0001);
    descriptor(&mut bytes, 4, 12, 0x13000, 0, 1, 1);
    descriptor(&mut bytes, 5, 15, 0x14000, 0, 2, 0);
    bytes
}

fn code(bytes: &[u8], size: u64, descriptor_size: u64, version: u32, key: u64) -> u32 {
    validate_raw_memory_map(bytes, size, descriptor_size, version, key).code
}

fn main() {
    let valid = valid_map();
    let accepted = validate_raw_memory_map(&valid, valid.len() as u64, 48, 1, 77);
    assert_eq!(accepted.code, 0);
    assert_eq!(accepted.map_key, 77);
    assert_eq!(accepted.descriptor_size, 48);
    assert_eq!(accepted.descriptor_version, 1);
    assert_eq!(accepted.descriptor_count, 6);
    assert_eq!(accepted.last_end, 0x16000);
    assert_eq!(accepted.usable_pages, 16);

    assert_eq!(code(&[], 0, 48, 1, 77), 100);
    assert_eq!(code(&valid, valid.len() as u64 - 1, 48, 1, 77), 101);
    assert_eq!(code(&valid, valid.len() as u64, 39, 1, 77), 102);
    assert_eq!(code(&valid, valid.len() as u64, 44, 1, 77), 102);
    assert_eq!(code(&valid, valid.len() as u64, 48, 2, 77), 102);
    assert_eq!(code(&valid, valid.len() as u64, 48, 1, 0), 102);

    let mut bad_type = valid.clone();
    put_u32(&mut bad_type, 0, 16);
    assert_eq!(code(&bad_type, bad_type.len() as u64, 48, 1, 77), 13);

    let mut unaligned = valid.clone();
    put_u64(&mut unaligned, DESCRIPTOR_SIZE + 8, 0x1001);
    assert_eq!(code(&unaligned, unaligned.len() as u64, 48, 1, 77), 12);

    let mut zero_pages = valid.clone();
    put_u64(&mut zero_pages, DESCRIPTOR_SIZE + 24, 0);
    assert_eq!(code(&zero_pages, zero_pages.len() as u64, 48, 1, 77), 106);

    let mut overlap = valid.clone();
    put_u64(&mut overlap, DESCRIPTOR_SIZE * 3 + 8, 0x11000);
    assert_eq!(code(&overlap, overlap.len() as u64, 48, 1, 77), 12);

    let mut virtual_unaligned = valid.clone();
    put_u64(&mut virtual_unaligned, 16, 1);
    assert_eq!(code(&virtual_unaligned, virtual_unaligned.len() as u64, 48, 1, 77), 106);

    let mut unknown_attribute = valid.clone();
    put_u64(&mut unknown_attribute, 32, 0x0000_0000_0020_0000);
    assert_eq!(code(&unknown_attribute, unknown_attribute.len() as u64, 48, 1, 77), 106);

    let mut isa_without_valid = valid.clone();
    put_u64(&mut isa_without_valid, 32, 0x0000_1000_0000_0000);
    assert_eq!(code(&isa_without_valid, isa_without_valid.len() as u64, 48, 1, 77), 106);

    let mut runtime_mismatch = valid.clone();
    put_u64(&mut runtime_mismatch, DESCRIPTOR_SIZE + 32, 0x8000_0000_0000_0008);
    assert_eq!(code(&runtime_mismatch, runtime_mismatch.len() as u64, 48, 1, 77), 13);

    let mut missing_cache = valid.clone();
    put_u64(&mut missing_cache, DESCRIPTOR_SIZE * 2 + 32, 0);
    assert_eq!(code(&missing_cache, missing_cache.len() as u64, 48, 1, 77), 13);

    let mut no_usable = valid.clone();
    put_u32(&mut no_usable, DESCRIPTOR_SIZE * 2, 0);
    put_u64(&mut no_usable, DESCRIPTOR_SIZE * 2 + 32, 0);
    assert_eq!(code(&no_usable, no_usable.len() as u64, 48, 1, 77), 20);

    let truncated = &valid[..valid.len() - 1];
    assert_eq!(code(truncated, truncated.len() as u64, 48, 1, 77), 102);

    println!(
        "M1_FIRMWARE_RAW_MAP_OK descriptors=6 size=48 key=77 usable=16 runtime-mmio=both unaccepted=reserved negatives=17"
    );
}
