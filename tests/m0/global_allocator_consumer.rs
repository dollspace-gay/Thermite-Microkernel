#![no_std]
#![no_main]

extern crate alloc;
extern crate tmk_global_allocator;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::alloc::Layout;
use core::panic::PanicInfo;

unsafe extern "C" {
    fn memcpy(destination: *mut u8, source: *const u8, length: usize) -> *mut u8;
    fn memset(destination: *mut u8, value: i32, length: usize) -> *mut u8;
    fn write(fd: i32, buffer: *const u8, length: usize) -> isize;
    fn _exit(status: i32) -> !;
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_eh_personality() {}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let boxed = Box::new(0x544d_4b31_u64);
    assert_eq!(*boxed, 0x544d_4b31);

    let mut values = Vec::with_capacity(4);
    values.push(3_u64);
    values.push(5_u64);
    values.push(8_u64);
    values.push(13_u64);
    assert_eq!(values.as_slice(), &[3, 5, 8, 13]);
    assert_ne!(
        (&*boxed as *const u64).cast::<u8>(),
        values.as_ptr().cast::<u8>()
    );

    let unsupported = unsafe { alloc::alloc::alloc(Layout::from_size_align(8, 128).unwrap()) };
    assert!(unsupported.is_null());

    tmk_global_allocator::tmk_global_alloc_seal();
    let sealed = unsafe { alloc::alloc::alloc(Layout::from_size_align(8, 8).unwrap()) };
    assert!(sealed.is_null());

    let source = [1_u8, 2, 3, 5, 8, 13, 21, 34];
    let mut destination = [0_u8; 8];
    let copied = unsafe { memcpy(destination.as_mut_ptr(), source.as_ptr(), source.len()) };
    assert_eq!(copied, destination.as_mut_ptr());
    assert_eq!(destination, source);
    let set = unsafe { memset(destination.as_mut_ptr(), 0x5a, destination.len()) };
    assert_eq!(set, destination.as_mut_ptr());
    assert_eq!(destination, [0x5a; 8]);

    const MARKER: &[u8] = b"M0_GLOBAL_ALLOC_OK:box:vec:reject:sealed\n";
    let written = unsafe { write(1, MARKER.as_ptr(), MARKER.len()) };
    unsafe {
        _exit(if written == MARKER.len() as isize {
            0
        } else {
            71
        })
    }
}
