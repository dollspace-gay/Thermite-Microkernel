fn initial_address_plan_state() -> (result: AddressPlanState)
    ensures
        result.well_formed(),
        result.phase == 0,
        result.expected_regions == 0,
        result.accepted_regions == 0,
        result.last_virtual_end == 0,
        result.kernel_virt_end == 0,
        result.image_virtual_next == 0,
        result.kernel_phys_start == 0,
        result.kernel_phys_end == 0,
        result.image_physical_next == 0,
        !result.saw_text,
        !result.saw_rodata,
        !result.saw_data,
        !result.saw_stack,
{
    AddressPlanState {
        phase: 0,
        expected_regions: 0,
        accepted_regions: 0,
        last_virtual_end: 0,
        kernel_virt_end: 0,
        image_virtual_next: 0,
        kernel_phys_start: 0,
        kernel_phys_end: 0,
        image_physical_next: 0,
        saw_text: false,
        saw_rodata: false,
        saw_data: false,
        saw_stack: false,
    }
}

fn valid_address_header() -> (result: (AddressPlanState, AddressPlanAction))
    ensures
        result.0.well_formed(),
        result.0.phase == 1,
        result.0.expected_regions == 6,
        result.0.accepted_regions == 0,
        result.0.kernel_virt_end == 0xffff_ffff_8000_6000,
        result.0.image_virtual_next == 0xffff_ffff_8000_0000,
        result.0.kernel_phys_start == 0x200000,
        result.0.kernel_phys_end == 0x206000,
        result.0.image_physical_next == 0x200000,
        !result.0.saw_text,
        !result.0.saw_rodata,
        !result.0.saw_data,
        !result.0.saw_stack,
        matches!(result.1, AddressPlanAction::HeaderAccepted),
{
    assert((0x200000u64 & 4095u64) == 0u64) by(bit_vector);
    assert((0x206000u64 & 4095u64) == 0u64) by(bit_vector);
    address_plan_step(
        initial_address_plan_state(),
        AddressPlanEvent::Header {
            region_count: 6,
            kernel_phys_start: 0x200000,
            kernel_phys_end: 0x206000,
            kernel_virt_start: 0xffff_ffff_8000_0000,
            kernel_virt_end: 0xffff_ffff_8000_6000,
            low_guard_unmapped: true,
            recursive_mapping_absent: true,
        },
    )
}

fn incomplete_address_header() -> (result: (AddressPlanState, AddressPlanAction))
    ensures
        result.0.well_formed(),
        result.0.phase == 1,
        result.0.expected_regions == 4,
        result.0.accepted_regions == 0,
        result.0.kernel_virt_end == 0xffff_ffff_8000_6000,
        result.0.image_virtual_next == 0xffff_ffff_8000_0000,
        result.0.kernel_phys_start == 0x200000,
        result.0.kernel_phys_end == 0x206000,
        result.0.image_physical_next == 0x200000,
        !result.0.saw_text,
        !result.0.saw_rodata,
        !result.0.saw_data,
        !result.0.saw_stack,
        matches!(result.1, AddressPlanAction::HeaderAccepted),
{
    assert((0x200000u64 & 4095u64) == 0u64) by(bit_vector);
    assert((0x206000u64 & 4095u64) == 0u64) by(bit_vector);
    address_plan_step(
        initial_address_plan_state(),
        AddressPlanEvent::Header {
            region_count: 4,
            kernel_phys_start: 0x200000,
            kernel_phys_end: 0x206000,
            kernel_virt_start: 0xffff_ffff_8000_0000,
            kernel_virt_end: 0xffff_ffff_8000_6000,
            low_guard_unmapped: true,
            recursive_mapping_absent: true,
        },
    )
}

pub fn address_space_policy_observation() -> (result: u64)
    ensures result == 511,
{
    assert((0x1000u64 & 4095u64) == 0u64) by(bit_vector);
    assert((0x2000u64 & 4095u64) == 0u64) by(bit_vector);
    assert((0x1800u64 & 4095u64) != 0u64) by(bit_vector);
    assert((0x100000u64 & 4095u64) == 0u64) by(bit_vector);
    assert((0x200000u64 & 4095u64) == 0u64) by(bit_vector);
    assert((0x202000u64 & 4095u64) == 0u64) by(bit_vector);
    assert((0x203000u64 & 4095u64) == 0u64) by(bit_vector);
    assert((0x204000u64 & 4095u64) == 0u64) by(bit_vector);
    assert((0x206000u64 & 4095u64) == 0u64) by(bit_vector);
    assert((0x300000u64 & 4095u64) == 0u64) by(bit_vector);
    assert((0x301000u64 & 4095u64) == 0u64) by(bit_vector);
    assert((0xffff_8000_0010_0000u64 & 4095u64) == 0u64) by(bit_vector);
    assert((0xffff_8000_0020_0000u64 & 4095u64) == 0u64) by(bit_vector);
    assert((0xffff_8000_0010_1000u64 & 4095u64) == 0u64) by(bit_vector);
    assert((0xffff_c000_0000_0000u64 & 4095u64) == 0u64) by(bit_vector);
    assert((0xffff_e000_0000_0000u64 & 4095u64) == 0u64) by(bit_vector);
    assert((0xffff_ffff_8000_0000u64 & 4095u64) == 0u64) by(bit_vector);
    assert((0xffff_ffff_8000_2000u64 & 4095u64) == 0u64) by(bit_vector);
    assert((0xffff_ffff_8000_3000u64 & 4095u64) == 0u64) by(bit_vector);
    assert((0xffff_ffff_8000_4000u64 & 4095u64) == 0u64) by(bit_vector);
    assert(!((3u32 & 2u32) == 2u32 && (3u32 & 4u32) == 4u32)) by(bit_vector);
    assert(!((5u32 & 2u32) == 2u32 && (5u32 & 4u32) == 4u32)) by(bit_vector);
    assert(!((1u32 & 2u32) == 2u32 && (1u32 & 4u32) == 4u32)) by(bit_vector);
    assert((7u32 & 2u32) == 2u32 && (7u32 & 4u32) == 4u32) by(bit_vector);

    let direct = address_plan_step(
        valid_address_header().0,
        AddressPlanEvent::Region {
            kind: 1,
            virtual_start: 0xffff_8000_0010_0000,
            physical_start: 0x100000,
            length: 0x1000,
            flags: 3,
            guard_before: false,
            guard_after: false,
        },
    );
    match direct.1 {
        AddressPlanAction::RegionAccepted { kind } => assert(kind == 1),
        _ => assert(false),
    }
    assert(direct.0.phase == 1);
    assert(direct.0.accepted_regions == 1);
    assert(direct.0.last_virtual_end == 0xffff_8000_0010_1000);
    assert(direct.0.image_virtual_next == 0xffff_ffff_8000_0000);
    assert(direct.0.image_physical_next == 0x200000);

    let heap = address_plan_step(
        direct.0,
        AddressPlanEvent::Region {
            kind: 2,
            virtual_start: 0xffff_c000_0000_0000,
            physical_start: 0x300000,
            length: 0x1000,
            flags: 3,
            guard_before: false,
            guard_after: false,
        },
    );
    match heap.1 {
        AddressPlanAction::RegionAccepted { kind } => assert(kind == 2),
        _ => assert(false),
    }
    assert(heap.0.accepted_regions == 2);
    assert(heap.0.last_virtual_end == 0xffff_c000_0000_1000);
    assert(heap.0.image_virtual_next == 0xffff_ffff_8000_0000);
    assert(heap.0.image_physical_next == 0x200000);

    let stack = address_plan_step(
        heap.0,
        AddressPlanEvent::Region {
            kind: 3,
            virtual_start: 0xffff_e000_0000_0000,
            physical_start: 0x301000,
            length: 0x1000,
            flags: 3,
            guard_before: true,
            guard_after: true,
        },
    );
    match stack.1 {
        AddressPlanAction::RegionAccepted { kind } => assert(kind == 3),
        _ => assert(false),
    }
    assert(stack.0.accepted_regions == 3);
    assert(stack.0.last_virtual_end == 0xffff_e000_0000_1000);
    assert(stack.0.image_virtual_next == 0xffff_ffff_8000_0000);
    assert(stack.0.image_physical_next == 0x200000);
    assert(stack.0.saw_stack);

    let text = address_plan_step(
        stack.0,
        AddressPlanEvent::Region {
            kind: 4,
            virtual_start: 0xffff_ffff_8000_0000,
            physical_start: 0x200000,
            length: 0x2000,
            flags: 5,
            guard_before: false,
            guard_after: false,
        },
    );
    match text.1 {
        AddressPlanAction::RegionAccepted { kind } => assert(kind == 4),
        _ => assert(false),
    }
    assert(text.0.accepted_regions == 4);
    assert(text.0.last_virtual_end == 0xffff_ffff_8000_2000);
    assert(text.0.image_virtual_next == 0xffff_ffff_8000_2000);
    assert(text.0.image_physical_next == 0x202000);
    assert(text.0.saw_text);
    assert(!text.0.saw_rodata);
    assert(!text.0.saw_data);
    assert(0x2000u64 <= 0x206000u64 - 0x202000u64);
    assert(0x2000u64 <= 0x0010_0000_0000_0000u64 - 0x202000u64);
    assert(0xffff_ffff_8000_2000u64 <=
        0xffff_ffff_ffff_ffffu64 - 0x2000u64);
    assert(0xffff_ffff_8000_2000u64 + 0x2000u64 <=
        0xffff_ffff_c000_0000u64);

    let rodata = address_plan_step(
        text.0,
        AddressPlanEvent::Region {
            kind: 5,
            virtual_start: 0xffff_ffff_8000_2000,
            physical_start: 0x202000,
            length: 0x2000,
            flags: 1,
            guard_before: false,
            guard_after: false,
        },
    );
    match rodata.1 {
        AddressPlanAction::RegionAccepted { kind } => assert(kind == 5),
        _ => assert(false),
    }
    assert(rodata.0.accepted_regions == 5);
    assert(rodata.0.last_virtual_end == 0xffff_ffff_8000_4000);
    assert(rodata.0.image_virtual_next == 0xffff_ffff_8000_4000);
    assert(rodata.0.image_physical_next == 0x204000);
    assert(rodata.0.saw_text);
    assert(rodata.0.saw_rodata);
    assert(!rodata.0.saw_data);

    let data = address_plan_step(
        rodata.0,
        AddressPlanEvent::Region {
            kind: 6,
            virtual_start: 0xffff_ffff_8000_4000,
            physical_start: 0x204000,
            length: 0x2000,
            flags: 3,
            guard_before: false,
            guard_after: false,
        },
    );
    match data.1 {
        AddressPlanAction::RegionAccepted { kind } => assert(kind == 6),
        _ => assert(false),
    }
    assert(data.0.accepted_regions == 6);
    assert(data.0.last_virtual_end == 0xffff_ffff_8000_6000);
    assert(data.0.image_virtual_next == 0xffff_ffff_8000_6000);
    assert(data.0.image_physical_next == 0x206000);
    assert(data.0.saw_data);
    assert(data.0.saw_stack);
    let complete = address_plan_step(data.0, AddressPlanEvent::Finish);
    match complete.1 {
        AddressPlanAction::Complete => {},
        _ => assert(false),
    }
    assert(complete.0.phase == 2);

    let alias = address_plan_step(
        valid_address_header().0,
        AddressPlanEvent::Region {
            kind: 1,
            virtual_start: 0xffff_8000_0020_0000,
            physical_start: 0x200000,
            length: 0x1000,
            flags: 3,
            guard_before: false,
            guard_after: false,
        },
    );
    match alias.1 {
        AddressPlanAction::Reject { code } => assert(code == 13),
        _ => assert(false),
    }

    let wrong_direct_offset = address_plan_step(
        valid_address_header().0,
        AddressPlanEvent::Region {
            kind: 1,
            virtual_start: 0xffff_8000_0010_1000,
            physical_start: 0x100000,
            length: 0x1000,
            flags: 3,
            guard_before: false,
            guard_after: false,
        },
    );
    match wrong_direct_offset.1 {
        AddressPlanAction::Reject { code } => assert(code == 13),
        _ => assert(false),
    }

    let writable_executable = address_plan_step(
        valid_address_header().0,
        AddressPlanEvent::Region {
            kind: 4,
            virtual_start: 0xffff_ffff_8000_0000,
            physical_start: 0x200000,
            length: 0x2000,
            flags: 7,
            guard_before: false,
            guard_after: false,
        },
    );
    match writable_executable.1 {
        AddressPlanAction::Reject { code } => assert(code == 12),
        _ => assert(false),
    }

    let missing_guard = address_plan_step(
        valid_address_header().0,
        AddressPlanEvent::Region {
            kind: 3,
            virtual_start: 0xffff_e000_0000_0000,
            physical_start: 0x301000,
            length: 0x1000,
            flags: 3,
            guard_before: false,
            guard_after: true,
        },
    );
    match missing_guard.1 {
        AddressPlanAction::Reject { code } => assert(code == 13),
        _ => assert(false),
    }

    let rodata_first = address_plan_step(
        valid_address_header().0,
        AddressPlanEvent::Region {
            kind: 5,
            virtual_start: 0xffff_ffff_8000_2000,
            physical_start: 0x200000,
            length: 0x2000,
            flags: 1,
            guard_before: false,
            guard_after: false,
        },
    );
    match rodata_first.1 {
        AddressPlanAction::Reject { code } => assert(code == 13),
        _ => assert(false),
    }

    let bad_length = address_plan_step(
        valid_address_header().0,
        AddressPlanEvent::Region {
            kind: 1,
            virtual_start: 0xffff_8000_0010_0000,
            physical_start: 0x100000,
            length: 0x1800,
            flags: 3,
            guard_before: false,
            guard_after: false,
        },
    );
    match bad_length.1 {
        AddressPlanAction::Reject { code } => assert(code == 12),
        _ => assert(false),
    }

    let bad_header = address_plan_step(
        initial_address_plan_state(),
        AddressPlanEvent::Header {
            region_count: 6,
            kernel_phys_start: 0x200000,
            kernel_phys_end: 0x206000,
            kernel_virt_start: 0xffff_ffff_8000_0000,
            kernel_virt_end: 0xffff_ffff_8000_6000,
            low_guard_unmapped: false,
            recursive_mapping_absent: true,
        },
    );
    assert(0xffff_ffff_8000_0000u64 +
        (0x206000u64 - 0x200000u64) == 0xffff_ffff_8000_6000u64);
    match bad_header.1 {
        AddressPlanAction::Reject { code } => assert(code == 3),
        _ => assert(false),
    }

    let text_first = address_plan_step(
        valid_address_header().0,
        AddressPlanEvent::Region {
            kind: 4,
            virtual_start: 0xffff_ffff_8000_0000,
            physical_start: 0x200000,
            length: 0x2000,
            flags: 5,
            guard_before: false,
            guard_after: false,
        },
    );
    match text_first.1 {
        AddressPlanAction::RegionAccepted { kind } => assert(kind == 4),
        _ => assert(false),
    }
    let physical_gap = address_plan_step(
        text_first.0,
        AddressPlanEvent::Region {
            kind: 5,
            virtual_start: 0xffff_ffff_8000_2000,
            physical_start: 0x203000,
            length: 0x1000,
            flags: 1,
            guard_before: false,
            guard_after: false,
        },
    );
    match physical_gap.1 {
        AddressPlanAction::Reject { code } => assert(code == 13),
        _ => assert(false),
    }

    let virtual_gap_text = address_plan_step(
        valid_address_header().0,
        AddressPlanEvent::Region {
            kind: 4,
            virtual_start: 0xffff_ffff_8000_0000,
            physical_start: 0x200000,
            length: 0x2000,
            flags: 5,
            guard_before: false,
            guard_after: false,
        },
    );
    match virtual_gap_text.1 {
        AddressPlanAction::RegionAccepted { kind } => assert(kind == 4),
        _ => assert(false),
    }
    let virtual_gap = address_plan_step(
        virtual_gap_text.0,
        AddressPlanEvent::Region {
            kind: 5,
            virtual_start: 0xffff_ffff_8000_3000,
            physical_start: 0x202000,
            length: 0x1000,
            flags: 1,
            guard_before: false,
            guard_after: false,
        },
    );
    match virtual_gap.1 {
        AddressPlanAction::Reject { code } => assert(code == 13),
        _ => assert(false),
    }

    let direct_first = address_plan_step(
        valid_address_header().0,
        AddressPlanEvent::Region {
            kind: 1,
            virtual_start: 0xffff_8000_0010_0000,
            physical_start: 0x100000,
            length: 0x1000,
            flags: 3,
            guard_before: false,
            guard_after: false,
        },
    );
    match direct_first.1 {
        AddressPlanAction::RegionAccepted { kind } => assert(kind == 1),
        _ => assert(false),
    }
    let virtual_overlap = address_plan_step(
        direct_first.0,
        AddressPlanEvent::Region {
            kind: 1,
            virtual_start: 0xffff_8000_0010_0000,
            physical_start: 0x101000,
            length: 0x1000,
            flags: 3,
            guard_before: false,
            guard_after: false,
        },
    );
    match virtual_overlap.1 {
        AddressPlanAction::Reject { code } => assert(code == 12),
        _ => assert(false),
    }

    let incomplete_direct = address_plan_step(
        incomplete_address_header().0,
        AddressPlanEvent::Region {
            kind: 1,
            virtual_start: 0xffff_8000_0010_0000,
            physical_start: 0x100000,
            length: 0x1000,
            flags: 3,
            guard_before: false,
            guard_after: false,
        },
    );
    match incomplete_direct.1 {
        AddressPlanAction::RegionAccepted { kind } => assert(kind == 1),
        _ => assert(false),
    }
    let incomplete_stack = address_plan_step(
        incomplete_direct.0,
        AddressPlanEvent::Region {
            kind: 3,
            virtual_start: 0xffff_e000_0000_0000,
            physical_start: 0x301000,
            length: 0x1000,
            flags: 3,
            guard_before: true,
            guard_after: true,
        },
    );
    match incomplete_stack.1 {
        AddressPlanAction::RegionAccepted { kind } => assert(kind == 3),
        _ => assert(false),
    }
    let incomplete_text = address_plan_step(
        incomplete_stack.0,
        AddressPlanEvent::Region {
            kind: 4,
            virtual_start: 0xffff_ffff_8000_0000,
            physical_start: 0x200000,
            length: 0x2000,
            flags: 5,
            guard_before: false,
            guard_after: false,
        },
    );
    match incomplete_text.1 {
        AddressPlanAction::RegionAccepted { kind } => assert(kind == 4),
        _ => assert(false),
    }
    let incomplete_rodata = address_plan_step(
        incomplete_text.0,
        AddressPlanEvent::Region {
            kind: 5,
            virtual_start: 0xffff_ffff_8000_2000,
            physical_start: 0x202000,
            length: 0x2000,
            flags: 1,
            guard_before: false,
            guard_after: false,
        },
    );
    match incomplete_rodata.1 {
        AddressPlanAction::RegionAccepted { kind } => assert(kind == 5),
        _ => assert(false),
    }
    let incomplete_finish = address_plan_step(incomplete_rodata.0, AddressPlanEvent::Finish);
    match incomplete_finish.1 {
        AddressPlanAction::Reject { code } => assert(code == 20),
        _ => assert(false),
    }

    511
}
