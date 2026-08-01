fn initial_memory_map_state() -> (result: MemoryMapState)
    ensures
        result.well_formed(),
        result.phase == 0,
        result.expected_descriptors == 0,
        result.accepted_descriptors == 0,
        result.last_end == 0,
        result.usable_pages == 0,
{
    MemoryMapState {
        phase: 0,
        expected_descriptors: 0,
        accepted_descriptors: 0,
        last_end: 0,
        usable_pages: 0,
    }
}

fn valid_memory_header(count: u64) -> (result: (MemoryMapState, MemoryMapAction))
    requires count >= 1, count <= 4096,
    ensures
        result.0.well_formed(),
        result.0.phase == 1,
        result.0.expected_descriptors == count,
        result.0.accepted_descriptors == 0,
        result.0.last_end == 0,
        result.0.usable_pages == 0,
        matches!(result.1, MemoryMapAction::HeaderAccepted),
{
    assert((48u64 & 7u64) == 0u64) by(bit_vector);
    memory_map_step(
        initial_memory_map_state(),
        MemoryMapEvent::Header {
            descriptor_size: 48,
            descriptor_version: 1,
            descriptor_count: count,
            raw_size: 48 * count,
        },
    )
}

fn initial_firmware_exit_state() -> (result: FirmwareExitState)
    ensures
        result.well_formed(),
        result.phase == 0,
        result.map_attempts == 0,
        result.exit_retries == 0,
        result.capacity == 0,
        result.map_key == 0,
        result.descriptor_count == 0,
{
    FirmwareExitState {
        phase: 0,
        map_attempts: 0,
        exit_retries: 0,
        capacity: 0,
        map_key: 0,
        descriptor_count: 0,
    }
}

pub fn firmware_policy_observation() -> (result: u64)
    ensures result == 255,
{
    assert((48u64 & 7u64) == 0u64) by(bit_vector);
    assert((44u64 & 7u64) != 0u64) by(bit_vector);
    assert((0u64 & 4095u64) == 0u64) by(bit_vector);
    assert((0x1000u64 & 4095u64) == 0u64) by(bit_vector);
    assert((0x20000u64 & 4095u64) == 0u64) by(bit_vector);
    assert(1u64 * 4096u64 == 4096u64);
    assert(16u64 * 4096u64 == 65536u64);
    assert(0u64 <= 0x0010_0000_0000_0000u64 - 4096u64);
    assert(0x1000u64 <= 0x0010_0000_0000_0000u64 - 65536u64);
    assert(0x20000u64 <= 0x0010_0000_0000_0000u64 - 4096u64);
    assert(0u64 <= 0x0010_0000_0000_0000u64 - 1u64 * 4096u64);
    assert(!((0u32 == 7u32 || 0u32 == 11u32 || 0u32 == 12u32) && 0u32 == 0u32));
    assert(false == (0u32 == 5u32 || 0u32 == 6u32 || 0u32 == 11u32 || 0u32 == 12u32));

    let map_header = valid_memory_header(3);
    let reserved = memory_map_step(
        map_header.0,
        MemoryMapEvent::Descriptor {
            memory_type: 0,
            physical_start: 0,
            page_count: 1,
            cache_class: 0,
            runtime: false,
            attributes_known: true,
        },
    );
    match reserved.1 {
        MemoryMapAction::RangeAccepted { kind } => assert(kind == 1),
        _ => assert(false),
    }

    let conventional = memory_map_step(
        reserved.0,
        MemoryMapEvent::Descriptor {
            memory_type: 7,
            physical_start: 0x1000,
            page_count: 16,
            cache_class: 4,
            runtime: false,
            attributes_known: true,
        },
    );
    match conventional.1 {
        MemoryMapAction::RangeAccepted { kind } => assert(kind == 4),
        _ => assert(false),
    }
    assert(conventional.0.usable_pages == 16);

    let mmio = memory_map_step(
        conventional.0,
        MemoryMapEvent::Descriptor {
            memory_type: 11,
            physical_start: 0x20000,
            page_count: 1,
            cache_class: 1,
            runtime: true,
            attributes_known: true,
        },
    );
    match mmio.1 {
        MemoryMapAction::RangeAccepted { kind } => assert(kind == 8),
        _ => assert(false),
    }
    let normalized = memory_map_step(mmio.0, MemoryMapEvent::Finish);
    match normalized.1 {
        MemoryMapAction::Complete => {},
        _ => assert(false),
    }
    assert(normalized.0.phase == 2);

    let bad_size = memory_map_step(
        initial_memory_map_state(),
        MemoryMapEvent::Header {
            descriptor_size: 44,
            descriptor_version: 1,
            descriptor_count: 3,
            raw_size: 132,
        },
    );
    match bad_size.1 {
        MemoryMapAction::Reject { code } => assert(code == 2),
        _ => assert(false),
    }

    let overlap_first = memory_map_step(
        valid_memory_header(2).0,
        MemoryMapEvent::Descriptor {
            memory_type: 7,
            physical_start: 0,
            page_count: 2,
            cache_class: 4,
            runtime: false,
            attributes_known: true,
        },
    );
    match overlap_first.1 {
        MemoryMapAction::RangeAccepted { kind } => assert(kind == 4),
        _ => assert(false),
    }
    let overlap = memory_map_step(
        overlap_first.0,
        MemoryMapEvent::Descriptor {
            memory_type: 7,
            physical_start: 0x1000,
            page_count: 1,
            cache_class: 4,
            runtime: false,
            attributes_known: true,
        },
    );
    match overlap.1 {
        MemoryMapAction::Reject { code } => assert(code == 12),
        _ => assert(false),
    }

    let runtime_mismatch = memory_map_step(
        valid_memory_header(1).0,
        MemoryMapEvent::Descriptor {
            memory_type: 5,
            physical_start: 0,
            page_count: 1,
            cache_class: 4,
            runtime: false,
            attributes_known: true,
        },
    );
    match runtime_mismatch.1 {
        MemoryMapAction::Reject { code } => assert(code == 13),
        _ => assert(false),
    }

    let unknown_attributes = memory_map_step(
        valid_memory_header(1).0,
        MemoryMapEvent::Descriptor {
            memory_type: 7,
            physical_start: 0,
            page_count: 1,
            cache_class: 4,
            runtime: false,
            attributes_known: false,
        },
    );
    match unknown_attributes.1 {
        MemoryMapAction::Reject { code } => assert(code == 13),
        _ => assert(false),
    }

    let no_usable_range = memory_map_step(
        valid_memory_header(1).0,
        MemoryMapEvent::Descriptor {
            memory_type: 0,
            physical_start: 0,
            page_count: 1,
            cache_class: 0,
            runtime: false,
            attributes_known: true,
        },
    );
    match no_usable_range.1 {
        MemoryMapAction::RangeAccepted { kind } => assert(kind == 1),
        _ => assert(false),
    }
    let no_usable = memory_map_step(no_usable_range.0, MemoryMapEvent::Finish);
    match no_usable.1 {
        MemoryMapAction::Reject { code } => assert(code == 20),
        _ => assert(false),
    }

    let resize = firmware_exit_step(
        initial_firmware_exit_state(),
        FirmwareExitEvent::MapResult {
            status: 1,
            required_size: 4096,
            returned_size: 0,
            descriptor_size: 48,
            descriptor_version: 1,
            descriptor_count: 0,
            map_key: 0,
            capacity: 0,
        },
    );
    match resize.1 {
        FirmwareExitAction::Resize { capacity } => assert(capacity == 4608),
        _ => assert(false),
    }

    let first_map = firmware_exit_step(
        resize.0,
        FirmwareExitEvent::MapResult {
            status: 0,
            required_size: 144,
            returned_size: 144,
            descriptor_size: 48,
            descriptor_version: 1,
            descriptor_count: 3,
            map_key: 77,
            capacity: 4608,
        },
    );
    match first_map.1 {
        FirmwareExitAction::MapAccepted => {},
        _ => assert(false),
    }
    let stale_key = firmware_exit_step(
        first_map.0,
        FirmwareExitEvent::ExitResult {
            status: 2,
            map_key: 77,
        },
    );
    match stale_key.1 {
        FirmwareExitAction::Reacquire => {},
        _ => assert(false),
    }
    assert(stale_key.0.exit_retries == 1);

    let fresh_map = firmware_exit_step(
        stale_key.0,
        FirmwareExitEvent::MapResult {
            status: 0,
            required_size: 144,
            returned_size: 144,
            descriptor_size: 48,
            descriptor_version: 1,
            descriptor_count: 3,
            map_key: 78,
            capacity: 4608,
        },
    );
    match fresh_map.1 {
        FirmwareExitAction::MapAccepted => {},
        _ => assert(false),
    }
    let exited = firmware_exit_step(
        fresh_map.0,
        FirmwareExitEvent::ExitResult {
            status: 0,
            map_key: 78,
        },
    );
    match exited.1 {
        FirmwareExitAction::ExitAccepted => {},
        _ => assert(false),
    }
    let firmware_complete = firmware_exit_step(exited.0, FirmwareExitEvent::Finish);
    match firmware_complete.1 {
        FirmwareExitAction::Complete => {},
        _ => assert(false),
    }
    assert(firmware_complete.0.phase == 3);

    let oversize = firmware_exit_step(
        initial_firmware_exit_state(),
        FirmwareExitEvent::MapResult {
            status: 1,
            required_size: 1048065,
            returned_size: 0,
            descriptor_size: 48,
            descriptor_version: 1,
            descriptor_count: 0,
            map_key: 0,
            capacity: 0,
        },
    );
    match oversize.1 {
        FirmwareExitAction::Reject { code } => assert(code == 32),
        _ => assert(false),
    }

    let device_error = firmware_exit_step(
        initial_firmware_exit_state(),
        FirmwareExitEvent::MapResult {
            status: 3,
            required_size: 0,
            returned_size: 0,
            descriptor_size: 48,
            descriptor_version: 1,
            descriptor_count: 0,
            map_key: 0,
            capacity: 0,
        },
    );
    match device_error.1 {
        FirmwareExitAction::Reject { code } => assert(code == 34),
        _ => assert(false),
    }

    let exhausted_map = FirmwareExitState {
        phase: 0,
        map_attempts: 8,
        exit_retries: 0,
        capacity: 4096,
        map_key: 0,
        descriptor_count: 0,
    };
    assert(exhausted_map.well_formed());
    let too_many_maps = firmware_exit_step(
        exhausted_map,
        FirmwareExitEvent::MapResult {
            status: 0,
            required_size: 48,
            returned_size: 48,
            descriptor_size: 48,
            descriptor_version: 1,
            descriptor_count: 1,
            map_key: 9,
            capacity: 4096,
        },
    );
    match too_many_maps.1 {
        FirmwareExitAction::Reject { code } => assert(code == 31),
        _ => assert(false),
    }

    let exhausted_exit = FirmwareExitState {
        phase: 1,
        map_attempts: 4,
        exit_retries: 4,
        capacity: 4096,
        map_key: 9,
        descriptor_count: 1,
    };
    assert(exhausted_exit.well_formed());
    let too_many_exits = firmware_exit_step(
        exhausted_exit,
        FirmwareExitEvent::ExitResult {
            status: 2,
            map_key: 9,
        },
    );
    match too_many_exits.1 {
        FirmwareExitAction::Reject { code } => assert(code == 41),
        _ => assert(false),
    }

    let wrong_key_state = FirmwareExitState {
        phase: 1,
        map_attempts: 1,
        exit_retries: 0,
        capacity: 4096,
        map_key: 9,
        descriptor_count: 1,
    };
    assert(wrong_key_state.well_formed());
    let wrong_key = firmware_exit_step(
        wrong_key_state,
        FirmwareExitEvent::ExitResult {
            status: 0,
            map_key: 10,
        },
    );
    match wrong_key.1 {
        FirmwareExitAction::Reject { code } => assert(code == 40),
        _ => assert(false),
    }

    let zero_key = firmware_exit_step(
        initial_firmware_exit_state(),
        FirmwareExitEvent::MapResult {
            status: 0,
            required_size: 48,
            returned_size: 48,
            descriptor_size: 48,
            descriptor_version: 1,
            descriptor_count: 1,
            map_key: 0,
            capacity: 0,
        },
    );
    match zero_key.1 {
        FirmwareExitAction::Reject { code } => assert(code == 33),
        _ => assert(false),
    }

    255
}
