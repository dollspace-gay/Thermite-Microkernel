fn initial_elf_policy_state() -> (result: ElfPolicyState)
    ensures
        result.well_formed(),
        result.phase == 0,
        result.expected_headers == 0,
        result.seen_headers == 0,
        result.load_segments == 0,
        result.last_virtual_end == 0,
        result.entry == 0,
        result.file_length == 0,
        !result.entry_covered,
{
    ElfPolicyState {
        phase: 0,
        expected_headers: 0,
        seen_headers: 0,
        load_segments: 0,
        last_virtual_end: 0,
        entry: 0,
        file_length: 0,
        entry_covered: false,
    }
}

fn valid_elf_header(state: ElfPolicyState) -> (result: (ElfPolicyState, ElfPolicyAction))
    requires state.well_formed(), state.phase == 0,
    ensures
        result.0.well_formed(),
        result.0.phase == 1,
        result.0.expected_headers == 4,
        result.0.seen_headers == 0,
        result.0.load_segments == 0,
        result.0.last_virtual_end == 0,
        result.0.entry == 0xffff_ffff_8000_0000,
        result.0.file_length == 0x5000,
        !result.0.entry_covered,
        matches!(result.1, ElfPolicyAction::HeaderAccepted),
{
    elf_policy_step(
        state,
        ElfPolicyEvent::Header {
            magic: 0x464c_457f,
            class: 2,
            data: 1,
            ident_version: 1,
            osabi: 0,
            abi_version: 0,
            elf_type: 2,
            machine: 62,
            version: 1,
            entry: 0xffff_ffff_8000_0000,
            program_offset: 64,
            flags: 0,
            header_size: 64,
            program_entry_size: 56,
            program_count: 4,
            file_length: 0x5000,
            digest_valid: true,
        },
    )
}

fn valid_text_segment(state: ElfPolicyState) -> (result: (ElfPolicyState, ElfPolicyAction))
    requires
        state.well_formed(),
        state.phase == 1,
        state.expected_headers == 4,
        state.seen_headers == 0,
        state.load_segments == 0,
        state.last_virtual_end == 0,
        state.entry == 0xffff_ffff_8000_0000,
        state.file_length == 0x5000,
        !state.entry_covered,
    ensures
        result.0.well_formed(),
        result.0.phase == 1,
        result.0.expected_headers == 4,
        result.0.seen_headers == 1,
        result.0.load_segments == 1,
        result.0.last_virtual_end == 0xffff_ffff_8000_1000,
        result.0.entry == 0xffff_ffff_8000_0000,
        result.0.file_length == 0x5000,
        result.0.entry_covered,
        matches!(result.1, ElfPolicyAction::LoadAccepted),
{
    assert((5u32 & 4u32) == 4u32) by(bit_vector);
    assert((5u32 & 1u32) == 1u32) by(bit_vector);
    assert(!((5u32 & 1u32) == 1u32 && (5u32 & 2u32) == 2u32)) by(bit_vector);
    assert(0x1000u64 <= state.file_length);
    assert(0x1000u64 <= state.file_length - 0x1000u64);
    assert(0x1000u64 <=
        0xffff_ffff_c000_0000u64 - 0xffff_ffff_8000_0000u64);
    assert((0x1000u64 & 4095u64) ==
        (0xffff_ffff_8000_0000u64 & 4095u64)) by(bit_vector);
    elf_policy_step(
        state,
        ElfPolicyEvent::ProgramHeader {
            segment_type: 1,
            segment_flags: 5,
            file_offset: 0x1000,
            virtual_address: 0xffff_ffff_8000_0000,
            physical_address: 0xffff_ffff_8000_0000,
            file_size: 0x1000,
            memory_size: 0x1000,
            alignment: 4096,
        },
    )
}

pub fn elf_policy_observation() -> (result: u64)
    ensures result == 127,
{
    let header = valid_elf_header(initial_elf_policy_state());
    let text = valid_text_segment(valid_elf_header(initial_elf_policy_state()).0);
    match text.1 {
        ElfPolicyAction::LoadAccepted => {},
        _ => assert(false),
    }
    assert(text.0.entry_covered);
    assert((5u32 & 4u32) == 4u32) by(bit_vector);
    assert((5u32 & 1u32) == 1u32) by(bit_vector);
    assert(!((5u32 & 1u32) == 1u32 && (5u32 & 2u32) == 2u32)) by(bit_vector);
    assert((0x1000u64 & 4095u64) ==
        (0xffff_ffff_8000_0000u64 & 4095u64)) by(bit_vector);
    assert((6u32 & 4u32) == 4u32) by(bit_vector);
    assert((6u32 & 1u32) == 0u32) by(bit_vector);
    assert(!((6u32 & 1u32) == 1u32 && (6u32 & 2u32) == 2u32)) by(bit_vector);
    assert((0x2000u64 & 4095u64) ==
        (0xffff_ffff_8000_1000u64 & 4095u64)) by(bit_vector);

    let data = elf_policy_step(
        text.0,
        ElfPolicyEvent::ProgramHeader {
            segment_type: 1,
            segment_flags: 6,
            file_offset: 0x2000,
            virtual_address: 0xffff_ffff_8000_1000,
            physical_address: 0xffff_ffff_8000_1000,
            file_size: 0x800,
            memory_size: 0x1000,
            alignment: 4096,
        },
    );
    match data.1 {
        ElfPolicyAction::LoadAccepted => {},
        _ => assert(false),
    }

    let relro = elf_policy_step(
        data.0,
        ElfPolicyEvent::ProgramHeader {
            segment_type: 0x6474_e552,
            segment_flags: 4,
            file_offset: 0x2000,
            virtual_address: 0xffff_ffff_8000_1000,
            physical_address: 0xffff_ffff_8000_1000,
            file_size: 0x800,
            memory_size: 0x800,
            alignment: 4096,
        },
    );
    match relro.1 {
        ElfPolicyAction::MetadataAccepted => {},
        _ => assert(false),
    }

    let stack = elf_policy_step(
        relro.0,
        ElfPolicyEvent::ProgramHeader {
            segment_type: 0x6474_e551,
            segment_flags: 6,
            file_offset: 0,
            virtual_address: 0,
            physical_address: 0,
            file_size: 0,
            memory_size: 0,
            alignment: 16,
        },
    );
    match stack.1 {
        ElfPolicyAction::MetadataAccepted => {},
        _ => assert(false),
    }
    let finished = elf_policy_step(stack.0, ElfPolicyEvent::Finish);
    match finished.1 {
        ElfPolicyAction::Complete => {},
        _ => assert(false),
    }
    assert(finished.0.phase == 2);

    let bad_digest = elf_policy_step(
        initial_elf_policy_state(),
        ElfPolicyEvent::Header {
            magic: 0x464c_457f,
            class: 2,
            data: 1,
            ident_version: 1,
            osabi: 0,
            abi_version: 0,
            elf_type: 2,
            machine: 62,
            version: 1,
            entry: 0xffff_ffff_8000_0000,
            program_offset: 64,
            flags: 0,
            header_size: 64,
            program_entry_size: 56,
            program_count: 4,
            file_length: 0x5000,
            digest_valid: false,
        },
    );
    match bad_digest.1 {
        ElfPolicyAction::Reject { code } => assert(code == 3),
        _ => assert(false),
    }

    assert((7u32 & 1u32) == 1u32 && (7u32 & 2u32) == 2u32) by(bit_vector);
    let wx = elf_policy_step(
        valid_elf_header(initial_elf_policy_state()).0,
        ElfPolicyEvent::ProgramHeader {
            segment_type: 1,
            segment_flags: 7,
            file_offset: 0x1000,
            virtual_address: 0xffff_ffff_8000_0000,
            physical_address: 0xffff_ffff_8000_0000,
            file_size: 0x1000,
            memory_size: 0x1000,
            alignment: 4096,
        },
    );
    match wx.1 {
        ElfPolicyAction::Reject { code } => assert(code == 12),
        _ => assert(false),
    }

    let dynamic = elf_policy_step(
        valid_elf_header(initial_elf_policy_state()).0,
        ElfPolicyEvent::ProgramHeader {
            segment_type: 2,
            segment_flags: 6,
            file_offset: 0x1000,
            virtual_address: 0xffff_ffff_8000_0000,
            physical_address: 0xffff_ffff_8000_0000,
            file_size: 0x1000,
            memory_size: 0x1000,
            alignment: 4096,
        },
    );
    match dynamic.1 {
        ElfPolicyAction::Reject { code } => assert(code == 13),
        _ => assert(false),
    }

    let executable_stack = elf_policy_step(
        valid_elf_header(initial_elf_policy_state()).0,
        ElfPolicyEvent::ProgramHeader {
            segment_type: 0x6474_e551,
            segment_flags: 7,
            file_offset: 0,
            virtual_address: 0,
            physical_address: 0,
            file_size: 0,
            memory_size: 0,
            alignment: 16,
        },
    );
    match executable_stack.1 {
        ElfPolicyAction::Reject { code } => assert(code == 13),
        _ => assert(false),
    }

    assert((4u32 & 4u32) == 4u32) by(bit_vector);
    assert((4u32 & 1u32) == 0u32) by(bit_vector);
    assert(!((4u32 & 1u32) == 1u32 && (4u32 & 2u32) == 2u32)) by(bit_vector);
    let first_nonexec = elf_policy_step(
        valid_elf_header(initial_elf_policy_state()).0,
        ElfPolicyEvent::ProgramHeader {
            segment_type: 1,
            segment_flags: 4,
            file_offset: 0x1000,
            virtual_address: 0xffff_ffff_8000_0000,
            physical_address: 0xffff_ffff_8000_0000,
            file_size: 0x1000,
            memory_size: 0x1000,
            alignment: 4096,
        },
    );
    match first_nonexec.1 {
        ElfPolicyAction::LoadAccepted => {},
        _ => assert(false),
    }
    let second_nonexec = elf_policy_step(
        first_nonexec.0,
        ElfPolicyEvent::ProgramHeader {
            segment_type: 1,
            segment_flags: 6,
            file_offset: 0x2000,
            virtual_address: 0xffff_ffff_8000_1000,
            physical_address: 0xffff_ffff_8000_1000,
            file_size: 0x1000,
            memory_size: 0x1000,
            alignment: 4096,
        },
    );
    match second_nonexec.1 {
        ElfPolicyAction::LoadAccepted => {},
        _ => assert(false),
    }
    let null_one = elf_policy_step(
        second_nonexec.0,
        ElfPolicyEvent::ProgramHeader {
            segment_type: 0,
            segment_flags: 0,
            file_offset: 0,
            virtual_address: 0,
            physical_address: 0,
            file_size: 0,
            memory_size: 0,
            alignment: 0,
        },
    );
    match null_one.1 {
        ElfPolicyAction::MetadataAccepted => {},
        _ => assert(false),
    }
    let null_two = elf_policy_step(
        null_one.0,
        ElfPolicyEvent::ProgramHeader {
            segment_type: 0,
            segment_flags: 0,
            file_offset: 0,
            virtual_address: 0,
            physical_address: 0,
            file_size: 0,
            memory_size: 0,
            alignment: 0,
        },
    );
    match null_two.1 {
        ElfPolicyAction::MetadataAccepted => {},
        _ => assert(false),
    }
    assert(null_two.0.seen_headers == 4);
    assert(null_two.0.load_segments == 2);
    assert(!null_two.0.entry_covered);
    let uncovered = elf_policy_step(null_two.0, ElfPolicyEvent::Finish);
    match uncovered.1 {
        ElfPolicyAction::Reject { code } => assert(code == 20),
        _ => assert(false),
    }

    assert(0x2000u64 <=
        0xffff_ffff_c000_0000u64 - 0xffff_ffff_8000_0000u64);
    let first = elf_policy_step(
        valid_elf_header(initial_elf_policy_state()).0,
        ElfPolicyEvent::ProgramHeader {
            segment_type: 1,
            segment_flags: 5,
            file_offset: 0x1000,
            virtual_address: 0xffff_ffff_8000_0000,
            physical_address: 0xffff_ffff_8000_0000,
            file_size: 0x1000,
            memory_size: 0x2000,
            alignment: 4096,
        },
    );
    match first.1 {
        ElfPolicyAction::LoadAccepted => {},
        _ => assert(false),
    }
    assert(first.0.last_virtual_end == 0xffff_ffff_8000_2000);
    let overlap = elf_policy_step(
        first.0,
        ElfPolicyEvent::ProgramHeader {
            segment_type: 1,
            segment_flags: 6,
            file_offset: 0x2000,
            virtual_address: 0xffff_ffff_8000_1000,
            physical_address: 0xffff_ffff_8000_1000,
            file_size: 0x1000,
            memory_size: 0x1000,
            alignment: 4096,
        },
    );
    match overlap.1 {
        ElfPolicyAction::Reject { code } => assert(code == 12),
        _ => assert(false),
    }

    let file_overflow = elf_policy_step(
        header.0,
        ElfPolicyEvent::ProgramHeader {
            segment_type: 1,
            segment_flags: 5,
            file_offset: 0x4000,
            virtual_address: 0xffff_ffff_8000_0000,
            physical_address: 0xffff_ffff_8000_0000,
            file_size: 0x2000,
            memory_size: 0x2000,
            alignment: 4096,
        },
    );
    match file_overflow.1 {
        ElfPolicyAction::Reject { code } => assert(code == 12),
        _ => assert(false),
    }

    127
}
