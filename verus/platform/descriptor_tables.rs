#![no_std]
#![crate_type = "rlib"]

use verus_builtin::*;
use verus_builtin_macros::*;
use vstd::array::ArrayAdditionalSpecFns;

verus! {

pub const GDT_ENTRIES: usize = 7;
pub const IDT_ENTRIES: usize = 256;
pub const KERNEL_CODE_SELECTOR: u16 = 0x08;
pub const KERNEL_DATA_SELECTOR: u16 = 0x10;
pub const USER_DATA_SELECTOR: u16 = 0x1b;
pub const USER_CODE_SELECTOR: u16 = 0x23;
pub const TSS_SELECTOR: u16 = 0x28;

pub const KERNEL_CODE_DESCRIPTOR: u64 = 0x00af_9a00_0000_ffff;
pub const KERNEL_DATA_DESCRIPTOR: u64 = 0x00cf_9200_0000_ffff;
pub const USER_DATA_DESCRIPTOR: u64 = 0x00cf_f200_0000_ffff;
pub const USER_CODE_DESCRIPTOR: u64 = 0x00af_fa00_0000_ffff;

pub const HANDLER_BASE: u64 = 0xffff_ffff_8001_0000;
pub const HANDLER_STRIDE: u64 = 16;
pub const KERNEL_RSP0: u64 = 0xffff_e000_0000_1000;
pub const DOUBLE_FAULT_IST_TOP: u64 = 0xffff_e000_0000_3000;
pub const NMI_IST_TOP: u64 = 0xffff_e000_0000_5000;
pub const MACHINE_CHECK_IST_TOP: u64 = 0xffff_e000_0000_7000;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct IdtGate {
    pub low: u64,
    pub high: u64,
}

#[repr(C, align(16))]
pub struct Idt {
    pub entries: [IdtGate; IDT_ENTRIES],
}

#[repr(C, align(8))]
pub struct Gdt {
    pub entries: [u64; GDT_ENTRIES],
}

#[repr(C, packed)]
pub struct Tss64 {
    pub reserved0: u32,
    pub rsp0: u64,
    pub rsp1: u64,
    pub rsp2: u64,
    pub reserved1: u64,
    pub ist1: u64,
    pub ist2: u64,
    pub ist3: u64,
    pub ist4: u64,
    pub ist5: u64,
    pub ist6: u64,
    pub ist7: u64,
    pub reserved2: u64,
    pub reserved3: u16,
    pub iomap_base: u16,
}

#[repr(C, packed)]
pub struct DescriptorTablePointer {
    pub limit: u16,
    pub base: u64,
}

pub struct TssDescriptor {
    pub low: u64,
    pub high: u64,
}

pub open spec fn canonical_address(address: u64) -> bool {
    address <= 0x0000_7fff_ffff_ffff
        || address >= 0xffff_8000_0000_0000
}

pub open spec fn spec_idt_offset(gate: &IdtGate) -> u64 {
    (gate.low & 0xffff)
        | (((gate.low >> 48) & 0xffff) << 16)
        | ((gate.high & 0xffff_ffff) << 32)
}

pub open spec fn spec_idt_selector(gate: &IdtGate) -> u64 {
    (gate.low >> 16) & 0xffff
}

pub open spec fn spec_idt_ist(gate: &IdtGate) -> u64 {
    (gate.low >> 32) & 7
}

pub open spec fn spec_idt_attributes(gate: &IdtGate) -> u64 {
    (gate.low >> 40) & 0xff
}

pub open spec fn spec_idt_reserved_zero(gate: &IdtGate) -> bool {
    ((gate.low >> 35) & 0x1f) == 0 && (gate.high >> 32) == 0
}

pub fn idt_offset(gate: &IdtGate) -> (result: u64)
    ensures result == spec_idt_offset(gate),
{
    (gate.low & 0xffff)
        | (((gate.low >> 48) & 0xffff) << 16)
        | ((gate.high & 0xffff_ffff) << 32)
}

pub fn idt_selector(gate: &IdtGate) -> (result: u64)
    ensures result == spec_idt_selector(gate),
{
    (gate.low >> 16) & 0xffff
}

pub fn idt_ist(gate: &IdtGate) -> (result: u64)
    ensures result == spec_idt_ist(gate),
{
    (gate.low >> 32) & 7
}

pub fn idt_attributes(gate: &IdtGate) -> (result: u64)
    ensures result == spec_idt_attributes(gate),
{
    (gate.low >> 40) & 0xff
}

pub fn idt_reserved_zero(gate: &IdtGate) -> (result: bool)
    ensures result == spec_idt_reserved_zero(gate),
{
    ((gate.low >> 35) & 0x1f) == 0 && (gate.high >> 32) == 0
}

pub open spec fn expected_handler(vector: u16) -> int {
    HANDLER_BASE as int + vector as int * HANDLER_STRIDE as int
}

pub open spec fn expected_ist(vector: u16) -> u64 {
    if vector == 8 { 1 }
    else if vector == 2 { 2 }
    else if vector == 18 { 3 }
    else { 0 }
}

pub open spec fn expected_attributes(vector: u16) -> u64 {
    if vector == 3 { 0xee } else { 0x8e }
}

pub open spec fn registered_gate(gate: &IdtGate, vector: u16) -> bool {
    vector < 256
        && spec_idt_offset(gate) as int == expected_handler(vector)
        && spec_idt_selector(gate) == KERNEL_CODE_SELECTOR as u64
        && spec_idt_ist(gate) == expected_ist(vector)
        && spec_idt_attributes(gate) == expected_attributes(vector)
        && spec_idt_reserved_zero(gate)
}

pub open spec fn idt_well_formed(idt: &Idt) -> bool {
    forall|index: int| 0 <= index < IDT_ENTRIES
        ==> registered_gate(&idt.entries[index], index as u16)
}

pub open spec fn spec_tss_descriptor_base(descriptor: &TssDescriptor) -> u64 {
    ((descriptor.low >> 16) & 0xffff)
        | (((descriptor.low >> 32) & 0xff) << 16)
        | (((descriptor.low >> 56) & 0xff) << 24)
        | ((descriptor.high & 0xffff_ffff) << 32)
}

pub fn tss_descriptor_base(descriptor: &TssDescriptor) -> (result: u64)
    ensures result == spec_tss_descriptor_base(descriptor),
{
    ((descriptor.low >> 16) & 0xffff)
        | (((descriptor.low >> 32) & 0xff) << 16)
        | (((descriptor.low >> 56) & 0xff) << 24)
        | ((descriptor.high & 0xffff_ffff) << 32)
}

pub open spec fn registered_tss_descriptor(
    descriptor: &TssDescriptor,
    base: u64,
) -> bool {
    spec_tss_descriptor_base(descriptor) == base
        && descriptor.low & 0xffff == 103
        && (descriptor.low >> 40) & 0xff == 0x89
        && (descriptor.low >> 48) & 0xff == 0
        && descriptor.high >> 32 == 0
}

pub open spec fn gdt_well_formed(gdt: &Gdt, tss_base: u64) -> bool {
    gdt.entries[0] == 0
        && gdt.entries[1] == KERNEL_CODE_DESCRIPTOR
        && gdt.entries[2] == KERNEL_DATA_DESCRIPTOR
        && gdt.entries[3] == USER_DATA_DESCRIPTOR
        && gdt.entries[4] == USER_CODE_DESCRIPTOR
        && registered_tss_descriptor(
            &TssDescriptor { low: gdt.entries[5], high: gdt.entries[6] },
            tss_base,
        )
}

pub open spec fn encoded_idt_low(handler: u64, attributes: u64, ist: u64) -> u64 {
    (handler & 0xffff)
        | ((KERNEL_CODE_SELECTOR as u64) << 16)
        | (ist << 32)
        | (attributes << 40)
        | (((handler >> 16) & 0xffff) << 48)
}

pub open spec fn encoded_idt_high(handler: u64) -> u64 {
    (handler >> 32) & 0xffff_ffff
}

#[verifier::bit_vector]
proof fn lemma_idt_encoding(handler: u64, attributes: u64, ist: u64)
    requires ist <= 7, attributes <= 255,
    ensures
        (encoded_idt_low(handler, attributes, ist) & 0xffff)
            | (((encoded_idt_low(handler, attributes, ist) >> 48) & 0xffff) << 16)
            | ((encoded_idt_high(handler) & 0xffff_ffff) << 32) == handler,
        (encoded_idt_low(handler, attributes, ist) >> 16) & 0xffff
            == KERNEL_CODE_SELECTOR as u64,
        (encoded_idt_low(handler, attributes, ist) >> 32) & 7 == ist,
        (encoded_idt_low(handler, attributes, ist) >> 40) & 0xff == attributes,
        (encoded_idt_low(handler, attributes, ist) >> 35) & 0x1f == 0,
        encoded_idt_high(handler) >> 32 == 0,
{}

pub open spec fn encoded_tss_low(base: u64) -> u64 {
    103u64
        | ((base & 0xffff) << 16)
        | (((base >> 16) & 0xff) << 32)
        | (0x89u64 << 40)
        | (((base >> 24) & 0xff) << 56)
}

pub open spec fn encoded_tss_high(base: u64) -> u64 {
    (base >> 32) & 0xffff_ffff
}

#[verifier::bit_vector]
proof fn lemma_tss_encoding(base: u64)
    ensures
        ((encoded_tss_low(base) >> 16) & 0xffff)
            | (((encoded_tss_low(base) >> 32) & 0xff) << 16)
            | (((encoded_tss_low(base) >> 56) & 0xff) << 24)
            | ((encoded_tss_high(base) & 0xffff_ffff) << 32) == base,
        encoded_tss_low(base) & 0xffff == 103,
        (encoded_tss_low(base) >> 40) & 0xff == 0x89,
        (encoded_tss_low(base) >> 48) & 0xff == 0,
        encoded_tss_high(base) >> 32 == 0,
{}

pub fn make_idt_gate(handler: u64, attributes: u64, ist: u64) -> (result: IdtGate)
    requires ist <= 7, attributes <= 255,
    ensures
        spec_idt_offset(&result) == handler,
        spec_idt_selector(&result) == KERNEL_CODE_SELECTOR,
        spec_idt_ist(&result) == ist,
        spec_idt_attributes(&result) == attributes,
        spec_idt_reserved_zero(&result),
{
    let low = (handler & 0xffff)
        | ((KERNEL_CODE_SELECTOR as u64) << 16)
        | (ist << 32)
        | (attributes << 40)
        | (((handler >> 16) & 0xffff) << 48);
    let high = (handler >> 32) & 0xffff_ffff;
    assert(low == encoded_idt_low(handler, attributes, ist));
    assert(high == encoded_idt_high(handler));
    proof { lemma_idt_encoding(handler, attributes, ist); }
    IdtGate { low, high }
}

pub fn registered_idt_gate(vector: u16) -> (result: IdtGate)
    requires vector < 256,
    ensures registered_gate(&result, vector),
{
    let handler = HANDLER_BASE + (vector as u64) * HANDLER_STRIDE;
    let ist: u64 = if vector == 8 { 1 }
        else if vector == 2 { 2 }
        else if vector == 18 { 3 }
        else { 0 };
    let attributes: u64 = if vector == 3 { 0xee } else { 0x8e };
    assert(ist <= 7);
    assert(attributes <= 255);
    assert(handler as int == expected_handler(vector));
    assert(ist == expected_ist(vector));
    assert(attributes == expected_attributes(vector));
    make_idt_gate(handler, attributes, ist)
}

pub fn registered_idt() -> (result: Idt)
    ensures idt_well_formed(&result),
{
    let zero = IdtGate { low: 0, high: 0 };
    let mut entries = [zero; IDT_ENTRIES];
    let mut slot: usize = 0;
    while slot < IDT_ENTRIES
        invariant
            0 <= slot <= IDT_ENTRIES,
            forall|index: int| 0 <= index < slot
                ==> registered_gate(&entries[index], index as u16),
        decreases IDT_ENTRIES - slot,
    {
        let gate = registered_idt_gate(slot as u16);
        entries[slot] = gate;
        slot = slot + 1;
    }
    Idt { entries }
}

pub fn make_tss_descriptor(base: u64) -> (result: TssDescriptor)
    ensures registered_tss_descriptor(&result, base),
{
    let low = 103u64
        | ((base & 0xffff) << 16)
        | (((base >> 16) & 0xff) << 32)
        | (0x89u64 << 40)
        | (((base >> 24) & 0xff) << 56);
    let high = (base >> 32) & 0xffff_ffff;
    assert(low == encoded_tss_low(base));
    assert(high == encoded_tss_high(base));
    proof { lemma_tss_encoding(base); }
    TssDescriptor { low, high }
}

pub fn registered_gdt(tss_base: u64) -> (result: Gdt)
    requires canonical_address(tss_base),
    ensures gdt_well_formed(&result, tss_base),
{
    let tss = make_tss_descriptor(tss_base);
    Gdt {
        entries: [
            0,
            KERNEL_CODE_DESCRIPTOR,
            KERNEL_DATA_DESCRIPTOR,
            USER_DATA_DESCRIPTOR,
            USER_CODE_DESCRIPTOR,
            tss.low,
            tss.high,
        ],
    }
}

pub fn registered_tss() -> (result: Tss64)
    ensures
        result.reserved0 == 0,
        result.rsp0 == KERNEL_RSP0,
        result.rsp1 == 0,
        result.rsp2 == 0,
        result.reserved1 == 0,
        result.ist1 == DOUBLE_FAULT_IST_TOP,
        result.ist2 == NMI_IST_TOP,
        result.ist3 == MACHINE_CHECK_IST_TOP,
        result.ist4 == 0,
        result.ist5 == 0,
        result.ist6 == 0,
        result.ist7 == 0,
        result.reserved2 == 0,
        result.reserved3 == 0,
        result.iomap_base == 104,
{
    Tss64 {
        reserved0: 0,
        rsp0: KERNEL_RSP0,
        rsp1: 0,
        rsp2: 0,
        reserved1: 0,
        ist1: DOUBLE_FAULT_IST_TOP,
        ist2: NMI_IST_TOP,
        ist3: MACHINE_CHECK_IST_TOP,
        ist4: 0,
        ist5: 0,
        ist6: 0,
        ist7: 0,
        reserved2: 0,
        reserved3: 0,
        iomap_base: 104,
    }
}

pub fn descriptor_table_pointer(base: u64, limit: u16) -> (result: DescriptorTablePointer)
    ensures result.base == base, result.limit == limit,
{
    DescriptorTablePointer { limit, base }
}

pub fn descriptor_table_observation() -> (result: u64)
    ensures result == 255,
{
    let idt = registered_idt();
    let gdt = registered_gdt(0xffff_e000_0000_8000);
    let tss = registered_tss();
    let nmi = idt.entries[2];
    let breakpoint = idt.entries[3];
    let double_fault = idt.entries[8];
    let machine_check = idt.entries[18];
    let timer = idt.entries[0xe0];
    assert(registered_gate(&nmi, 2));
    assert(registered_gate(&breakpoint, 3));
    assert(registered_gate(&double_fault, 8));
    assert(registered_gate(&machine_check, 18));
    assert(registered_gate(&timer, 0xe0));
    assert(gdt_well_formed(&gdt, 0xffff_e000_0000_8000));
    assert(tss.rsp0 == KERNEL_RSP0);
    assert(tss.ist1 == DOUBLE_FAULT_IST_TOP);
    assert(tss.ist2 == NMI_IST_TOP);
    assert(tss.ist3 == MACHINE_CHECK_IST_TOP);
    assert(tss.iomap_base == 104);
    let nmi_ist = idt_ist(&nmi);
    let breakpoint_attributes = idt_attributes(&breakpoint);
    let double_fault_ist = idt_ist(&double_fault);
    let machine_check_ist = idt_ist(&machine_check);
    let timer_attributes = idt_attributes(&timer);
    assert(nmi_ist == 2);
    assert(breakpoint_attributes == 0xee);
    assert(double_fault_ist == 1);
    assert(machine_check_ist == 3);
    assert(timer_attributes == 0x8e);
    assert((0u64 | 1u64) == 1u64
        && (1u64 | 2u64) == 3u64
        && (3u64 | 4u64) == 7u64
        && (7u64 | 8u64) == 15u64
        && (15u64 | 16u64) == 31u64
        && (31u64 | 32u64) == 63u64
        && (63u64 | 64u64) == 127u64
        && (127u64 | 128u64) == 255u64) by(bit_vector);
    let mut observation = 0u64;
    if nmi_ist == 2 { observation = observation | 1; }
    assert(observation == 1);
    if breakpoint_attributes == 0xee { observation = observation | 2; }
    assert(observation == 3);
    if double_fault_ist == 1 { observation = observation | 4; }
    assert(observation == 7);
    if machine_check_ist == 3 { observation = observation | 8; }
    assert(observation == 15);
    if timer_attributes == 0x8e { observation = observation | 16; }
    assert(observation == 31);
    if gdt.entries[1] == KERNEL_CODE_DESCRIPTOR { observation = observation | 32; }
    assert(observation == 63);
    if gdt.entries[4] == USER_CODE_DESCRIPTOR { observation = observation | 64; }
    assert(observation == 127);
    if tss.iomap_base == 104 { observation = observation | 128; }
    assert(observation == 255);
    observation
}

}
