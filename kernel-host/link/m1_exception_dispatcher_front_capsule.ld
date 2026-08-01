ENTRY(tmk_exception_dispatcher)

SECTIONS
{
  . = 0xffffffff80011100;

  .text.tmk_exception_dispatcher : ALIGN(1)
  {
    KEEP(*(.text.tmk_exception_dispatcher))
  }

  tmk_exception_scalar = 0xffffffff80011200;

  /DISCARD/ :
  {
    *(.comment)
    *(.note .note.*)
    *(.eh_frame .eh_frame_hdr)
  }
}

ASSERT(ADDR(.text.tmk_exception_dispatcher) == 0xffffffff80011100,
       "dispatcher-front capsule virtual address drift")
ASSERT(SIZEOF(.text.tmk_exception_dispatcher) == 93,
       "dispatcher-front capsule size drift")
ASSERT(tmk_exception_scalar == 0xffffffff80011200,
       "dispatcher scalar seam drift")
