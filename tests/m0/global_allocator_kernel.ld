ENTRY(_start)

SECTIONS
{
  . = 0xffffffff80000000;

  .text : ALIGN(4096)
  {
    *(.text._start)
    *(.text .text.*)
  }

  .rodata : ALIGN(4096)
  {
    *(.rodata .rodata.*)
  }

  .data : ALIGN(4096)
  {
    *(.data .data.*)
  }

  .bss : ALIGN(4096)
  {
    *(.bss .bss.*)
    *(COMMON)
  }

  /DISCARD/ :
  {
    *(.comment)
    *(.eh_frame .eh_frame_hdr)
    *(.note .note.*)
  }
}
