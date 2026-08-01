ENTRY(tmk_exception_stub_table)

SECTIONS
{
  . = 0xffffffff80010000;
  .text.tmk_exception_stubs : ALIGN(4096)
  {
    tmk_exception_stub_table_link_start = .;
    KEEP(*(.text.tmk_exception_stubs))
    tmk_exception_stub_table_link_end = .;
  }

  tmk_exception_common_entry = 0xffffffff80011000;

  /DISCARD/ :
  {
    *(.comment)
    *(.note*)
  }
}

ASSERT(SIZEOF(.text.tmk_exception_stubs) == 4096,
       "M1 exception-stub table must be exactly 4096 bytes")
ASSERT(tmk_exception_stub_table == tmk_exception_stub_table_link_start,
       "M1 exception-stub entry must equal section start")
ASSERT(tmk_exception_stub_table_link_end == tmk_exception_common_entry,
       "M1 exception stubs must end at the registered common-entry address")
