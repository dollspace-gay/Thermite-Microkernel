ENTRY(tmk_descriptor_install_capsule)

SECTIONS
{
  . = 0xffffffff80001010;
  .text.tmk_descriptor_install_capsule : ALIGN(16)
  {
    tmk_descriptor_install_capsule_link_start = .;
    KEEP(*(.text.tmk_descriptor_install_capsule))
    tmk_descriptor_install_capsule_link_end = .;
  }

  /DISCARD/ :
  {
    *(.comment)
    *(.note*)
  }
}

ASSERT(SIZEOF(.text.tmk_descriptor_install_capsule) == 38,
       "M1 descriptor-install capsule must be exactly 38 bytes")
ASSERT(tmk_descriptor_install_capsule == tmk_descriptor_install_capsule_link_start,
       "M1 descriptor-install capsule entry must equal section start")
