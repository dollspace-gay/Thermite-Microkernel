#![no_std]
#![crate_type = "rlib"]

use verus_builtin::*;
use verus_builtin_macros::*;

verus! {

pub const CAPSULE_VIRTUAL: u64 = 0xffff_ffff_8000_1010;
pub const FAR_TARGET_VIRTUAL: u64 = CAPSULE_VIRTUAL + 26;
pub const KERNEL_CODE_SELECTOR: u16 = 0x08;
pub const KERNEL_DATA_SELECTOR: u16 = 0x10;
pub const TSS_SELECTOR: u16 = 0x28;
pub const GDT_LIMIT: u16 = 55;
pub const IDT_LIMIT: u16 = 4095;

pub const REGISTERED_QWORD0: u64 = 0x0000_0010_b817_010f;
pub const REGISTERED_QWORD1: u64 = 0x086a_d08e_c08e_d88e;
pub const REGISTERED_QWORD2: u64 = 0x5000_0000_0305_8d48;
pub const REGISTERED_QWORD3: u64 = 0x0f00_0000_28b8_cb48;
pub const REGISTERED_DWORD4: u32 = 0x010f_d800;
pub const REGISTERED_WORD5: u16 = 0xc31e;

pub struct CapsuleImage {
    pub qword0: u64,
    pub qword1: u64,
    pub qword2: u64,
    pub qword3: u64,
    pub dword4: u32,
    pub word5: u16,
}

pub struct MachineState {
    pub rax: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rsp: u64,
    pub rip: u64,
    pub rflags: u64,
    pub cpl: u8,
    pub interrupts_enabled: bool,
    pub asynchronous_events_absent: bool,
    pub gdtr_pointer_readable: bool,
    pub idtr_pointer_readable: bool,
    pub far_stack_writable: bool,
    pub return_stack_readable: bool,
    pub gdt_image_registered: bool,
    pub idt_image_registered: bool,
    pub tss_descriptor_writable: bool,
    pub tss_descriptor_busy: bool,
    pub gdtr_operand_base: u64,
    pub gdtr_operand_limit: u16,
    pub idtr_operand_base: u64,
    pub idtr_operand_limit: u16,
    pub gdtr_base: u64,
    pub gdtr_limit: u16,
    pub idtr_base: u64,
    pub idtr_limit: u16,
    pub cs: u16,
    pub ds: u16,
    pub es: u16,
    pub ss: u16,
    pub tr: u16,
    pub return_address: u64,
}

pub struct CapsuleStep {
    pub accepted: bool,
    pub state: MachineState,
}

pub open spec fn canonical_address(address: u64) -> bool {
    address <= 0x0000_7fff_ffff_ffff
        || address >= 0xffff_8000_0000_0000
}

pub open spec fn registered_image_spec(image: &CapsuleImage) -> bool {
    image.qword0 == REGISTERED_QWORD0
        && image.qword1 == REGISTERED_QWORD1
        && image.qword2 == REGISTERED_QWORD2
        && image.qword3 == REGISTERED_QWORD3
        && image.dword4 == REGISTERED_DWORD4
        && image.word5 == REGISTERED_WORD5
}

pub open spec fn install_precondition(state: &MachineState) -> bool {
    state.cpl == 0
        && !state.interrupts_enabled
        && state.asynchronous_events_absent
        && state.gdtr_pointer_readable
        && state.idtr_pointer_readable
        && state.far_stack_writable
        && state.return_stack_readable
        && state.gdt_image_registered
        && state.idt_image_registered
        && state.tss_descriptor_writable
        && !state.tss_descriptor_busy
        && canonical_address(state.rdi)
        && canonical_address(state.rsi)
        && state.rdi <= 0xffff_ffff_ffff_fff6
        && state.rsi <= 0xffff_ffff_ffff_fff6
        && (state.rdi as int + 9 <= 0x0000_7fff_ffff_ffff
            || state.rdi as int + 9 >= 0xffff_8000_0000_0000)
        && (state.rsi as int + 9 <= 0x0000_7fff_ffff_ffff
            || state.rsi as int + 9 >= 0xffff_8000_0000_0000)
        && canonical_address(state.gdtr_operand_base)
        && canonical_address(state.idtr_operand_base)
        && state.gdtr_operand_limit == GDT_LIMIT
        && state.idtr_operand_limit == IDT_LIMIT
        && state.gdtr_operand_base <= 0xffff_ffff_ffff_ffc8
        && state.idtr_operand_base <= 0xffff_ffff_ffff_f000
        && (state.gdtr_operand_base as int + GDT_LIMIT as int <= 0x0000_7fff_ffff_ffff
            || state.gdtr_operand_base as int + GDT_LIMIT as int >= 0xffff_8000_0000_0000)
        && (state.idtr_operand_base as int + IDT_LIMIT as int <= 0x0000_7fff_ffff_ffff
            || state.idtr_operand_base as int + IDT_LIMIT as int >= 0xffff_8000_0000_0000)
        && state.rsp >= 16
        && state.rsp <= 0xffff_ffff_ffff_fff7
        && state.rsp & 7 == 0
        && canonical_address(state.rsp)
        && (state.rsp as int - 16 <= 0x0000_7fff_ffff_ffff
            || state.rsp as int - 16 >= 0xffff_8000_0000_0000)
        && (state.rsp as int + 7 <= 0x0000_7fff_ffff_ffff
            || state.rsp as int + 7 >= 0xffff_8000_0000_0000)
        && canonical_address(state.return_address)
        && canonical_address(FAR_TARGET_VIRTUAL)
}

pub fn registered_image() -> (result: CapsuleImage)
    ensures registered_image_spec(&result),
{
    CapsuleImage {
        qword0: REGISTERED_QWORD0,
        qword1: REGISTERED_QWORD1,
        qword2: REGISTERED_QWORD2,
        qword3: REGISTERED_QWORD3,
        dword4: REGISTERED_DWORD4,
        word5: REGISTERED_WORD5,
    }
}

pub fn decode_execute(image: CapsuleImage, state: MachineState) -> (result: CapsuleStep)
    ensures
        result.accepted <==> registered_image_spec(&image) && install_precondition(&state),
        result.accepted ==> result.state.rax == TSS_SELECTOR as u64,
        result.accepted ==> result.state.rdi == state.rdi,
        result.accepted ==> result.state.rsi == state.rsi,
        result.accepted ==> result.state.rsp == state.rsp + 8,
        result.accepted ==> result.state.rip == state.return_address,
        result.accepted ==> result.state.rflags == state.rflags,
        result.accepted ==> result.state.cpl == 0,
        result.accepted ==> !result.state.interrupts_enabled,
        result.accepted ==> result.state.gdtr_base == state.gdtr_operand_base,
        result.accepted ==> result.state.gdtr_limit == GDT_LIMIT,
        result.accepted ==> result.state.idtr_base == state.idtr_operand_base,
        result.accepted ==> result.state.idtr_limit == IDT_LIMIT,
        result.accepted ==> result.state.cs == KERNEL_CODE_SELECTOR,
        result.accepted ==> result.state.ds == KERNEL_DATA_SELECTOR,
        result.accepted ==> result.state.es == KERNEL_DATA_SELECTOR,
        result.accepted ==> result.state.ss == KERNEL_DATA_SELECTOR,
        result.accepted ==> result.state.tr == TSS_SELECTOR,
        result.accepted ==> result.state.tss_descriptor_busy,
        result.accepted ==> result.state.asynchronous_events_absent == state.asynchronous_events_absent,
        result.accepted ==> result.state.gdtr_pointer_readable == state.gdtr_pointer_readable,
        result.accepted ==> result.state.idtr_pointer_readable == state.idtr_pointer_readable,
        result.accepted ==> result.state.far_stack_writable == state.far_stack_writable,
        result.accepted ==> result.state.return_stack_readable == state.return_stack_readable,
        result.accepted ==> result.state.gdt_image_registered == state.gdt_image_registered,
        result.accepted ==> result.state.idt_image_registered == state.idt_image_registered,
        result.accepted ==> result.state.tss_descriptor_writable == state.tss_descriptor_writable,
        result.accepted ==> result.state.gdtr_operand_base == state.gdtr_operand_base,
        result.accepted ==> result.state.gdtr_operand_limit == state.gdtr_operand_limit,
        result.accepted ==> result.state.idtr_operand_base == state.idtr_operand_base,
        result.accepted ==> result.state.idtr_operand_limit == state.idtr_operand_limit,
        result.accepted ==> result.state.return_address == state.return_address,
        !result.accepted ==> result.state.rax == state.rax,
        !result.accepted ==> result.state.rdi == state.rdi,
        !result.accepted ==> result.state.rsi == state.rsi,
        !result.accepted ==> result.state.rsp == state.rsp,
        !result.accepted ==> result.state.rip == state.rip,
        !result.accepted ==> result.state.rflags == state.rflags,
        !result.accepted ==> result.state.cpl == state.cpl,
        !result.accepted ==> result.state.interrupts_enabled == state.interrupts_enabled,
        !result.accepted ==> result.state.gdtr_base == state.gdtr_base,
        !result.accepted ==> result.state.gdtr_limit == state.gdtr_limit,
        !result.accepted ==> result.state.idtr_base == state.idtr_base,
        !result.accepted ==> result.state.idtr_limit == state.idtr_limit,
        !result.accepted ==> result.state.cs == state.cs,
        !result.accepted ==> result.state.ds == state.ds,
        !result.accepted ==> result.state.es == state.es,
        !result.accepted ==> result.state.ss == state.ss,
        !result.accepted ==> result.state.tr == state.tr,
        !result.accepted ==> result.state.tss_descriptor_busy == state.tss_descriptor_busy,
{
    if image.qword0 == REGISTERED_QWORD0
        && image.qword1 == REGISTERED_QWORD1
        && image.qword2 == REGISTERED_QWORD2
        && image.qword3 == REGISTERED_QWORD3
        && image.dword4 == REGISTERED_DWORD4
        && image.word5 == REGISTERED_WORD5
        && state.cpl == 0
        && !state.interrupts_enabled
        && state.asynchronous_events_absent
        && state.gdtr_pointer_readable
        && state.idtr_pointer_readable
        && state.far_stack_writable
        && state.return_stack_readable
        && state.gdt_image_registered
        && state.idt_image_registered
        && state.tss_descriptor_writable
        && !state.tss_descriptor_busy
        && (state.rdi <= 0x0000_7fff_ffff_ffff || state.rdi >= 0xffff_8000_0000_0000)
        && (state.rsi <= 0x0000_7fff_ffff_ffff || state.rsi >= 0xffff_8000_0000_0000)
        && state.rdi <= 0xffff_ffff_ffff_fff6
        && state.rsi <= 0xffff_ffff_ffff_fff6
        && (state.rdi + 9 <= 0x0000_7fff_ffff_ffff
            || state.rdi + 9 >= 0xffff_8000_0000_0000)
        && (state.rsi + 9 <= 0x0000_7fff_ffff_ffff
            || state.rsi + 9 >= 0xffff_8000_0000_0000)
        && (state.gdtr_operand_base <= 0x0000_7fff_ffff_ffff
            || state.gdtr_operand_base >= 0xffff_8000_0000_0000)
        && (state.idtr_operand_base <= 0x0000_7fff_ffff_ffff
            || state.idtr_operand_base >= 0xffff_8000_0000_0000)
        && state.gdtr_operand_limit == GDT_LIMIT
        && state.idtr_operand_limit == IDT_LIMIT
        && state.gdtr_operand_base <= 0xffff_ffff_ffff_ffc8
        && state.idtr_operand_base <= 0xffff_ffff_ffff_f000
        && (state.gdtr_operand_base + GDT_LIMIT as u64 <= 0x0000_7fff_ffff_ffff
            || state.gdtr_operand_base + GDT_LIMIT as u64 >= 0xffff_8000_0000_0000)
        && (state.idtr_operand_base + IDT_LIMIT as u64 <= 0x0000_7fff_ffff_ffff
            || state.idtr_operand_base + IDT_LIMIT as u64 >= 0xffff_8000_0000_0000)
        && state.rsp >= 16
        && state.rsp <= 0xffff_ffff_ffff_fff7
        && state.rsp & 7 == 0
        && (state.rsp <= 0x0000_7fff_ffff_ffff || state.rsp >= 0xffff_8000_0000_0000)
        && (state.rsp - 16 <= 0x0000_7fff_ffff_ffff
            || state.rsp - 16 >= 0xffff_8000_0000_0000)
        && (state.rsp + 7 <= 0x0000_7fff_ffff_ffff
            || state.rsp + 7 >= 0xffff_8000_0000_0000)
        && (state.return_address <= 0x0000_7fff_ffff_ffff
            || state.return_address >= 0xffff_8000_0000_0000)
        && (FAR_TARGET_VIRTUAL <= 0x0000_7fff_ffff_ffff
            || FAR_TARGET_VIRTUAL >= 0xffff_8000_0000_0000)
    {
        CapsuleStep {
            accepted: true,
            state: MachineState {
                rax: TSS_SELECTOR as u64,
                rdi: state.rdi,
                rsi: state.rsi,
                rsp: state.rsp + 8,
                rip: state.return_address,
                rflags: state.rflags,
                cpl: state.cpl,
                interrupts_enabled: state.interrupts_enabled,
                asynchronous_events_absent: state.asynchronous_events_absent,
                gdtr_pointer_readable: state.gdtr_pointer_readable,
                idtr_pointer_readable: state.idtr_pointer_readable,
                far_stack_writable: state.far_stack_writable,
                return_stack_readable: state.return_stack_readable,
                gdt_image_registered: state.gdt_image_registered,
                idt_image_registered: state.idt_image_registered,
                tss_descriptor_writable: state.tss_descriptor_writable,
                tss_descriptor_busy: true,
                gdtr_operand_base: state.gdtr_operand_base,
                gdtr_operand_limit: state.gdtr_operand_limit,
                idtr_operand_base: state.idtr_operand_base,
                idtr_operand_limit: state.idtr_operand_limit,
                gdtr_base: state.gdtr_operand_base,
                gdtr_limit: state.gdtr_operand_limit,
                idtr_base: state.idtr_operand_base,
                idtr_limit: state.idtr_operand_limit,
                cs: KERNEL_CODE_SELECTOR,
                ds: KERNEL_DATA_SELECTOR,
                es: KERNEL_DATA_SELECTOR,
                ss: KERNEL_DATA_SELECTOR,
                tr: TSS_SELECTOR,
                return_address: state.return_address,
            },
        }
    } else {
        CapsuleStep { accepted: false, state }
    }
}

pub fn install_registered_tables(state: MachineState) -> (result: CapsuleStep)
    requires install_precondition(&state),
    ensures
        result.accepted,
        result.state.gdtr_base == state.gdtr_operand_base,
        result.state.gdtr_limit == GDT_LIMIT,
        result.state.idtr_base == state.idtr_operand_base,
        result.state.idtr_limit == IDT_LIMIT,
        result.state.cs == KERNEL_CODE_SELECTOR,
        result.state.ds == KERNEL_DATA_SELECTOR,
        result.state.es == KERNEL_DATA_SELECTOR,
        result.state.ss == KERNEL_DATA_SELECTOR,
        result.state.tr == TSS_SELECTOR,
        result.state.tss_descriptor_busy,
        result.state.rsp == state.rsp + 8,
        result.state.rip == state.return_address,
{
    decode_execute(registered_image(), state)
}

pub fn registered_install_observation() -> (result: u64)
    ensures result == 255,
{
    let state = MachineState {
        rax: 0x55aa,
        rdi: 0xffff_e000_0000_7fe0,
        rsi: 0xffff_e000_0000_7ff0,
        rsp: 0xffff_e000_0000_1000,
        rip: CAPSULE_VIRTUAL,
        rflags: 0x2,
        cpl: 0,
        interrupts_enabled: false,
        asynchronous_events_absent: true,
        gdtr_pointer_readable: true,
        idtr_pointer_readable: true,
        far_stack_writable: true,
        return_stack_readable: true,
        gdt_image_registered: true,
        idt_image_registered: true,
        tss_descriptor_writable: true,
        tss_descriptor_busy: false,
        gdtr_operand_base: 0xffff_e000_0000_8000,
        gdtr_operand_limit: GDT_LIMIT,
        idtr_operand_base: 0xffff_e000_0000_a000,
        idtr_operand_limit: IDT_LIMIT,
        gdtr_base: 0,
        gdtr_limit: 0,
        idtr_base: 0,
        idtr_limit: 0,
        cs: 0x38,
        ds: 0x30,
        es: 0x30,
        ss: 0x30,
        tr: 0,
        return_address: 0xffff_ffff_8000_2000,
    };
    assert(0xffff_e000_0000_1000u64 & 7 == 0) by(bit_vector);
    assert(canonical_address(state.rdi));
    assert(canonical_address(state.rsi));
    assert(state.rdi as int + 9 >= 0xffff_8000_0000_0000);
    assert(state.rsi as int + 9 >= 0xffff_8000_0000_0000);
    assert(canonical_address(state.gdtr_operand_base));
    assert(canonical_address(state.idtr_operand_base));
    assert(state.gdtr_operand_base as int + GDT_LIMIT as int >= 0xffff_8000_0000_0000);
    assert(state.idtr_operand_base as int + IDT_LIMIT as int >= 0xffff_8000_0000_0000);
    assert(canonical_address(state.rsp));
    assert(state.rsp as int - 16 >= 0xffff_8000_0000_0000);
    assert(state.rsp as int + 7 >= 0xffff_8000_0000_0000);
    assert(canonical_address(state.return_address));
    assert(canonical_address(FAR_TARGET_VIRTUAL));
    assert(install_precondition(&state));
    let step = install_registered_tables(state);
    assert(step.accepted);
    assert(step.state.gdtr_limit == GDT_LIMIT);
    assert(step.state.idtr_limit == IDT_LIMIT);
    assert(step.state.cs == KERNEL_CODE_SELECTOR);
    assert(step.state.ds == KERNEL_DATA_SELECTOR);
    assert(step.state.es == KERNEL_DATA_SELECTOR);
    assert(step.state.ss == KERNEL_DATA_SELECTOR);
    assert(step.state.tr == TSS_SELECTOR);
    assert(step.state.tss_descriptor_busy);
    assert((0u64 | 1u64) == 1u64
        && (1u64 | 2u64) == 3u64
        && (3u64 | 4u64) == 7u64
        && (7u64 | 8u64) == 15u64
        && (15u64 | 16u64) == 31u64
        && (31u64 | 32u64) == 63u64
        && (63u64 | 64u64) == 127u64
        && (127u64 | 128u64) == 255u64) by(bit_vector);
    let mut observation = 0u64;
    if step.state.gdtr_limit == GDT_LIMIT { observation = observation | 1; }
    assert(observation == 1);
    if step.state.idtr_limit == IDT_LIMIT { observation = observation | 2; }
    assert(observation == 3);
    if step.state.cs == KERNEL_CODE_SELECTOR { observation = observation | 4; }
    assert(observation == 7);
    if step.state.ds == KERNEL_DATA_SELECTOR { observation = observation | 8; }
    assert(observation == 15);
    if step.state.es == KERNEL_DATA_SELECTOR { observation = observation | 16; }
    assert(observation == 31);
    if step.state.ss == KERNEL_DATA_SELECTOR { observation = observation | 32; }
    assert(observation == 63);
    if step.state.tr == TSS_SELECTOR { observation = observation | 64; }
    assert(observation == 127);
    if step.state.tss_descriptor_busy { observation = observation | 128; }
    assert(observation == 255);
    observation
}

}
