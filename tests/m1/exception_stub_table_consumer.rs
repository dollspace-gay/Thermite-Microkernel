extern crate tmk_exception_stub_table;

use tmk_exception_stub_table::{
    COMMON_ENTRY_VIRTUAL, STUB_TABLE_VIRTUAL, StubImage, StubTable, VECTOR_COUNT,
    exception_stub_observation, registered_stub_table, stub_encoding_valid,
    stub_has_synthetic_error, stub_jump_target, stub_padding_valid, stub_vector,
    vector_has_error_code,
};

fn main() {
    assert_eq!(std::mem::size_of::<StubImage>(), 16);
    assert_eq!(std::mem::size_of::<StubTable>(), 4096);
    assert_eq!(std::mem::align_of::<StubTable>(), 4096);

    let table = Box::new(registered_stub_table());
    let mut bytes = Vec::with_capacity(4096);
    let mut cpu_error = 0usize;
    let mut synthetic = 0usize;
    for (vector, stub) in table.entries.iter().enumerate() {
        assert_eq!(stub_vector(stub), vector as u64);
        assert!(stub_encoding_valid(stub));
        assert!(stub_padding_valid(stub));
        assert_eq!(stub_jump_target(stub, vector as u16), COMMON_ENTRY_VIRTUAL);
        let has_cpu_error = vector_has_error_code(vector as u16);
        assert_eq!(stub_has_synthetic_error(stub), !has_cpu_error);
        if has_cpu_error {
            cpu_error += 1;
        } else {
            synthetic += 1;
        }

        let mut encoded = [0u8; 16];
        encoded[..8].copy_from_slice(&stub.qword0.to_le_bytes());
        encoded[8..].copy_from_slice(&stub.qword1.to_le_bytes());
        let pushed_vector = if has_cpu_error {
            assert_eq!(encoded[0], 0x68);
            assert_eq!(encoded[5], 0xe9);
            assert!(encoded[10..].iter().all(|byte| *byte == 0x90));
            u32::from_le_bytes(encoded[1..5].try_into().unwrap())
        } else {
            assert_eq!(&encoded[..3], &[0x6a, 0x00, 0x68]);
            assert_eq!(encoded[7], 0xe9);
            assert!(encoded[12..].iter().all(|byte| *byte == 0x90));
            u32::from_le_bytes(encoded[3..7].try_into().unwrap())
        };
        assert_eq!(pushed_vector, vector as u32);
        bytes.extend_from_slice(&encoded);
    }
    assert_eq!(cpu_error, 10);
    assert_eq!(synthetic, 246);
    assert_eq!(bytes.len(), 4096);
    assert_eq!(STUB_TABLE_VIRTUAL + bytes.len() as u64, COMMON_ENTRY_VIRTUAL);

    let mut args = std::env::args_os().skip(1);
    if let Some(path) = args.next() {
        assert!(args.next().is_none(), "expected at most one stub-table output path");
        std::fs::write(path, &bytes).expect("write verified exception-stub bytes");
    }

    let observation = exception_stub_observation();
    assert_eq!(observation, 255);
    assert_eq!(VECTOR_COUNT, 256);
    println!(
        "M1_EXCEPTION_STUBS_OK observation={observation} vectors=256 error={cpu_error} synthetic={synthetic} bytes=4096 target={COMMON_ENTRY_VIRTUAL:016x}"
    );
}
