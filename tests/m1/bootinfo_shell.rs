pub struct BootInfoValidation {
    pub code: u32,
    pub range_count: u64,
    pub last_end: u64,
    pub bsp_apic_id: u32,
}

fn failed(code: u32) -> (result: BootInfoValidation)
    requires code != 0,
    ensures
        result.code == code,
        result.range_count == 0,
        result.last_end == 0,
        result.bsp_apic_id == 0,
{
    BootInfoValidation {
        code,
        range_count: 0,
        last_end: 0,
        bsp_apic_id: 0,
    }
}

fn read_u32(bytes: &[u8], offset: usize) -> (result: u32)
    requires
        offset <= bytes.len(),
        4 <= bytes.len() - offset,
    ensures
        result == (bytes[offset] as u32)
            | ((bytes[offset + 1] as u32) << 8)
            | ((bytes[offset + 2] as u32) << 16)
            | ((bytes[offset + 3] as u32) << 24),
{
    (bytes[offset] as u32)
        | ((bytes[offset + 1] as u32) << 8)
        | ((bytes[offset + 2] as u32) << 16)
        | ((bytes[offset + 3] as u32) << 24)
}

fn read_u64(bytes: &[u8], offset: usize) -> (result: u64)
    requires
        offset <= bytes.len(),
        8 <= bytes.len() - offset,
    ensures
        result == (bytes[offset] as u64)
            | ((bytes[offset + 1] as u64) << 8)
            | ((bytes[offset + 2] as u64) << 16)
            | ((bytes[offset + 3] as u64) << 24)
            | ((bytes[offset + 4] as u64) << 32)
            | ((bytes[offset + 5] as u64) << 40)
            | ((bytes[offset + 6] as u64) << 48)
            | ((bytes[offset + 7] as u64) << 56),
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

pub fn validate_bootinfo(bytes: &[u8]) -> (result: BootInfoValidation)
    ensures
        result.code == 0 ==> result.range_count <= 256,
        result.code == 0 ==> result.last_end <= 0x0010_0000_0000_0000,
{
    if bytes.len() < 256 {
        return failed(100);
    }

    let magic = read_u64(bytes, 0);
    let abi_major = read_u32(bytes, 8);
    let abi_minor = read_u32(bytes, 12);
    let header_length = read_u32(bytes, 16) as u64;
    let total_length = read_u32(bytes, 20) as u64;
    let flags = read_u64(bytes, 32);
    let map_offset = read_u32(bytes, 40) as u64;
    let map_entry_size = read_u32(bytes, 44) as u64;
    let map_count = read_u32(bytes, 48) as u64;
    let header_reserved = read_u32(bytes, 52);
    let kernel_phys_start = read_u64(bytes, 56);
    let kernel_phys_end = read_u64(bytes, 64);
    let kernel_virt_start = read_u64(bytes, 72);
    let kernel_virt_end = read_u64(bytes, 80);
    let service_start = read_u64(bytes, 88);
    let service_length = read_u64(bytes, 96);
    let config_start = read_u64(bytes, 136);
    let config_length = read_u64(bytes, 144);
    let rsdp = read_u64(bytes, 184);
    let command_offset = read_u32(bytes, 192) as u64;
    let command_length = read_u32(bytes, 196) as u64;
    let seed_offset = read_u32(bytes, 200) as u64;
    let seed_length = read_u32(bytes, 204) as u64;
    let bsp_apic_id = read_u32(bytes, 208);
    let framebuffer_format = read_u32(bytes, 212);
    let framebuffer_base = read_u64(bytes, 216);
    let framebuffer_size = read_u64(bytes, 224);
    let framebuffer_width = read_u32(bytes, 232) as u64;
    let framebuffer_height = read_u32(bytes, 236) as u64;
    let framebuffer_stride = read_u32(bytes, 240) as u64;
    let framebuffer_reserved = read_u32(bytes, 244);
    let tail_reserved = read_u64(bytes, 248);

    let checksum =
        read_u64(bytes, 0)
        ^ read_u64(bytes, 8)
        ^ read_u64(bytes, 16)
        ^ read_u64(bytes, 24)
        ^ read_u64(bytes, 32)
        ^ read_u64(bytes, 40)
        ^ read_u64(bytes, 48)
        ^ read_u64(bytes, 56)
        ^ read_u64(bytes, 64)
        ^ read_u64(bytes, 72)
        ^ read_u64(bytes, 80)
        ^ read_u64(bytes, 88)
        ^ read_u64(bytes, 96)
        ^ read_u64(bytes, 104)
        ^ read_u64(bytes, 112)
        ^ read_u64(bytes, 120)
        ^ read_u64(bytes, 128)
        ^ read_u64(bytes, 136)
        ^ read_u64(bytes, 144)
        ^ read_u64(bytes, 152)
        ^ read_u64(bytes, 160)
        ^ read_u64(bytes, 168)
        ^ read_u64(bytes, 176)
        ^ read_u64(bytes, 184)
        ^ read_u64(bytes, 192)
        ^ read_u64(bytes, 200)
        ^ read_u64(bytes, 208)
        ^ read_u64(bytes, 216)
        ^ read_u64(bytes, 224)
        ^ read_u64(bytes, 232)
        ^ read_u64(bytes, 240)
        ^ read_u64(bytes, 248);
    let service_digest_present =
        bytes[104] != 0 || bytes[105] != 0 || bytes[106] != 0 || bytes[107] != 0 ||
        bytes[108] != 0 || bytes[109] != 0 || bytes[110] != 0 || bytes[111] != 0 ||
        bytes[112] != 0 || bytes[113] != 0 || bytes[114] != 0 || bytes[115] != 0 ||
        bytes[116] != 0 || bytes[117] != 0 || bytes[118] != 0 || bytes[119] != 0 ||
        bytes[120] != 0 || bytes[121] != 0 || bytes[122] != 0 || bytes[123] != 0 ||
        bytes[124] != 0 || bytes[125] != 0 || bytes[126] != 0 || bytes[127] != 0 ||
        bytes[128] != 0 || bytes[129] != 0 || bytes[130] != 0 || bytes[131] != 0 ||
        bytes[132] != 0 || bytes[133] != 0 || bytes[134] != 0 || bytes[135] != 0;
    let config_digest_present =
        bytes[152] != 0 || bytes[153] != 0 || bytes[154] != 0 || bytes[155] != 0 ||
        bytes[156] != 0 || bytes[157] != 0 || bytes[158] != 0 || bytes[159] != 0 ||
        bytes[160] != 0 || bytes[161] != 0 || bytes[162] != 0 || bytes[163] != 0 ||
        bytes[164] != 0 || bytes[165] != 0 || bytes[166] != 0 || bytes[167] != 0 ||
        bytes[168] != 0 || bytes[169] != 0 || bytes[170] != 0 || bytes[171] != 0 ||
        bytes[172] != 0 || bytes[173] != 0 || bytes[174] != 0 || bytes[175] != 0 ||
        bytes[176] != 0 || bytes[177] != 0 || bytes[178] != 0 || bytes[179] != 0 ||
        bytes[180] != 0 || bytes[181] != 0 || bytes[182] != 0 || bytes[183] != 0;
    let framebuffer_valid = if flags & 4 == 0 {
        framebuffer_format == 0 && framebuffer_base == 0 && framebuffer_size == 0 &&
        framebuffer_width == 0 && framebuffer_height == 0 && framebuffer_stride == 0
    } else {
        framebuffer_format <= 2 && framebuffer_base != 0 && framebuffer_size != 0 &&
        framebuffer_base <= u64::MAX - framebuffer_size &&
        framebuffer_width != 0 && framebuffer_height != 0 &&
        framebuffer_stride >= framebuffer_width
    };
    let checksum_valid = checksum == 0 &&
        total_length as usize == bytes.len() &&
        header_reserved == 0 && framebuffer_reserved == 0 && tail_reserved == 0;

    let initial = BootPolicyState {
        phase: 0,
        expected_ranges: 0,
        accepted_ranges: 0,
        last_end: 0,
    };
    let header = boot_policy_step(
        initial,
        BootPolicyEvent::Header {
            magic,
            abi_major,
            abi_minor,
            header_length,
            total_length,
            flags,
            map_offset,
            map_entry_size,
            map_count,
            command_offset,
            command_length,
            seed_offset,
            seed_length,
            kernel_phys_start,
            kernel_phys_end,
            kernel_virt_start,
            kernel_virt_end,
            service_start,
            service_length,
            config_start,
            config_length,
            rsdp,
            checksum_valid,
            service_digest_present,
            config_digest_present,
            framebuffer_valid,
        },
    );
    let mut policy = match header.1 {
        BootPolicyAction::HeaderAccepted => header.0,
        BootPolicyAction::Reject { code } => return failed(code),
        BootPolicyAction::RangeAccepted => return failed(101),
        BootPolicyAction::Complete => return failed(102),
    };

    let mut index: u64 = 0;
    while index < map_count
        invariant
            policy.well_formed(),
            policy.phase == 1,
            policy.expected_ranges == map_count,
            policy.accepted_ranges == index,
            index <= map_count,
            map_count <= 256,
        decreases map_count - index,
    {
        let offset = 256usize + index as usize * 32usize;
        let start = read_u64(bytes, offset);
        let end = read_u64(bytes, offset + 8);
        let kind = read_u32(bytes, offset + 16);
        let reserved = read_u32(bytes, offset + 20);
        let next = boot_policy_step(
            policy,
            BootPolicyEvent::Range {
                start,
                end,
                kind,
                reserved_zero: reserved == 0,
            },
        );
        policy = match next.1 {
            BootPolicyAction::RangeAccepted => next.0,
            BootPolicyAction::Reject { code } => return failed(code),
            BootPolicyAction::HeaderAccepted => return failed(103),
            BootPolicyAction::Complete => return failed(104),
        };
        index = index + 1;
    }

    let finish = boot_policy_step(policy, BootPolicyEvent::Finish);
    match finish.1 {
        BootPolicyAction::Complete => BootInfoValidation {
            code: 0,
            range_count: finish.0.accepted_ranges,
            last_end: finish.0.last_end,
            bsp_apic_id,
        },
        BootPolicyAction::Reject { code } => failed(code),
        BootPolicyAction::HeaderAccepted => failed(105),
        BootPolicyAction::RangeAccepted => failed(106),
    }
}
