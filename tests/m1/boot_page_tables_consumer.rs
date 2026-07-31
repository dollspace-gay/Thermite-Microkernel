extern crate tmk_boot_page_tables;

use tmk_boot_page_tables::{
    BootPageTables, DIRECT_VIRTUAL, HEAP_VIRTUAL, IMAGE_VIRTUAL, NO_EXECUTE, PAGE_SIZE, PRESENT,
    ROOT_PHYSICAL, STACK_VIRTUAL, WRITABLE, boot_page_table_observation,
    registered_boot_page_tables, walk_boot_page_tables,
};

fn main() {
    let tables = registered_boot_page_tables();
    assert_eq!(ROOT_PHYSICAL, 0x0040_0000);
    assert_eq!(std::mem::size_of::<BootPageTables>(), 13 * 4096);
    assert_eq!(std::mem::align_of::<BootPageTables>(), 4096);

    let pages = [
        &tables.pml4,
        &tables.direct_pdpt,
        &tables.direct_pd,
        &tables.direct_pt,
        &tables.heap_pdpt,
        &tables.heap_pd,
        &tables.heap_pt,
        &tables.stack_pdpt,
        &tables.stack_pd,
        &tables.stack_pt,
        &tables.image_pdpt,
        &tables.image_pd,
        &tables.image_pt,
    ];
    let present = pages
        .iter()
        .flat_map(|page| page.entries.iter())
        .filter(|entry| **entry & PRESENT != 0)
        .count();
    assert_eq!(present, 21);
    let base = pages[0] as *const _ as usize;
    assert_eq!(base & 4095, 0);
    for (index, page) in pages.iter().enumerate() {
        let address = *page as *const _ as usize;
        assert_eq!(address & 4095, 0);
        assert_eq!(address - base, index * 4096);
    }

    let direct = walk_boot_page_tables(&tables, DIRECT_VIRTUAL + 123);
    assert!(direct.present && direct.physical == 0x0010_0000 + 123);
    assert!(direct.writable && !direct.executable && !direct.user);

    let heap = walk_boot_page_tables(&tables, HEAP_VIRTUAL + 77);
    assert!(heap.present && heap.physical == 0x0030_0000 + 77);
    assert!(heap.writable && !heap.executable && !heap.user);

    let stack = walk_boot_page_tables(&tables, STACK_VIRTUAL + 31);
    assert!(stack.present && stack.physical == 0x0030_1000 + 31);
    assert!(stack.writable && !stack.executable && !stack.user);
    assert!(!walk_boot_page_tables(&tables, STACK_VIRTUAL - PAGE_SIZE).present);
    assert!(!walk_boot_page_tables(&tables, STACK_VIRTUAL + PAGE_SIZE).present);

    let text = walk_boot_page_tables(&tables, IMAGE_VIRTUAL + 99);
    assert!(text.present && text.physical == 0x0020_0000 + 99);
    assert!(!text.writable && text.executable && !text.user);

    let rodata = walk_boot_page_tables(&tables, IMAGE_VIRTUAL + 0x2000 + 17);
    assert!(rodata.present && rodata.physical == 0x0020_2000 + 17);
    assert!(!rodata.writable && !rodata.executable && !rodata.user);

    let data = walk_boot_page_tables(&tables, IMAGE_VIRTUAL + 0x4000 + 55);
    assert!(data.present && data.physical == 0x0020_4000 + 55);
    assert!(data.writable && !data.executable && !data.user);

    assert!(!walk_boot_page_tables(&tables, 0).present);
    assert!(!walk_boot_page_tables(&tables, 0xffff_ffff_ffff_f000).present);
    assert_eq!(tables.image_pt.entries[0] & (WRITABLE | NO_EXECUTE), 0);
    assert_eq!(tables.image_pt.entries[2] & NO_EXECUTE, NO_EXECUTE);
    assert_eq!(tables.image_pt.entries[4] & (WRITABLE | NO_EXECUTE), WRITABLE | NO_EXECUTE);

    let observation = boot_page_table_observation();
    assert_eq!(observation, 1023);
    println!(
        "M1_BOOT_PAGE_TABLES_OK observation={observation} pages=13 present={present} aligned=4096"
    );
}
