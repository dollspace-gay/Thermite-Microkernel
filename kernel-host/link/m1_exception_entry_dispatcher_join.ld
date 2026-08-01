OUTPUT_FORMAT(elf64-x86-64)
OUTPUT_ARCH(i386:x86-64)
ENTRY(tmk_exception_common)

SECTIONS
{
  . = 0xffffffff80011000;
  .text.tmk_exception_common : ALIGN(1)
  {
    KEEP(*(.text.tmk_exception_common))
  }

  . = 0xffffffff80011100;
  .text.tmk_exception_dispatcher : ALIGN(1)
  {
    KEEP(*(.text.tmk_exception_dispatcher))
  }

  tmk_exception_scalar = 0xffffffff80011200;

  /DISCARD/ :
  {
    *(.comment)
    *(.note*)
    *(.eh_frame*)
    *(.data*)
    *(.bss*)
    *(.rodata*)
  }
}

ASSERT(ADDR(.text.tmk_exception_common) == 0xffffffff80011000,
       "common-entry address changed")
ASSERT(SIZEOF(.text.tmk_exception_common) == 105,
       "common-entry size changed")
ASSERT(ADDR(.text.tmk_exception_dispatcher) == 0xffffffff80011100,
       "dispatcher-front address changed")
ASSERT(SIZEOF(.text.tmk_exception_dispatcher) == 93,
       "dispatcher-front size changed")
ASSERT(tmk_exception_scalar == 0xffffffff80011200,
       "scalar seam address changed")
