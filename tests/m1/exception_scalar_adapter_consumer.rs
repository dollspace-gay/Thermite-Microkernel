extern crate tmk_exception_scalar;

use std::mem::{align_of, offset_of, size_of};
use tmk_exception_scalar::exception_scalar_shell::{
    tmk_exception_scalar_adapter, ScalarCoreBlock, CONTROL_FAIL_STOP, CONTROL_SCHEDULE,
};

fn empty_block() -> ScalarCoreBlock {
    ScalarCoreBlock {
        slot_00: 0,
        slot_01: 0,
        slot_02: 0,
        slot_03: 0,
        slot_04: 0,
        slot_05: 0,
        slot_06: 0,
        slot_07: 0,
        slot_08: 0,
        slot_09: 0,
        slot_10: 0,
        slot_11: 0,
        slot_12: 0,
        slot_13: 0,
        slot_14: 0,
        slot_15: 0,
        slot_16: 0,
        slot_17: 0,
        slot_18: 0,
        slot_19: 0,
        slot_20: 0,
        slot_21: 0,
        slot_22: 0,
        slot_23: 0,
        slot_24: 0,
        slot_25: 0,
        slot_26: 0,
        slot_27: 0,
        slot_28: 0,
        slot_29: 0,
        slot_30: 0,
        slot_31: 0,
        slot_32: 0,
        slot_33: 0,
        slot_34: 0,
        slot_35: 0,
        slot_36: 0,
        slot_37: 0,
        slot_38: 0,
        slot_39: 0,
        slot_40: 0,
        slot_41: 0,
        slot_42: 0,
        slot_43: 0,
        slot_44: 0,
        slot_45: 0,
        slot_46: 0,
        slot_47: 0,
        slot_48: 0,
        slot_49: 0,
        slot_50: 0,
        slot_51: 0,
        slot_52: 0,
        slot_53: 0,
        slot_54: 0,
        slot_55: 0,
        slot_56: 0,
        slot_57: 0,
        slot_58: 0,
        slot_59: 0,
        slot_60: 0,
        slot_61: 0,
        slot_62: 0,
        slot_63: 0,
        slot_64: 0,
        slot_65: 0,
        slot_66: 0,
        slot_67: 0,
        slot_68: 0,
        slot_69: 0,
        slot_70: 0,
        slot_71: 0,
        slot_72: 0,
        slot_73: 0,
        slot_74: 0,
        slot_75: 0,
        slot_76: 0,
        slot_77: 0,
        slot_78: 0,
        slot_79: 0,
    }
}

fn page_fault_block() -> ScalarCoreBlock {
    let mut block = empty_block();
    block.slot_14 = 0x1234_5000;
    block.slot_16 = 14;
    block.slot_17 = 6;
    block.slot_18 = 0x0040_1000;
    block.slot_19 = 0x23;
    block.slot_20 = 0x202;
    block.slot_21 = 0x0000_7fff_ffff_e000;
    block.slot_22 = 0x1b;
    block.slot_23 = 23;
    block.slot_24 = 0x1234_5000;
    block.slot_25 = 6;
    block.slot_26 = 0x0040_1000;
    block.slot_27 = 0x202;
    block.slot_28 = 0x0000_7fff_ffff_e000;
    block.slot_29 = 0x001b_0023_0000_000e;
    block.slot_30 = 42;
    block.slot_31 = 1;
    block.slot_32 = 1;
    block.slot_33 = 77;
    block.slot_38 = 0;
    block.slot_39 = 1;
    block.slot_40 = 1;
    block.slot_41 = 1;
    block.slot_42 = 42;
    block.slot_43 = 1;
    block.slot_44 = 1;
    block.slot_45 = 1;
    block.slot_46 = 1;
    block.slot_47 = 1;
    block
}

fn main() {
    assert_eq!(size_of::<ScalarCoreBlock>(), 640);
    assert_eq!(align_of::<ScalarCoreBlock>(), 8);
    assert_eq!(offset_of!(ScalarCoreBlock, slot_00), 0);
    assert_eq!(offset_of!(ScalarCoreBlock, slot_14), 112);
    assert_eq!(offset_of!(ScalarCoreBlock, slot_23), 184);
    assert_eq!(offset_of!(ScalarCoreBlock, slot_24), 192);
    assert_eq!(offset_of!(ScalarCoreBlock, slot_48), 384);
    assert_eq!(offset_of!(ScalarCoreBlock, slot_75), 600);
    assert_eq!(offset_of!(ScalarCoreBlock, slot_79), 632);

    let mut page = page_fault_block();
    let page_control = tmk_exception_scalar_adapter(&mut page);
    assert_eq!(page_control, CONTROL_SCHEDULE);
    assert_eq!(page.slot_48, 1);
    assert_eq!(page.slot_54, 0);
    assert_eq!(page.slot_56, 1);
    assert_eq!(page.slot_57, 1);
    assert_eq!(page.slot_58, 42);
    assert_eq!(page.slot_59, 14);
    assert_eq!(page.slot_60, 6);
    assert_eq!(page.slot_61, 0x1234_5000);
    assert_eq!(page.slot_62, 1);
    assert_eq!(page.slot_63, 77);
    assert_eq!(page.slot_65, 1);
    assert_eq!(page.slot_75, CONTROL_SCHEDULE as u64);
    assert_eq!(page.slot_76, 1);
    assert_eq!(page.slot_77, 1);
    assert_eq!(page.slot_78, 1);
    assert_eq!(page.slot_79, 1);

    let mut mismatch = page_fault_block();
    mismatch.slot_24 = 0x9999_9000;
    let mismatch_control = tmk_exception_scalar_adapter(&mut mismatch);
    assert_eq!(mismatch_control, CONTROL_FAIL_STOP);
    assert_eq!(mismatch.slot_55, 1);
    assert_eq!(mismatch.slot_73, 1);
    assert_eq!(mismatch.slot_74, 101);
    assert_eq!(mismatch.slot_75, CONTROL_FAIL_STOP as u64);
    assert_eq!(mismatch.slot_76, 0);
    assert_eq!(mismatch.slot_77, 0);
    assert_eq!(mismatch.slot_78, 0);
    assert_eq!(mismatch.slot_79, 0);

    let mut bad_snapshot = page_fault_block();
    bad_snapshot.slot_40 = 0;
    let snapshot_control = tmk_exception_scalar_adapter(&mut bad_snapshot);
    assert_eq!(snapshot_control, CONTROL_FAIL_STOP);
    assert_eq!(bad_snapshot.slot_73, 1);
    assert_eq!(bad_snapshot.slot_74, 100);
    assert_eq!(bad_snapshot.slot_77, 0);
    assert_eq!(bad_snapshot.slot_79, 0);

    println!(
        "M1_EXCEPTION_SCALAR_ADAPTER_OK layout=640 offsets=0,112,184,192,384,600,632 scenarios=page-fault,mismatch,bad-snapshot"
    );
}
