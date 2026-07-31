OUTPUT_FORMAT(pei-x86-64)
ENTRY(efi_main)

SECTIONS
{
  . = __image_base__ + 0x1000;
  .text : ALIGN(16)
  {
    efi_entry_start = .;
    KEEP(*(.text))
    efi_entry_end = .;
  }

  /DISCARD/ :
  {
    *(*)
  }
}

ASSERT((efi_entry_end - efi_entry_start) == 56,
       "M0 UEFI entry capsule must be exactly 56 bytes")
ASSERT(efi_main == efi_entry_start,
       "M0 UEFI entry must equal capsule start")
