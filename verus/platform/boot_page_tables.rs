#![no_std]
#![crate_type = "rlib"]

use verus_builtin::*;
use verus_builtin_macros::*;
use vstd::array::ArrayAdditionalSpecFns;

verus! {

pub const PAGE_ENTRIES: usize = 512;
pub const PAGE_SIZE: u64 = 4096;
pub const PRESENT: u64 = 1;
pub const WRITABLE: u64 = 2;
pub const USER: u64 = 4;
pub const LARGE: u64 = 128;
pub const NO_EXECUTE: u64 = 0x8000_0000_0000_0000;
pub const ADDRESS_MASK: u64 = 0x000f_ffff_ffff_f000;

pub const ROOT_PHYSICAL: u64 = 0x0040_0000;
pub const DIRECT_PDPT_PHYSICAL: u64 = 0x0040_1000;
pub const DIRECT_PD_PHYSICAL: u64 = 0x0040_2000;
pub const DIRECT_PT_PHYSICAL: u64 = 0x0040_3000;
pub const HEAP_PDPT_PHYSICAL: u64 = 0x0040_4000;
pub const HEAP_PD_PHYSICAL: u64 = 0x0040_5000;
pub const HEAP_PT_PHYSICAL: u64 = 0x0040_6000;
pub const STACK_PDPT_PHYSICAL: u64 = 0x0040_7000;
pub const STACK_PD_PHYSICAL: u64 = 0x0040_8000;
pub const STACK_PT_PHYSICAL: u64 = 0x0040_9000;
pub const IMAGE_PDPT_PHYSICAL: u64 = 0x0040_a000;
pub const IMAGE_PD_PHYSICAL: u64 = 0x0040_b000;
pub const IMAGE_PT_PHYSICAL: u64 = 0x0040_c000;

pub const DIRECT_VIRTUAL: u64 = 0xffff_8000_0010_0000;
pub const HEAP_VIRTUAL: u64 = 0xffff_c000_0000_0000;
pub const STACK_VIRTUAL: u64 = 0xffff_e000_0000_0000;
pub const IMAGE_VIRTUAL: u64 = 0xffff_ffff_8000_0000;

#[repr(C, align(4096))]
pub struct PageTablePage {
    pub entries: [u64; PAGE_ENTRIES],
}

#[repr(C)]
pub struct BootPageTables {
    pub pml4: PageTablePage,
    pub direct_pdpt: PageTablePage,
    pub direct_pd: PageTablePage,
    pub direct_pt: PageTablePage,
    pub heap_pdpt: PageTablePage,
    pub heap_pd: PageTablePage,
    pub heap_pt: PageTablePage,
    pub stack_pdpt: PageTablePage,
    pub stack_pd: PageTablePage,
    pub stack_pt: PageTablePage,
    pub image_pdpt: PageTablePage,
    pub image_pd: PageTablePage,
    pub image_pt: PageTablePage,
}

pub struct PageTranslation {
    pub present: bool,
    pub physical: u64,
    pub writable: bool,
    pub executable: bool,
    pub user: bool,
}

pub open spec fn single_entry_page(page: &PageTablePage, slot: int, entry: u64) -> bool {
    0 <= slot < PAGE_ENTRIES
        && page.entries[slot] == entry
        && forall|index: int| 0 <= index < PAGE_ENTRIES && index != slot
            ==> page.entries[index] == 0
}

pub open spec fn pml4_page(page: &PageTablePage) -> bool {
    page.entries[256] == (DIRECT_PDPT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE)
        && page.entries[384] == (HEAP_PDPT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE)
        && page.entries[448] == (STACK_PDPT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE)
        && page.entries[511] == (IMAGE_PDPT_PHYSICAL | PRESENT | WRITABLE)
        && forall|index: int| 0 <= index < PAGE_ENTRIES
            && index != 256 && index != 384 && index != 448 && index != 511
            ==> page.entries[index] == 0
}

pub open spec fn image_leaf_page(page: &PageTablePage) -> bool {
    page.entries[0] == (0x0020_0000 | PRESENT)
        && page.entries[1] == (0x0020_1000 | PRESENT)
        && page.entries[2] == (0x0020_2000 | PRESENT | NO_EXECUTE)
        && page.entries[3] == (0x0020_3000 | PRESENT | NO_EXECUTE)
        && page.entries[4] == (0x0020_4000 | PRESENT | WRITABLE | NO_EXECUTE)
        && page.entries[5] == (0x0020_5000 | PRESENT | WRITABLE | NO_EXECUTE)
        && forall|index: int| 6 <= index < PAGE_ENTRIES ==> page.entries[index] == 0
}

pub open spec fn boot_tables_well_formed(tables: &BootPageTables) -> bool {
    pml4_page(&tables.pml4)
        && single_entry_page(
            &tables.direct_pdpt,
            0,
            DIRECT_PD_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE,
        )
        && single_entry_page(
            &tables.direct_pd,
            0,
            DIRECT_PT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE,
        )
        && single_entry_page(
            &tables.direct_pt,
            256,
            0x0010_0000 | PRESENT | WRITABLE | NO_EXECUTE,
        )
        && single_entry_page(
            &tables.heap_pdpt,
            0,
            HEAP_PD_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE,
        )
        && single_entry_page(
            &tables.heap_pd,
            0,
            HEAP_PT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE,
        )
        && single_entry_page(
            &tables.heap_pt,
            0,
            0x0030_0000 | PRESENT | WRITABLE | NO_EXECUTE,
        )
        && single_entry_page(
            &tables.stack_pdpt,
            0,
            STACK_PD_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE,
        )
        && single_entry_page(
            &tables.stack_pd,
            0,
            STACK_PT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE,
        )
        && single_entry_page(
            &tables.stack_pt,
            0,
            0x0030_1000 | PRESENT | WRITABLE | NO_EXECUTE,
        )
        && single_entry_page(
            &tables.image_pdpt,
            510,
            IMAGE_PD_PHYSICAL | PRESENT | WRITABLE,
        )
        && single_entry_page(
            &tables.image_pd,
            0,
            IMAGE_PT_PHYSICAL | PRESENT | WRITABLE,
        )
        && image_leaf_page(&tables.image_pt)
}

pub open spec fn registered_sample_translation(
    virtual_address: u64,
    translation: PageTranslation,
) -> bool {
    (virtual_address == DIRECT_VIRTUAL + 123 ==> (
        translation.present
            && translation.physical == 0x0010_0000 + 123
            && translation.writable && !translation.executable && !translation.user
    ))
        && (virtual_address == HEAP_VIRTUAL + 77 ==> (
            translation.present
                && translation.physical == 0x0030_0000 + 77
                && translation.writable && !translation.executable && !translation.user
        ))
        && (virtual_address == STACK_VIRTUAL + 31 ==> (
            translation.present
                && translation.physical == 0x0030_1000 + 31
                && translation.writable && !translation.executable && !translation.user
        ))
        && (virtual_address == STACK_VIRTUAL - PAGE_SIZE ==> !translation.present)
        && (virtual_address == STACK_VIRTUAL + PAGE_SIZE ==> !translation.present)
        && (virtual_address == IMAGE_VIRTUAL + 99 ==> (
            translation.present
                && translation.physical == 0x0020_0000 + 99
                && !translation.writable && translation.executable && !translation.user
        ))
        && (virtual_address == IMAGE_VIRTUAL + 0x2000 + 17 ==> (
            translation.present
                && translation.physical == 0x0020_2000 + 17
                && !translation.writable && !translation.executable && !translation.user
        ))
        && (virtual_address == IMAGE_VIRTUAL + 0x4000 + 55 ==> (
            translation.present
                && translation.physical == 0x0020_4000 + 55
                && translation.writable && !translation.executable && !translation.user
        ))
        && (virtual_address == 0 ==> !translation.present)
        && (virtual_address == 0xffff_ffff_ffff_f000 ==> !translation.present)
}

pub fn registered_boot_page_tables() -> (result: BootPageTables)
    ensures boot_tables_well_formed(&result),
{
    let mut pml4 = [0u64; PAGE_ENTRIES];
    pml4[256] = DIRECT_PDPT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE;
    pml4[384] = HEAP_PDPT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE;
    pml4[448] = STACK_PDPT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE;
    pml4[511] = IMAGE_PDPT_PHYSICAL | PRESENT | WRITABLE;

    let mut direct_pdpt = [0u64; PAGE_ENTRIES];
    direct_pdpt[0] = DIRECT_PD_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE;
    let mut direct_pd = [0u64; PAGE_ENTRIES];
    direct_pd[0] = DIRECT_PT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE;
    let mut direct_pt = [0u64; PAGE_ENTRIES];
    direct_pt[256] = 0x0010_0000 | PRESENT | WRITABLE | NO_EXECUTE;

    let mut heap_pdpt = [0u64; PAGE_ENTRIES];
    heap_pdpt[0] = HEAP_PD_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE;
    let mut heap_pd = [0u64; PAGE_ENTRIES];
    heap_pd[0] = HEAP_PT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE;
    let mut heap_pt = [0u64; PAGE_ENTRIES];
    heap_pt[0] = 0x0030_0000 | PRESENT | WRITABLE | NO_EXECUTE;

    let mut stack_pdpt = [0u64; PAGE_ENTRIES];
    stack_pdpt[0] = STACK_PD_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE;
    let mut stack_pd = [0u64; PAGE_ENTRIES];
    stack_pd[0] = STACK_PT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE;
    let mut stack_pt = [0u64; PAGE_ENTRIES];
    stack_pt[0] = 0x0030_1000 | PRESENT | WRITABLE | NO_EXECUTE;

    let mut image_pdpt = [0u64; PAGE_ENTRIES];
    image_pdpt[510] = IMAGE_PD_PHYSICAL | PRESENT | WRITABLE;
    let mut image_pd = [0u64; PAGE_ENTRIES];
    image_pd[0] = IMAGE_PT_PHYSICAL | PRESENT | WRITABLE;
    let mut image_pt = [0u64; PAGE_ENTRIES];
    image_pt[0] = 0x0020_0000 | PRESENT;
    image_pt[1] = 0x0020_1000 | PRESENT;
    image_pt[2] = 0x0020_2000 | PRESENT | NO_EXECUTE;
    image_pt[3] = 0x0020_3000 | PRESENT | NO_EXECUTE;
    image_pt[4] = 0x0020_4000 | PRESENT | WRITABLE | NO_EXECUTE;
    image_pt[5] = 0x0020_5000 | PRESENT | WRITABLE | NO_EXECUTE;

    BootPageTables {
        pml4: PageTablePage { entries: pml4 },
        direct_pdpt: PageTablePage { entries: direct_pdpt },
        direct_pd: PageTablePage { entries: direct_pd },
        direct_pt: PageTablePage { entries: direct_pt },
        heap_pdpt: PageTablePage { entries: heap_pdpt },
        heap_pd: PageTablePage { entries: heap_pd },
        heap_pt: PageTablePage { entries: heap_pt },
        stack_pdpt: PageTablePage { entries: stack_pdpt },
        stack_pd: PageTablePage { entries: stack_pd },
        stack_pt: PageTablePage { entries: stack_pt },
        image_pdpt: PageTablePage { entries: image_pdpt },
        image_pd: PageTablePage { entries: image_pd },
        image_pt: PageTablePage { entries: image_pt },
    }
}

fn absent_translation() -> (result: PageTranslation)
    ensures
        !result.present,
        result.physical == 0,
        !result.writable,
        !result.executable,
        !result.user,
{
    PageTranslation {
        present: false,
        physical: 0,
        writable: false,
        executable: false,
        user: false,
    }
}

pub fn walk_boot_page_tables(
    tables: &BootPageTables,
    virtual_address: u64,
) -> (result: PageTranslation)
    requires boot_tables_well_formed(tables),
    ensures registered_sample_translation(virtual_address, result),
{
    assert(((virtual_address >> 39) & 511u64) <= 511u64) by(bit_vector);
    assert(((virtual_address >> 30) & 511u64) <= 511u64) by(bit_vector);
    assert(((virtual_address >> 21) & 511u64) <= 511u64) by(bit_vector);
    assert(((virtual_address >> 12) & 511u64) <= 511u64) by(bit_vector);
    let pml4_index_u64 = (virtual_address >> 39) & 511u64;
    let pdpt_index_u64 = (virtual_address >> 30) & 511u64;
    let pd_index_u64 = (virtual_address >> 21) & 511u64;
    let pt_index_u64 = (virtual_address >> 12) & 511u64;
    let pml4_index = pml4_index_u64 as usize;
    let pdpt_index = pdpt_index_u64 as usize;
    let pd_index = pd_index_u64 as usize;
    let pt_index = pt_index_u64 as usize;
    assert(pml4_index < 512usize);
    assert(pdpt_index < 512usize);
    assert(pd_index < 512usize);
    assert(pt_index < 512usize);
    assert(virtual_address == DIRECT_VIRTUAL + 123 ==> (
        ((virtual_address >> 39) & 511u64) == 256
            && ((virtual_address >> 30) & 511u64) == 0
            && ((virtual_address >> 21) & 511u64) == 0
            && ((virtual_address >> 12) & 511u64) == 256
    )) by(bit_vector);
    assert(virtual_address == HEAP_VIRTUAL + 77 ==> (
        ((virtual_address >> 39) & 511u64) == 384
            && ((virtual_address >> 30) & 511u64) == 0
            && ((virtual_address >> 21) & 511u64) == 0
            && ((virtual_address >> 12) & 511u64) == 0
    )) by(bit_vector);
    assert(virtual_address == STACK_VIRTUAL + 31 ==> (
        ((virtual_address >> 39) & 511u64) == 448
            && ((virtual_address >> 30) & 511u64) == 0
            && ((virtual_address >> 21) & 511u64) == 0
            && ((virtual_address >> 12) & 511u64) == 0
    )) by(bit_vector);
    assert(virtual_address == STACK_VIRTUAL - PAGE_SIZE ==> (
        ((virtual_address >> 39) & 511u64) == 447
            && ((virtual_address >> 30) & 511u64) == 511
            && ((virtual_address >> 21) & 511u64) == 511
            && ((virtual_address >> 12) & 511u64) == 511
    )) by(bit_vector);
    assert(virtual_address == STACK_VIRTUAL + PAGE_SIZE ==> (
        ((virtual_address >> 39) & 511u64) == 448
            && ((virtual_address >> 30) & 511u64) == 0
            && ((virtual_address >> 21) & 511u64) == 0
            && ((virtual_address >> 12) & 511u64) == 1
    )) by(bit_vector);
    assert(virtual_address == IMAGE_VIRTUAL + 99 ==> (
        ((virtual_address >> 39) & 511u64) == 511
            && ((virtual_address >> 30) & 511u64) == 510
            && ((virtual_address >> 21) & 511u64) == 0
            && ((virtual_address >> 12) & 511u64) == 0
    )) by(bit_vector);
    assert(virtual_address == IMAGE_VIRTUAL + 0x2000 + 17 ==> (
        ((virtual_address >> 39) & 511u64) == 511
            && ((virtual_address >> 30) & 511u64) == 510
            && ((virtual_address >> 21) & 511u64) == 0
            && ((virtual_address >> 12) & 511u64) == 2
    )) by(bit_vector);
    assert(virtual_address == IMAGE_VIRTUAL + 0x4000 + 55 ==> (
        ((virtual_address >> 39) & 511u64) == 511
            && ((virtual_address >> 30) & 511u64) == 510
            && ((virtual_address >> 21) & 511u64) == 0
            && ((virtual_address >> 12) & 511u64) == 4
    )) by(bit_vector);
    assert(virtual_address == 0 ==> (
        ((virtual_address >> 39) & 511u64) == 0
            && ((virtual_address >> 30) & 511u64) == 0
            && ((virtual_address >> 21) & 511u64) == 0
            && ((virtual_address >> 12) & 511u64) == 0
    )) by(bit_vector);
    assert(virtual_address == 0xffff_ffff_ffff_f000 ==> (
        ((virtual_address >> 39) & 511u64) == 511
            && ((virtual_address >> 30) & 511u64) == 511
            && ((virtual_address >> 21) & 511u64) == 511
            && ((virtual_address >> 12) & 511u64) == 511
    )) by(bit_vector);
    assert(((DIRECT_PDPT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & ADDRESS_MASK)
        == DIRECT_PDPT_PHYSICAL
        && ((DIRECT_PDPT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & PRESENT) != 0) by(bit_vector);
    assert(((HEAP_PDPT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & ADDRESS_MASK)
        == HEAP_PDPT_PHYSICAL
        && ((HEAP_PDPT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & PRESENT) != 0) by(bit_vector);
    assert(((STACK_PDPT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & ADDRESS_MASK)
        == STACK_PDPT_PHYSICAL
        && ((STACK_PDPT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & PRESENT) != 0) by(bit_vector);
    assert(((IMAGE_PDPT_PHYSICAL | PRESENT | WRITABLE) & ADDRESS_MASK)
        == IMAGE_PDPT_PHYSICAL
        && ((IMAGE_PDPT_PHYSICAL | PRESENT | WRITABLE) & PRESENT) != 0) by(bit_vector);
    assert(((DIRECT_PD_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & ADDRESS_MASK)
        == DIRECT_PD_PHYSICAL
        && ((DIRECT_PD_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & PRESENT) != 0
        && ((DIRECT_PD_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & LARGE) == 0) by(bit_vector);
    assert(((HEAP_PD_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & ADDRESS_MASK)
        == HEAP_PD_PHYSICAL
        && ((HEAP_PD_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & PRESENT) != 0
        && ((HEAP_PD_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & LARGE) == 0) by(bit_vector);
    assert(((STACK_PD_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & ADDRESS_MASK)
        == STACK_PD_PHYSICAL
        && ((STACK_PD_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & PRESENT) != 0
        && ((STACK_PD_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & LARGE) == 0) by(bit_vector);
    assert(((IMAGE_PD_PHYSICAL | PRESENT | WRITABLE) & ADDRESS_MASK)
        == IMAGE_PD_PHYSICAL
        && ((IMAGE_PD_PHYSICAL | PRESENT | WRITABLE) & PRESENT) != 0
        && ((IMAGE_PD_PHYSICAL | PRESENT | WRITABLE) & LARGE) == 0) by(bit_vector);
    assert(((DIRECT_PT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & ADDRESS_MASK)
        == DIRECT_PT_PHYSICAL
        && ((DIRECT_PT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & PRESENT) != 0
        && ((DIRECT_PT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & LARGE) == 0) by(bit_vector);
    assert(((HEAP_PT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & ADDRESS_MASK)
        == HEAP_PT_PHYSICAL
        && ((HEAP_PT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & PRESENT) != 0
        && ((HEAP_PT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & LARGE) == 0) by(bit_vector);
    assert(((STACK_PT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & ADDRESS_MASK)
        == STACK_PT_PHYSICAL
        && ((STACK_PT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & PRESENT) != 0
        && ((STACK_PT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & LARGE) == 0) by(bit_vector);
    assert(((IMAGE_PT_PHYSICAL | PRESENT | WRITABLE) & ADDRESS_MASK)
        == IMAGE_PT_PHYSICAL
        && ((IMAGE_PT_PHYSICAL | PRESENT | WRITABLE) & PRESENT) != 0
        && ((IMAGE_PT_PHYSICAL | PRESENT | WRITABLE) & LARGE) == 0) by(bit_vector);
    assert(((0x0010_0000u64 | PRESENT | WRITABLE | NO_EXECUTE) & ADDRESS_MASK) == 0x0010_0000
        && ((0x0010_0000u64 | PRESENT | WRITABLE | NO_EXECUTE) & PRESENT) != 0) by(bit_vector);
    assert(((0x0030_0000u64 | PRESENT | WRITABLE | NO_EXECUTE) & ADDRESS_MASK) == 0x0030_0000
        && ((0x0030_0000u64 | PRESENT | WRITABLE | NO_EXECUTE) & PRESENT) != 0) by(bit_vector);
    assert(((0x0030_1000u64 | PRESENT | WRITABLE | NO_EXECUTE) & ADDRESS_MASK) == 0x0030_1000
        && ((0x0030_1000u64 | PRESENT | WRITABLE | NO_EXECUTE) & PRESENT) != 0) by(bit_vector);
    assert(((0x0020_0000u64 | PRESENT) & ADDRESS_MASK) == 0x0020_0000
        && ((0x0020_0000u64 | PRESENT) & PRESENT) != 0) by(bit_vector);
    assert(((0x0020_2000u64 | PRESENT | NO_EXECUTE) & ADDRESS_MASK) == 0x0020_2000
        && ((0x0020_2000u64 | PRESENT | NO_EXECUTE) & PRESENT) != 0) by(bit_vector);
    assert(((0x0020_4000u64 | PRESENT | WRITABLE | NO_EXECUTE) & ADDRESS_MASK) == 0x0020_4000
        && ((0x0020_4000u64 | PRESENT | WRITABLE | NO_EXECUTE) & PRESENT) != 0) by(bit_vector);
    assert((0u64 & PRESENT) == 0) by(bit_vector);
    assert(
        ((DIRECT_PDPT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & WRITABLE) != 0
            && ((DIRECT_PDPT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & NO_EXECUTE) != 0
            && ((DIRECT_PD_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & WRITABLE) != 0
            && ((DIRECT_PD_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & NO_EXECUTE) != 0
            && ((DIRECT_PT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & WRITABLE) != 0
            && ((DIRECT_PT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & NO_EXECUTE) != 0
            && ((HEAP_PDPT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & WRITABLE) != 0
            && ((HEAP_PDPT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & NO_EXECUTE) != 0
            && ((HEAP_PD_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & WRITABLE) != 0
            && ((HEAP_PD_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & NO_EXECUTE) != 0
            && ((HEAP_PT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & WRITABLE) != 0
            && ((HEAP_PT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & NO_EXECUTE) != 0
            && ((STACK_PDPT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & WRITABLE) != 0
            && ((STACK_PDPT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & NO_EXECUTE) != 0
            && ((STACK_PD_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & WRITABLE) != 0
            && ((STACK_PD_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & NO_EXECUTE) != 0
            && ((STACK_PT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & WRITABLE) != 0
            && ((STACK_PT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & NO_EXECUTE) != 0
    ) by(bit_vector);
    assert(
        ((IMAGE_PDPT_PHYSICAL | PRESENT | WRITABLE) & WRITABLE) != 0
            && ((IMAGE_PDPT_PHYSICAL | PRESENT | WRITABLE) & NO_EXECUTE) == 0
            && ((IMAGE_PD_PHYSICAL | PRESENT | WRITABLE) & WRITABLE) != 0
            && ((IMAGE_PD_PHYSICAL | PRESENT | WRITABLE) & NO_EXECUTE) == 0
            && ((IMAGE_PT_PHYSICAL | PRESENT | WRITABLE) & WRITABLE) != 0
            && ((IMAGE_PT_PHYSICAL | PRESENT | WRITABLE) & NO_EXECUTE) == 0
    ) by(bit_vector);
    assert(
        ((0x0010_0000u64 | PRESENT | WRITABLE | NO_EXECUTE) & WRITABLE) != 0
            && ((0x0010_0000u64 | PRESENT | WRITABLE | NO_EXECUTE) & NO_EXECUTE) != 0
            && ((0x0030_0000u64 | PRESENT | WRITABLE | NO_EXECUTE) & WRITABLE) != 0
            && ((0x0030_0000u64 | PRESENT | WRITABLE | NO_EXECUTE) & NO_EXECUTE) != 0
            && ((0x0030_1000u64 | PRESENT | WRITABLE | NO_EXECUTE) & WRITABLE) != 0
            && ((0x0030_1000u64 | PRESENT | WRITABLE | NO_EXECUTE) & NO_EXECUTE) != 0
            && ((0x0020_0000u64 | PRESENT) & WRITABLE) == 0
            && ((0x0020_0000u64 | PRESENT) & NO_EXECUTE) == 0
            && ((0x0020_2000u64 | PRESENT | NO_EXECUTE) & WRITABLE) == 0
            && ((0x0020_2000u64 | PRESENT | NO_EXECUTE) & NO_EXECUTE) != 0
            && ((0x0020_4000u64 | PRESENT | WRITABLE | NO_EXECUTE) & WRITABLE) != 0
            && ((0x0020_4000u64 | PRESENT | WRITABLE | NO_EXECUTE) & NO_EXECUTE) != 0
    ) by(bit_vector);
    assert(
        ((DIRECT_PDPT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & USER) == 0
            && ((HEAP_PDPT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & USER) == 0
            && ((STACK_PDPT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE) & USER) == 0
            && ((IMAGE_PDPT_PHYSICAL | PRESENT | WRITABLE) & USER) == 0
    ) by(bit_vector);
    assert(virtual_address == DIRECT_VIRTUAL + 123
        ==> (virtual_address & 4095u64) == 123) by(bit_vector);
    assert(virtual_address == HEAP_VIRTUAL + 77
        ==> (virtual_address & 4095u64) == 77) by(bit_vector);
    assert(virtual_address == STACK_VIRTUAL + 31
        ==> (virtual_address & 4095u64) == 31) by(bit_vector);
    assert(virtual_address == IMAGE_VIRTUAL + 99
        ==> (virtual_address & 4095u64) == 99) by(bit_vector);
    assert(virtual_address == IMAGE_VIRTUAL + 0x2000 + 17
        ==> (virtual_address & 4095u64) == 17) by(bit_vector);
    assert(virtual_address == IMAGE_VIRTUAL + 0x4000 + 55
        ==> (virtual_address & 4095u64) == 55) by(bit_vector);

    let pml4_entry = tables.pml4.entries[pml4_index];
    assert(pml4_page(&tables.pml4));
    if pml4_index != 256 && pml4_index != 384
        && pml4_index != 448 && pml4_index != 511 {
        assert(pml4_entry == 0);
    }
    if virtual_address == DIRECT_VIRTUAL + 123 {
        assert(pml4_entry == (DIRECT_PDPT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE));
    } else if virtual_address == HEAP_VIRTUAL + 77 {
        assert(pml4_entry == (HEAP_PDPT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE));
    } else if virtual_address == STACK_VIRTUAL + 31
        || virtual_address == STACK_VIRTUAL + PAGE_SIZE {
        assert(pml4_entry == (STACK_PDPT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE));
    } else if virtual_address == IMAGE_VIRTUAL + 99
        || virtual_address == IMAGE_VIRTUAL + 0x2000 + 17
        || virtual_address == IMAGE_VIRTUAL + 0x4000 + 55
        || virtual_address == 0xffff_ffff_ffff_f000 {
        assert(pml4_entry == (IMAGE_PDPT_PHYSICAL | PRESENT | WRITABLE));
    } else if virtual_address == STACK_VIRTUAL - PAGE_SIZE || virtual_address == 0 {
        assert(pml4_entry == 0);
        assert(pml4_entry & PRESENT == 0);
    }
    if pml4_entry & PRESENT == 0 {
        return absent_translation();
    }
    let pml4_address = pml4_entry & ADDRESS_MASK;
    let pdpt = if pml4_address == DIRECT_PDPT_PHYSICAL {
        assert(single_entry_page(
            &tables.direct_pdpt,
            0,
            DIRECT_PD_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE,
        ));
        &tables.direct_pdpt
    } else if pml4_address == HEAP_PDPT_PHYSICAL {
        assert(single_entry_page(
            &tables.heap_pdpt,
            0,
            HEAP_PD_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE,
        ));
        &tables.heap_pdpt
    } else if pml4_address == STACK_PDPT_PHYSICAL {
        assert(single_entry_page(
            &tables.stack_pdpt,
            0,
            STACK_PD_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE,
        ));
        &tables.stack_pdpt
    } else if pml4_address == IMAGE_PDPT_PHYSICAL {
        assert(single_entry_page(
            &tables.image_pdpt,
            510,
            IMAGE_PD_PHYSICAL | PRESENT | WRITABLE,
        ));
        &tables.image_pdpt
    } else {
        return absent_translation();
    };
    let pdpt_entry = pdpt.entries[pdpt_index];
    if virtual_address == DIRECT_VIRTUAL + 123 {
        assert(pdpt_entry == (DIRECT_PD_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE));
    } else if virtual_address == HEAP_VIRTUAL + 77 {
        assert(pdpt_entry == (HEAP_PD_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE));
    } else if virtual_address == STACK_VIRTUAL + 31
        || virtual_address == STACK_VIRTUAL + PAGE_SIZE {
        assert(pdpt_entry == (STACK_PD_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE));
    } else if virtual_address == IMAGE_VIRTUAL + 99
        || virtual_address == IMAGE_VIRTUAL + 0x2000 + 17
        || virtual_address == IMAGE_VIRTUAL + 0x4000 + 55 {
        assert(pdpt_entry == (IMAGE_PD_PHYSICAL | PRESENT | WRITABLE));
    } else if virtual_address == 0xffff_ffff_ffff_f000 {
        assert(pdpt_entry == 0);
        assert(pdpt_entry & PRESENT == 0);
    }
    if pdpt_entry & PRESENT == 0 || pdpt_entry & LARGE != 0 {
        return absent_translation();
    }
    let pdpt_address = pdpt_entry & ADDRESS_MASK;
    let pd = if pdpt_address == DIRECT_PD_PHYSICAL {
        assert(single_entry_page(
            &tables.direct_pd,
            0,
            DIRECT_PT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE,
        ));
        &tables.direct_pd
    } else if pdpt_address == HEAP_PD_PHYSICAL {
        assert(single_entry_page(
            &tables.heap_pd,
            0,
            HEAP_PT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE,
        ));
        &tables.heap_pd
    } else if pdpt_address == STACK_PD_PHYSICAL {
        assert(single_entry_page(
            &tables.stack_pd,
            0,
            STACK_PT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE,
        ));
        &tables.stack_pd
    } else if pdpt_address == IMAGE_PD_PHYSICAL {
        assert(single_entry_page(
            &tables.image_pd,
            0,
            IMAGE_PT_PHYSICAL | PRESENT | WRITABLE,
        ));
        &tables.image_pd
    } else {
        return absent_translation();
    };
    let pd_entry = pd.entries[pd_index];
    if virtual_address == DIRECT_VIRTUAL + 123 {
        assert(pd_entry == (DIRECT_PT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE));
    } else if virtual_address == HEAP_VIRTUAL + 77 {
        assert(pd_entry == (HEAP_PT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE));
    } else if virtual_address == STACK_VIRTUAL + 31
        || virtual_address == STACK_VIRTUAL + PAGE_SIZE {
        assert(pd_entry == (STACK_PT_PHYSICAL | PRESENT | WRITABLE | NO_EXECUTE));
    } else if virtual_address == IMAGE_VIRTUAL + 99
        || virtual_address == IMAGE_VIRTUAL + 0x2000 + 17
        || virtual_address == IMAGE_VIRTUAL + 0x4000 + 55 {
        assert(pd_entry == (IMAGE_PT_PHYSICAL | PRESENT | WRITABLE));
    }
    if pd_entry & PRESENT == 0 || pd_entry & LARGE != 0 {
        return absent_translation();
    }
    let pd_address = pd_entry & ADDRESS_MASK;
    let pt = if pd_address == DIRECT_PT_PHYSICAL {
        assert(single_entry_page(
            &tables.direct_pt,
            256,
            0x0010_0000 | PRESENT | WRITABLE | NO_EXECUTE,
        ));
        &tables.direct_pt
    } else if pd_address == HEAP_PT_PHYSICAL {
        assert(single_entry_page(
            &tables.heap_pt,
            0,
            0x0030_0000 | PRESENT | WRITABLE | NO_EXECUTE,
        ));
        &tables.heap_pt
    } else if pd_address == STACK_PT_PHYSICAL {
        assert(single_entry_page(
            &tables.stack_pt,
            0,
            0x0030_1000 | PRESENT | WRITABLE | NO_EXECUTE,
        ));
        &tables.stack_pt
    } else if pd_address == IMAGE_PT_PHYSICAL {
        assert(image_leaf_page(&tables.image_pt));
        &tables.image_pt
    } else {
        return absent_translation();
    };
    let pt_entry = pt.entries[pt_index];
    if virtual_address == DIRECT_VIRTUAL + 123 {
        assert(pt_entry == (0x0010_0000 | PRESENT | WRITABLE | NO_EXECUTE));
    } else if virtual_address == HEAP_VIRTUAL + 77 {
        assert(pt_entry == (0x0030_0000 | PRESENT | WRITABLE | NO_EXECUTE));
    } else if virtual_address == STACK_VIRTUAL + 31 {
        assert(pt_entry == (0x0030_1000 | PRESENT | WRITABLE | NO_EXECUTE));
    } else if virtual_address == STACK_VIRTUAL + PAGE_SIZE {
        assert(pt_entry == 0);
        assert(pt_entry & PRESENT == 0);
    } else if virtual_address == IMAGE_VIRTUAL + 99 {
        assert(pt_entry == (0x0020_0000 | PRESENT));
    } else if virtual_address == IMAGE_VIRTUAL + 0x2000 + 17 {
        assert(pt_entry == (0x0020_2000 | PRESENT | NO_EXECUTE));
    } else if virtual_address == IMAGE_VIRTUAL + 0x4000 + 55 {
        assert(pt_entry == (0x0020_4000 | PRESENT | WRITABLE | NO_EXECUTE));
    }
    if pt_entry & PRESENT == 0 {
        return absent_translation();
    }
    let physical_page = pt_entry & ADDRESS_MASK;
    let offset = virtual_address & 4095;
    assert((pt_entry & 0x000f_ffff_ffff_f000u64)
        <= 0x000f_ffff_ffff_f000u64) by(bit_vector);
    assert((virtual_address & 4095u64) <= 4095u64) by(bit_vector);
    assert(physical_page <= u64::MAX - offset);
    let writable = pml4_entry & WRITABLE != 0
        && pdpt_entry & WRITABLE != 0
        && pd_entry & WRITABLE != 0
        && pt_entry & WRITABLE != 0;
    let executable = pml4_entry & NO_EXECUTE == 0
        && pdpt_entry & NO_EXECUTE == 0
        && pd_entry & NO_EXECUTE == 0
        && pt_entry & NO_EXECUTE == 0;
    let user = pml4_entry & USER != 0
        && pdpt_entry & USER != 0
        && pd_entry & USER != 0
        && pt_entry & USER != 0;
    let translation = PageTranslation {
        present: true,
        physical: physical_page + offset,
        writable,
        executable,
        user,
    };
    if virtual_address == DIRECT_VIRTUAL + 123 {
        assert(translation.present && translation.physical == 0x0010_0000 + 123);
        assert(translation.writable && !translation.executable && !translation.user);
    } else if virtual_address == HEAP_VIRTUAL + 77 {
        assert(translation.present && translation.physical == 0x0030_0000 + 77);
        assert(translation.writable && !translation.executable && !translation.user);
    } else if virtual_address == STACK_VIRTUAL + 31 {
        assert(translation.present && translation.physical == 0x0030_1000 + 31);
        assert(translation.writable && !translation.executable && !translation.user);
    } else if virtual_address == IMAGE_VIRTUAL + 99 {
        assert(translation.present && translation.physical == 0x0020_0000 + 99);
        assert(!translation.writable && translation.executable && !translation.user);
    } else if virtual_address == IMAGE_VIRTUAL + 0x2000 + 17 {
        assert(translation.present && translation.physical == 0x0020_2000 + 17);
        assert(!translation.writable && !translation.executable && !translation.user);
    } else if virtual_address == IMAGE_VIRTUAL + 0x4000 + 55 {
        assert(translation.present && translation.physical == 0x0020_4000 + 55);
        assert(translation.writable && !translation.executable && !translation.user);
    }
    assert(virtual_address != STACK_VIRTUAL - PAGE_SIZE);
    assert(virtual_address != STACK_VIRTUAL + PAGE_SIZE);
    assert(virtual_address != 0);
    assert(virtual_address != 0xffff_ffff_ffff_f000);
    assert(registered_sample_translation(virtual_address, translation));
    translation
}

pub fn boot_page_table_observation() -> (result: u64)
    ensures result == 1023,
{
    let tables = registered_boot_page_tables();
    let direct = walk_boot_page_tables(&tables, DIRECT_VIRTUAL + 123);
    let heap = walk_boot_page_tables(&tables, HEAP_VIRTUAL + 77);
    let stack = walk_boot_page_tables(&tables, STACK_VIRTUAL + 31);
    let guard_before = walk_boot_page_tables(&tables, STACK_VIRTUAL - PAGE_SIZE);
    let guard_after = walk_boot_page_tables(&tables, STACK_VIRTUAL + PAGE_SIZE);
    let text = walk_boot_page_tables(&tables, IMAGE_VIRTUAL + 99);
    let rodata = walk_boot_page_tables(&tables, IMAGE_VIRTUAL + 0x2000 + 17);
    let data = walk_boot_page_tables(&tables, IMAGE_VIRTUAL + 0x4000 + 55);
    let low_guard = walk_boot_page_tables(&tables, 0);
    let recursive = walk_boot_page_tables(&tables, 0xffff_ffff_ffff_f000);
    assert(direct.present && direct.physical == 0x0010_0000 + 123
        && direct.writable && !direct.executable && !direct.user);
    assert(heap.present && heap.physical == 0x0030_0000 + 77
        && heap.writable && !heap.executable && !heap.user);
    assert(stack.present && stack.physical == 0x0030_1000 + 31
        && stack.writable && !stack.executable && !stack.user);
    assert(!guard_before.present && !guard_after.present);
    assert(text.present && text.physical == 0x0020_0000 + 99
        && !text.writable && text.executable && !text.user);
    assert(rodata.present && rodata.physical == 0x0020_2000 + 17
        && !rodata.writable && !rodata.executable && !rodata.user);
    assert(data.present && data.physical == 0x0020_4000 + 55
        && data.writable && !data.executable && !data.user);
    assert(!low_guard.present && !recursive.present);
    assert((0u64 | 1u64) == 1u64
        && (1u64 | 2u64) == 3u64
        && (3u64 | 4u64) == 7u64
        && (7u64 | 8u64) == 15u64
        && (15u64 | 16u64) == 31u64
        && (31u64 | 32u64) == 63u64
        && (63u64 | 64u64) == 127u64
        && (127u64 | 128u64) == 255u64
        && (255u64 | 256u64) == 511u64
        && (511u64 | 512u64) == 1023u64) by(bit_vector);
    let mut observation = 0u64;
    if direct.present && direct.physical == 0x0010_0000 + 123
        && direct.writable && !direct.executable && !direct.user {
        observation = observation | 1;
    }
    assert(observation == 1);
    if heap.present && heap.physical == 0x0030_0000 + 77
        && heap.writable && !heap.executable && !heap.user {
        observation = observation | 2;
    }
    assert(observation == 3);
    if stack.present && stack.physical == 0x0030_1000 + 31
        && stack.writable && !stack.executable && !stack.user {
        observation = observation | 4;
    }
    assert(observation == 7);
    if !guard_before.present {
        observation = observation | 8;
    }
    assert(observation == 15);
    if !guard_after.present {
        observation = observation | 16;
    }
    assert(observation == 31);
    if text.present && text.physical == 0x0020_0000 + 99
        && !text.writable && text.executable && !text.user {
        observation = observation | 32;
    }
    assert(observation == 63);
    if rodata.present && rodata.physical == 0x0020_2000 + 17
        && !rodata.writable && !rodata.executable && !rodata.user {
        observation = observation | 64;
    }
    assert(observation == 127);
    if data.present && data.physical == 0x0020_4000 + 55
        && data.writable && !data.executable && !data.user {
        observation = observation | 128;
    }
    assert(observation == 255);
    if !low_guard.present {
        observation = observation | 256;
    }
    assert(observation == 511);
    if !recursive.present {
        observation = observation | 512;
    }
    assert(observation == 1023);
    observation
}

}
