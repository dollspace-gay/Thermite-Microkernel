extern crate tmk_descriptor_tables;

use tmk_descriptor_tables::{
    DescriptorTablePointer, GDT_ENTRIES, Gdt, HANDLER_BASE, HANDLER_STRIDE, IDT_ENTRIES, Idt,
    KERNEL_CODE_DESCRIPTOR, KERNEL_CODE_SELECTOR, KERNEL_DATA_DESCRIPTOR, TSS_SELECTOR, Tss64,
    USER_CODE_DESCRIPTOR, USER_CODE_SELECTOR, USER_DATA_DESCRIPTOR, USER_DATA_SELECTOR,
    descriptor_table_observation, descriptor_table_pointer, idt_attributes, idt_ist, idt_offset,
    idt_reserved_zero, idt_selector, registered_gdt, registered_idt, registered_tss,
    tss_descriptor_base,
};

fn main() {
    assert_eq!(std::mem::size_of::<Tss64>(), 104);
    assert_eq!(std::mem::size_of::<Gdt>(), GDT_ENTRIES * 8);
    assert_eq!(std::mem::align_of::<Gdt>(), 8);
    assert_eq!(std::mem::size_of::<Idt>(), IDT_ENTRIES * 16);
    assert_eq!(std::mem::align_of::<Idt>(), 16);
    assert_eq!(std::mem::size_of::<DescriptorTablePointer>(), 10);

    let tss = Box::new(registered_tss());
    let tss_base = (&*tss as *const Tss64) as u64;
    assert!(tss_base <= 0x0000_7fff_ffff_ffff || tss_base >= 0xffff_8000_0000_0000);
    let gdt = registered_gdt(tss_base);
    let idt = registered_idt();

    assert_eq!(gdt.entries[0], 0);
    assert_eq!(gdt.entries[1], KERNEL_CODE_DESCRIPTOR);
    assert_eq!(gdt.entries[2], KERNEL_DATA_DESCRIPTOR);
    assert_eq!(gdt.entries[3], USER_DATA_DESCRIPTOR);
    assert_eq!(gdt.entries[4], USER_CODE_DESCRIPTOR);
    let tss_descriptor = tmk_descriptor_tables::TssDescriptor {
        low: gdt.entries[5],
        high: gdt.entries[6],
    };
    assert_eq!(tss_descriptor_base(&tss_descriptor), tss_base);

    assert_eq!(KERNEL_CODE_SELECTOR, 0x08);
    assert_eq!(tmk_descriptor_tables::KERNEL_DATA_SELECTOR, 0x10);
    assert_eq!(USER_DATA_SELECTOR, 0x1b);
    assert_eq!(USER_CODE_SELECTOR, 0x23);
    assert_eq!(TSS_SELECTOR, 0x28);

    let rsp0 = tss.rsp0;
    let ist1 = tss.ist1;
    let ist2 = tss.ist2;
    let ist3 = tss.ist3;
    let iomap_base = tss.iomap_base;
    assert_eq!(rsp0, tmk_descriptor_tables::KERNEL_RSP0);
    assert_eq!(ist1, tmk_descriptor_tables::DOUBLE_FAULT_IST_TOP);
    assert_eq!(ist2, tmk_descriptor_tables::NMI_IST_TOP);
    assert_eq!(ist3, tmk_descriptor_tables::MACHINE_CHECK_IST_TOP);
    assert_eq!(iomap_base, 104);

    let mut present = 0usize;
    let mut user_callable = 0usize;
    let mut ist_entries = 0usize;
    for (vector, gate) in idt.entries.iter().enumerate() {
        assert_eq!(idt_offset(gate), HANDLER_BASE + vector as u64 * HANDLER_STRIDE);
        assert_eq!(idt_selector(gate), KERNEL_CODE_SELECTOR as u64);
        assert!(idt_reserved_zero(gate));
        let attributes = idt_attributes(gate);
        assert_eq!(attributes & 0x80, 0x80);
        assert_eq!(attributes & 0x0f, 0x0e);
        present += 1;
        if attributes & 0x60 == 0x60 {
            user_callable += 1;
            assert_eq!(vector, 3);
        }
        let expected_ist = match vector {
            8 => 1,
            2 => 2,
            18 => 3,
            _ => 0,
        };
        assert_eq!(idt_ist(gate), expected_ist);
        if expected_ist != 0 {
            ist_entries += 1;
        }
    }
    assert_eq!(present, 256);
    assert_eq!(user_callable, 1);
    assert_eq!(ist_entries, 3);

    let gdtr = descriptor_table_pointer((&gdt as *const Gdt) as u64, 55);
    let idtr = descriptor_table_pointer((&idt as *const Idt) as u64, 4095);
    let gdtr_limit = gdtr.limit;
    let gdtr_base = gdtr.base;
    let idtr_limit = idtr.limit;
    let idtr_base = idtr.base;
    assert_eq!(gdtr_limit, 55);
    assert_eq!(gdtr_base, (&gdt as *const Gdt) as u64);
    assert_eq!(idtr_limit, 4095);
    assert_eq!(idtr_base, (&idt as *const Idt) as u64);

    let observation = descriptor_table_observation();
    assert_eq!(observation, 255);
    println!(
        "M1_DESCRIPTOR_TABLES_OK observation={observation} gdt=7 idt={present} ist={ist_entries} dpl3={user_callable} tss=104"
    );
}
