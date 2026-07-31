PHDRS
{
  text PT_LOAD FLAGS(5);
}

SECTIONS
{
  . = 0x00100000;

  .text.tmk_capsule : ALIGN(16)
  {
    KEEP(*(.text.tmk_capsule))
  } :text

  .text.host : ALIGN(16)
  {
    *(.text .text.*)
  } :text

  .rodata : ALIGN(16)
  {
    *(.rodata .rodata.*)
  } :text

  .data : ALIGN(16)
  {
    *(.data .data.*)
  } :text

  .bss : ALIGN(16)
  {
    *(.bss .bss.* COMMON)
  } :text

  /DISCARD/ :
  {
    *(.comment)
    *(.note*)
    *(.eh_frame .eh_frame.*)
    *(.gcc_except_table .gcc_except_table.*)
    *(.llvmbc)
    *(.llvmcmd)
  }
}

ASSERT(SIZEOF(.text.tmk_capsule) == 4, "registered HLT capsule must be four bytes")
ASSERT(SIZEOF(.rodata) == 0, "M0 host must not contain read-only runtime data")
ASSERT(SIZEOF(.data) == 0, "M0 host must not contain initialized writable data")
ASSERT(SIZEOF(.bss) == 0, "M0 host must not contain writable BSS")
