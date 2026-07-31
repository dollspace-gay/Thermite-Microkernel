ENTRY(tmk_hlt_register_capsule)

SECTIONS
{
  . = 0x00100000;
  .text.tmk_capsule : ALIGN(16)
  {
    tmk_hlt_register_capsule_link_start = .;
    KEEP(*(.text.tmk_capsule))
    tmk_hlt_register_capsule_link_end = .;
  }

  /DISCARD/ :
  {
    *(.comment)
    *(.note*)
  }
}

ASSERT(SIZEOF(.text.tmk_capsule) == 4, "M0 capsule must be exactly four bytes")
ASSERT(tmk_hlt_register_capsule == tmk_hlt_register_capsule_link_start,
       "M0 capsule entry must equal section start")
