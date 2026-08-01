ENTRY(tmk_exception_scalar_entry)

SECTIONS
{
  . = 0xffffffff80011200;

  .text.tmk_exception_scalar_entry : ALIGN(1)
  {
    tmk_exception_scalar_entry = .;
    KEEP(*(.text.tmk_exception_scalar_entry))
  }

  tmk_exception_scalar_core = 0xffffffff80011300;

  /DISCARD/ :
  {
    *(.comment)
    *(.note .note.*)
    *(.eh_frame .eh_frame_hdr)
  }
}

ASSERT(ADDR(.text.tmk_exception_scalar_entry) == 0xffffffff80011200,
       "scalar-entry capsule virtual address drift")
ASSERT(SIZEOF(.text.tmk_exception_scalar_entry) == 11,
       "scalar-entry capsule size drift")
ASSERT(tmk_exception_scalar_core == 0xffffffff80011300,
       "scalar-core seam drift")
