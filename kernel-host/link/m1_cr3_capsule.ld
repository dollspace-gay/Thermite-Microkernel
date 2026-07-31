ENTRY(tmk_install_cr3_capsule)

SECTIONS
{
  . = 0xffffffff80001000;
  .text.tmk_cr3_capsule : ALIGN(16)
  {
    tmk_install_cr3_capsule_link_start = .;
    KEEP(*(.text.tmk_cr3_capsule))
    tmk_install_cr3_capsule_link_end = .;
  }

  /DISCARD/ :
  {
    *(.comment)
    *(.note*)
  }
}

ASSERT(SIZEOF(.text.tmk_cr3_capsule) == 4,
       "M1 CR3 capsule must be exactly four bytes")
ASSERT(tmk_install_cr3_capsule == tmk_install_cr3_capsule_link_start,
       "M1 CR3 capsule entry must equal section start")
