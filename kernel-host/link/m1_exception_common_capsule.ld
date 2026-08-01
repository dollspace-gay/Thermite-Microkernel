ENTRY(tmk_exception_common_entry)

SECTIONS
{
  . = 0xffffffff80011000;
  .text.tmk_exception_common : ALIGN(16)
  {
    tmk_exception_common_link_start = .;
    KEEP(*(.text.tmk_exception_common))
    tmk_exception_common_link_end = .;
  }

  tmk_exception_dispatcher = 0xffffffff80011100;

  /DISCARD/ :
  {
    *(.comment)
    *(.note*)
  }
}

ASSERT(SIZEOF(.text.tmk_exception_common) == 105,
       "M1 common exception capsule must be exactly 105 bytes")
ASSERT(tmk_exception_common_entry == tmk_exception_common_link_start,
       "M1 common exception entry must equal section start")
