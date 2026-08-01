ENTRY(tmk_exception_scalar_entry)

SECTIONS
{
  . = 0xffffffff80001040;
  .text.tmk_per_cpu_gs_setup : ALIGN(1)
  {
    tmk_per_cpu_gs_setup = .;
    KEEP(*(.text.tmk_per_cpu_gs_setup))
  }

  . = 0xffffffff80011200;
  .text.tmk_exception_scalar_entry : ALIGN(1)
  {
    tmk_exception_scalar_entry = .;
    KEEP(*(.text.tmk_exception_scalar_entry))
  }

  . = 0xffffffff80011300;
  .text.tmk_exception_scalar_wrapper : ALIGN(1)
  {
    tmk_exception_scalar_wrapper = .;
    KEEP(*(.text.tmk_exception_scalar_wrapper))
  }

  . = 0xffffffff80011500;
  .text.tmk_exception_fail_stop : ALIGN(1)
  {
    tmk_exception_fail_stop = .;
    KEEP(*(.text.tmk_exception_fail_stop))
  }
  rust_eh_personality = tmk_exception_fail_stop;

  . = 0xffffffff80011600;
  .text.tmk_exception_schedule_unavailable : ALIGN(1)
  {
    tmk_exception_schedule = .;
    KEEP(*(.text.tmk_exception_schedule_unavailable))
  }

  . = 0xffffffff80012000;
  .text.tmk_exception_scalar_adapter : ALIGN(1)
  {
    tmk_exception_scalar_adapter = .;
    KEEP(*(.text.*tmk_exception_scalar_adapter))
  }

  . = ALIGN(16);
  .text.tmk_memcpy_capsule : ALIGN(1)
  {
    KEEP(*(.text.tmk_memcpy_capsule))
  }

  . = ALIGN(16);
  .text.tmk_exception_scalar_runtime :
  {
    *(.text .text.*)
  }
  .rodata : { *(.rodata .rodata.*) }
  .gcc_except_table : { *(.gcc_except_table .gcc_except_table.*) }
  .data.rel.ro : { *(.data.rel.ro .data.rel.ro.*) }
  .got : { *(.got .got.*) }
  .data : { *(.data .data.*) }
  .bss (NOLOAD) : { *(.bss .bss.* COMMON) }

  /DISCARD/ :
  {
    *(.comment)
    *(.note .note.*)
    *(.eh_frame .eh_frame_hdr)
  }
}

ASSERT(ADDR(.text.tmk_per_cpu_gs_setup) == 0xffffffff80001040,
       "GS setup virtual address drift")
ASSERT(SIZEOF(.text.tmk_per_cpu_gs_setup) == 35,
       "GS setup size drift")
ASSERT(ADDR(.text.tmk_exception_scalar_entry) == 0xffffffff80011200,
       "scalar entry virtual address drift")
ASSERT(SIZEOF(.text.tmk_exception_scalar_entry) == 11,
       "scalar entry size drift")
ASSERT(ADDR(.text.tmk_exception_scalar_wrapper) == 0xffffffff80011300,
       "scalar wrapper virtual address drift")
ASSERT(SIZEOF(.text.tmk_exception_scalar_wrapper) == 314,
       "scalar wrapper size drift")
ASSERT(ADDR(.text.tmk_exception_fail_stop) == 0xffffffff80011500,
       "fail-stop virtual address drift")
ASSERT(SIZEOF(.text.tmk_exception_fail_stop) == 4,
       "fail-stop size drift")
ASSERT(ADDR(.text.tmk_exception_schedule_unavailable) == 0xffffffff80011600,
       "schedule stub virtual address drift")
ASSERT(SIZEOF(.text.tmk_exception_schedule_unavailable) == 5,
       "schedule stub size drift")
ASSERT(ADDR(.text.tmk_exception_scalar_adapter) == 0xffffffff80012000,
       "scalar adapter virtual address drift")
ASSERT(SIZEOF(.text.tmk_exception_scalar_adapter) == 0x75d,
       "scalar adapter size drift")
ASSERT(SIZEOF(.text.tmk_memcpy_capsule) == 9,
       "verified memcpy size drift")
