#![no_std]
#![crate_type = "rlib"]

use core::alloc::{GlobalAlloc, Layout};

const BOOT_ARENA_BYTES: usize = 65_536;

#[repr(C, align(4096))]
struct BootArena {
    bytes: [u8; BOOT_ARENA_BYTES],
    cursor: usize,
    sealed: usize,
}

#[unsafe(no_mangle)]
static mut TMK_BOOT_ARENA: BootArena = BootArena {
    bytes: [0; BOOT_ARENA_BYTES],
    cursor: 0,
    sealed: 0,
};

unsafe extern "C" {
    fn tmk_alloc_capsule(arena: *mut BootArena, size: usize, align: usize) -> *mut u8;
    fn tmk_seal_capsule(arena: *mut BootArena);
}

struct TmkBootAllocator;

unsafe impl GlobalAlloc for TmkBootAllocator {
    #[inline(never)]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { tmk_alloc_capsule(&raw mut TMK_BOOT_ARENA, layout.size(), layout.align()) }
    }

    #[inline(never)]
    unsafe fn dealloc(&self, _pointer: *mut u8, _layout: Layout) {}

    #[inline(never)]
    unsafe fn alloc_zeroed(&self, _layout: Layout) -> *mut u8 {
        core::ptr::null_mut()
    }

    #[inline(never)]
    unsafe fn realloc(&self, _pointer: *mut u8, _layout: Layout, _new_size: usize) -> *mut u8 {
        core::ptr::null_mut()
    }
}

#[global_allocator]
static TMK_GLOBAL_ALLOCATOR: TmkBootAllocator = TmkBootAllocator;

#[unsafe(no_mangle)]
pub extern "C" fn tmk_global_alloc_seal() {
    unsafe { tmk_seal_capsule(&raw mut TMK_BOOT_ARENA) }
}
