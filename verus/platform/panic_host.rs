#![no_std]
#![crate_type = "rlib"]

use verus_builtin::*;
use verus_builtin_macros::*;

verus! {

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExPanicInfo<'a>(core::panic::PanicInfo<'a>);

#[panic_handler]
#[verifier::exec_allows_no_decreases_clause]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { }
}

}
