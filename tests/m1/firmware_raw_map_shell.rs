use vstd::prelude::*;

pub const RAW_MAP_LIMIT: u64 = 1_048_576;
pub const DESCRIPTOR_LIMIT: u64 = 4_096;
pub const PAGE_LIMIT: u64 = 1_099_511_627_776;
pub const PHYSICAL_LIMIT: u64 = 0x0010_0000_0000_0000;
pub const EFI_MEMORY_ATTRIBUTE_MASK: u64 = 0xcfff_f000_001f_f01f;
pub const EFI_MEMORY_ISA_VALID: u64 = 0x4000_0000_0000_0000;
pub const EFI_MEMORY_ISA_MASK: u64 = 0x0fff_f000_0000_0000;
pub const EFI_MEMORY_RUNTIME: u64 = 0x8000_0000_0000_0000;

pub struct RawMapValidation {
    pub code: u32,
    pub map_key: u64,
    pub descriptor_size: u64,
    pub descriptor_version: u32,
    pub descriptor_count: u64,
    pub last_end: u64,
    pub usable_pages: u64,
}

pub open spec fn spec_read_u32(bytes: &[u8], offset: int) -> u32 {
    (bytes@[offset] as u32)
        | ((bytes@[offset + 1] as u32) << 8)
        | ((bytes@[offset + 2] as u32) << 16)
        | ((bytes@[offset + 3] as u32) << 24)
}

pub open spec fn spec_read_u64(bytes: &[u8], offset: int) -> u64 {
    (bytes@[offset] as u64)
        | ((bytes@[offset + 1] as u64) << 8)
        | ((bytes@[offset + 2] as u64) << 16)
        | ((bytes@[offset + 3] as u64) << 24)
        | ((bytes@[offset + 4] as u64) << 32)
        | ((bytes@[offset + 5] as u64) << 40)
        | ((bytes@[offset + 6] as u64) << 48)
        | ((bytes@[offset + 7] as u64) << 56)
}

pub open spec fn spec_cache_class(attributes: u64) -> u32 {
    if attributes & 8 != 0 {
        4
    } else if attributes & 4 != 0 {
        3
    } else if attributes & 2 != 0 {
        2
    } else if attributes & 1 != 0 {
        1
    } else if attributes & 16 != 0 {
        5
    } else {
        0
    }
}

pub open spec fn spec_descriptor_valid(
    bytes: &[u8],
    descriptor_size: u64,
    index: int,
) -> bool {
    let offset = index * descriptor_size as int;
    let memory_type = spec_read_u32(bytes, offset);
    let physical_start = spec_read_u64(bytes, offset + 8);
    let virtual_start = spec_read_u64(bytes, offset + 16);
    let page_count = spec_read_u64(bytes, offset + 24);
    let attributes = spec_read_u64(bytes, offset + 32);
    memory_type <= 15
        && page_count != 0
        && page_count <= PAGE_LIMIT
        && (physical_start & 4095) == 0
        && physical_start <= PHYSICAL_LIMIT - page_count * 4096
        && (virtual_start & 4095) == 0
        && virtual_start <= u64::MAX - page_count * 4096
        && (attributes & !EFI_MEMORY_ATTRIBUTE_MASK) == 0
        && ((attributes & EFI_MEMORY_ISA_MASK) == 0
            || (attributes & EFI_MEMORY_ISA_VALID) != 0)
        && !((memory_type == 7 || memory_type == 11 || memory_type == 12)
            && spec_cache_class(attributes) == 0)
        && (!(memory_type == 5 || memory_type == 6)
            || (attributes & EFI_MEMORY_RUNTIME) != 0)
        && ((attributes & EFI_MEMORY_RUNTIME) == 0
            || memory_type == 5 || memory_type == 6
            || memory_type == 11 || memory_type == 12)
}

pub open spec fn spec_descriptor_ordered(
    bytes: &[u8],
    descriptor_size: u64,
    index: int,
) -> bool {
    let previous = (index - 1) * descriptor_size as int;
    let current = index * descriptor_size as int;
    spec_read_u64(bytes, previous + 8)
        + spec_read_u64(bytes, previous + 24) * 4096
        <= spec_read_u64(bytes, current + 8)
}

pub open spec fn spec_descriptors_valid(
    bytes: &[u8],
    descriptor_size: u64,
    count: u64,
) -> bool {
    (forall|index: int| 0 <= index < count as int ==>
        spec_descriptor_valid(bytes, descriptor_size, index))
        && (forall|index: int| 1 <= index < count as int ==>
            spec_descriptor_ordered(bytes, descriptor_size, index))
}

pub open spec fn raw_map_accepted(bytes: &[u8], result: &RawMapValidation) -> bool {
    bytes.len() > 0
        && bytes.len() <= RAW_MAP_LIMIT
        && result.map_key != 0
        && result.descriptor_size >= 40
        && result.descriptor_size <= 256
        && result.descriptor_size & 7 == 0
        && result.descriptor_version == 1
        && result.descriptor_count >= 1
        && result.descriptor_count <= DESCRIPTOR_LIMIT
        && bytes.len() as u64 % result.descriptor_size == 0
        && bytes.len() as u64 / result.descriptor_size == result.descriptor_count
        && spec_descriptors_valid(bytes, result.descriptor_size, result.descriptor_count)
        && result.usable_pages != 0
        && result.last_end ==
            spec_read_u64(
                bytes,
                (result.descriptor_count as int - 1) * result.descriptor_size as int + 8,
            ) + spec_read_u64(
                bytes,
                (result.descriptor_count as int - 1) * result.descriptor_size as int + 24,
            ) * 4096
}

fn failed(code: u32) -> (result: RawMapValidation)
    requires code != 0,
    ensures
        result.code == code,
        result.map_key == 0,
        result.descriptor_size == 0,
        result.descriptor_version == 0,
        result.descriptor_count == 0,
        result.last_end == 0,
        result.usable_pages == 0,
{
    RawMapValidation {
        code,
        map_key: 0,
        descriptor_size: 0,
        descriptor_version: 0,
        descriptor_count: 0,
        last_end: 0,
        usable_pages: 0,
    }
}

fn read_u32(bytes: &[u8], offset: usize) -> (result: u32)
    requires offset + 4 <= bytes.len(),
    ensures result == spec_read_u32(bytes, offset as int),
{
    (bytes[offset] as u32)
        | ((bytes[offset + 1] as u32) << 8)
        | ((bytes[offset + 2] as u32) << 16)
        | ((bytes[offset + 3] as u32) << 24)
}

fn read_u64(bytes: &[u8], offset: usize) -> (result: u64)
    requires offset + 8 <= bytes.len(),
    ensures result == spec_read_u64(bytes, offset as int),
{
    (bytes[offset] as u64)
        | ((bytes[offset + 1] as u64) << 8)
        | ((bytes[offset + 2] as u64) << 16)
        | ((bytes[offset + 3] as u64) << 24)
        | ((bytes[offset + 4] as u64) << 32)
        | ((bytes[offset + 5] as u64) << 40)
        | ((bytes[offset + 6] as u64) << 48)
        | ((bytes[offset + 7] as u64) << 56)
}

fn cache_class(attributes: u64) -> (result: u32)
    ensures result == spec_cache_class(attributes), result <= 5,
{
    if attributes & 8 != 0 {
        4
    } else if attributes & 4 != 0 {
        3
    } else if attributes & 2 != 0 {
        2
    } else if attributes & 1 != 0 {
        1
    } else if attributes & 16 != 0 {
        5
    } else {
        0
    }
}

pub fn validate_raw_memory_map(
    bytes: &[u8],
    returned_size: u64,
    descriptor_size: u64,
    descriptor_version: u32,
    map_key: u64,
) -> (result: RawMapValidation)
    ensures
        result.code == 0 ==> result.map_key == map_key,
        result.code == 0 ==> result.descriptor_size == descriptor_size,
        result.code == 0 ==> result.descriptor_version == descriptor_version,
        result.code == 0 ==> result.descriptor_count <= DESCRIPTOR_LIMIT,
        result.code == 0 ==> result.last_end <= PHYSICAL_LIMIT,
        result.code == 0 ==> result.usable_pages <= PAGE_LIMIT,
        result.code == 0 ==> raw_map_accepted(bytes, &result),
{
    if bytes.len() == 0 || bytes.len() > RAW_MAP_LIMIT as usize {
        return failed(100);
    }
    if returned_size != bytes.len() as u64 {
        return failed(101);
    }
    if descriptor_size < 40 || descriptor_size > 256 || descriptor_size & 7 != 0
        || descriptor_version != 1 || map_key == 0
        || returned_size % descriptor_size != 0
    {
        return failed(102);
    }
    let descriptor_count = returned_size / descriptor_size;
    if descriptor_count == 0 || descriptor_count > DESCRIPTOR_LIMIT {
        return failed(103);
    }

    let initial = MemoryMapState {
        phase: 0,
        expected_descriptors: 0,
        accepted_descriptors: 0,
        last_end: 0,
        usable_pages: 0,
    };
    let header = memory_map_step(
        initial,
        MemoryMapEvent::Header {
            descriptor_size,
            descriptor_version,
            descriptor_count,
            raw_size: returned_size,
        },
    );
    let mut policy = match header.1 {
        MemoryMapAction::HeaderAccepted => header.0,
        MemoryMapAction::Reject { code } => return failed(code),
        MemoryMapAction::RangeAccepted { kind: _ } => return failed(104),
        MemoryMapAction::Complete => return failed(105),
    };
    assert(returned_size == bytes.len() as u64);

    let mut index: u64 = 0;
    while index < descriptor_count
        invariant
            policy.well_formed(),
            policy.phase == 1,
            policy.expected_descriptors == descriptor_count,
            policy.accepted_descriptors == index,
            index <= descriptor_count,
            descriptor_count >= 1,
            descriptor_count <= DESCRIPTOR_LIMIT,
            descriptor_size >= 40,
            descriptor_size <= 256,
            descriptor_size & 7 == 0,
            descriptor_version == 1,
            map_key != 0,
            returned_size == bytes.len() as u64,
            returned_size <= RAW_MAP_LIMIT,
            policy.last_end <= PHYSICAL_LIMIT,
            policy.usable_pages <= PAGE_LIMIT,
            index == 0 ==> policy.last_end == 0,
            index > 0 ==> policy.last_end ==
                spec_read_u64(bytes, (index as int - 1) * descriptor_size as int + 8)
                + spec_read_u64(bytes, (index as int - 1) * descriptor_size as int + 24) * 4096,
            forall|prior: int| 0 <= prior < index as int ==>
                spec_descriptor_valid(bytes, descriptor_size, prior),
            forall|prior: int| 1 <= prior < index as int ==>
                spec_descriptor_ordered(bytes, descriptor_size, prior),
        decreases descriptor_count - index,
    {
        let offset_u64 = match index.checked_mul(descriptor_size) {
            Some(value) => value,
            None => return failed(111),
        };
        if descriptor_size > returned_size || offset_u64 > returned_size - descriptor_size {
            return failed(111);
        }
        let offset = offset_u64 as usize;
        assert(offset_u64 + 40 <= returned_size);
        assert(offset + 40 <= bytes.len());
        let memory_type = read_u32(bytes, offset);
        let physical_start = read_u64(bytes, offset + 8);
        let virtual_start = read_u64(bytes, offset + 16);
        let page_count = read_u64(bytes, offset + 24);
        let attributes = read_u64(bytes, offset + 32);
        if page_count == 0 || page_count > PAGE_LIMIT
            || virtual_start & 4095 != 0
            || virtual_start > u64::MAX - page_count * 4096
            || attributes & !EFI_MEMORY_ATTRIBUTE_MASK != 0
            || ((attributes & EFI_MEMORY_ISA_MASK) != 0
                && (attributes & EFI_MEMORY_ISA_VALID) == 0)
        {
            return failed(106);
        }
        let class = cache_class(attributes);
        let runtime = attributes & EFI_MEMORY_RUNTIME != 0;
        let next = memory_map_step(
            policy,
            MemoryMapEvent::Descriptor {
                memory_type,
                physical_start,
                page_count,
                cache_class: class,
                runtime,
                attributes_known: true,
            },
        );
        let accepted = match next.1 {
            MemoryMapAction::RangeAccepted { kind: _ } => {
                assert(spec_descriptor_valid(bytes, descriptor_size, index as int));
                if index > 0 {
                    assert(spec_descriptor_ordered(bytes, descriptor_size, index as int));
                }
                next.0
            },
            MemoryMapAction::Reject { code } => return failed(code),
            MemoryMapAction::HeaderAccepted => return failed(107),
            MemoryMapAction::Complete => return failed(108),
        };
        policy = accepted;
        index = index + 1;
    }

    let finish = memory_map_step(policy, MemoryMapEvent::Finish);
    match finish.1 {
        MemoryMapAction::Complete => {
            let accepted = RawMapValidation {
                code: 0,
                map_key,
                descriptor_size,
                descriptor_version,
                descriptor_count: finish.0.accepted_descriptors,
                last_end: finish.0.last_end,
                usable_pages: finish.0.usable_pages,
            };
            assert(spec_descriptors_valid(bytes, descriptor_size, descriptor_count));
            assert(raw_map_accepted(bytes, &accepted));
            accepted
        },
        MemoryMapAction::Reject { code } => failed(code),
        MemoryMapAction::HeaderAccepted => failed(109),
        MemoryMapAction::RangeAccepted { kind: _ } => failed(110),
    }
}
