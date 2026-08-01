extern crate tmk_exception_scalar;

use tmk_exception_scalar::exception_scalar_shell::{
    scalar_dispatch_checked, DispatchContext, MachineState, PerCpuSnapshot, ScalarArguments,
    CONTROL_FAIL_STOP, CONTROL_RETURN, CONTROL_SCHEDULE, KERNEL_CODE_SELECTOR,
    USER_CODE_SELECTOR, USER_DATA_SELECTOR,
};
use tmk_exception_scalar::ExceptionState;

fn state() -> ExceptionState {
    ExceptionState {
        fault_generation: 0,
        timer_expiries: 0,
        irq_deliveries: 0,
        quarantined_vectors: 0,
        spurious_vectors: 0,
        last_tlb_epoch: 0,
        reschedule_pending: false,
        panic_latched: false,
    }
}

fn context() -> DispatchContext {
    DispatchContext {
        thread: 42,
        thread_live: true,
        fault_endpoint_valid: true,
        vspace_epoch: 77,
        irq_bound: false,
        acknowledge_required: false,
        wakes_higher_priority: false,
        shootdown_epoch: 0,
    }
}

fn cpu() -> PerCpuSnapshot {
    PerCpuSnapshot {
        cpu_id: 0,
        lock_held: true,
        unique_state_token: true,
        interrupts_masked: true,
        current_thread: 42,
        fault_slot_ready: true,
        irq_backend_ready: true,
        tlb_backend_ready: true,
        scheduler_ready: true,
        crash_record_ready: true,
    }
}

fn machine() -> MachineState {
    MachineState {
        current_thread_state: 0,
        fault_generation: 0,
        fault_thread: 0,
        fault_vector: 0,
        fault_error: 0,
        fault_address: 0,
        fault_access: 0,
        fault_vspace_epoch: 0,
        timer_expiries: 0,
        reschedule_pending: false,
        irq_masked_vector: 0,
        notification_vector: 0,
        irq_acknowledgements: 0,
        tlb_epoch: 0,
        tlb_acknowledgements: 0,
        quarantined_vector: 0,
        spurious_count: 0,
        crash_latched: false,
        crash_reason: 0,
    }
}

fn kernel_frame(vector: u64, error: u64, cr2: u64) -> [u64; 21] {
    [
        15, 14, 13, 12, 11, 10, 9, 8, 0xb0, 0xd1, 0x51, 0xd2, 0xc1, 0xb3, cr2, 0xa0,
        vector, error, 0xffff_ffff_8000_2000, KERNEL_CODE_SELECTOR, 0x202,
    ]
}

fn user_frame(vector: u64, error: u64, cr2: u64) -> [u64; 23] {
    [
        15,
        14,
        13,
        12,
        11,
        10,
        9,
        8,
        0xb0,
        0xd1,
        0x51,
        0xd2,
        0xc1,
        0xb3,
        cr2,
        0xa0,
        vector,
        error,
        0x0040_1000,
        USER_CODE_SELECTOR,
        0x202,
        0x0000_7fff_ffff_e000,
        USER_DATA_SELECTOR,
    ]
}

fn args(words: &[u64]) -> ScalarArguments {
    let user = words.len() == 23;
    ScalarArguments {
        cr2: words[14],
        error: words[17],
        rip: words[18],
        rflags: words[20],
        user_rsp: if user { words[21] } else { 0 },
        metadata: words[16]
            | (words[19] << 32)
            | if user { words[22] << 48 } else { 0 },
    }
}

fn main() {
    let mut observation = 0u64;

    let page_frame = user_frame(14, 6, 0x1234_5000);
    let page = scalar_dispatch_checked(
        state(),
        &page_frame,
        &args(&page_frame),
        &context(),
        &cpu(),
        machine(),
    );
    assert_eq!(page.control, CONTROL_SCHEDULE);
    assert_eq!(page.action_code, 1);
    assert!(page.bridge_valid && page.policy_invoked && page.action_committed);
    assert_eq!(page.machine.current_thread_state, 1);
    assert_eq!(page.machine.fault_generation, 1);
    assert_eq!(page.machine.fault_thread, 42);
    assert_eq!(page.machine.fault_vector, 14);
    assert_eq!(page.machine.fault_error, 6);
    assert_eq!(page.machine.fault_address, 0x1234_5000);
    assert_eq!(page.machine.fault_access, 1);
    assert_eq!(page.machine.fault_vspace_epoch, 77);
    observation |= 1;

    let timer_frame = kernel_frame(0xe0, 0, 0);
    let timer = scalar_dispatch_checked(
        state(),
        &timer_frame,
        &args(&timer_frame),
        &context(),
        &cpu(),
        machine(),
    );
    assert_eq!(timer.control, CONTROL_SCHEDULE);
    assert_eq!(timer.action_code, 3);
    assert_eq!(timer.machine.timer_expiries, 1);
    assert!(timer.machine.reschedule_pending);
    observation |= 2;

    let terminate_frame = user_frame(13, 0, 0);
    let mut no_endpoint = context();
    no_endpoint.fault_endpoint_valid = false;
    let terminate = scalar_dispatch_checked(
        state(),
        &terminate_frame,
        &args(&terminate_frame),
        &no_endpoint,
        &cpu(),
        machine(),
    );
    assert_eq!(terminate.control, CONTROL_SCHEDULE);
    assert_eq!(terminate.action_code, 2);
    assert_eq!(terminate.machine.current_thread_state, 2);
    observation |= 4;

    let irq_frame = kernel_frame(0x40, 0, 0);
    let mut irq_context = context();
    irq_context.irq_bound = true;
    irq_context.acknowledge_required = true;
    let irq = scalar_dispatch_checked(
        state(),
        &irq_frame,
        &args(&irq_frame),
        &irq_context,
        &cpu(),
        machine(),
    );
    assert_eq!(irq.control, CONTROL_RETURN);
    assert_eq!(irq.action_code, 5);
    assert_eq!(irq.machine.irq_masked_vector, 0x40);
    assert_eq!(irq.machine.notification_vector, 0x40);
    assert_eq!(irq.machine.irq_acknowledgements, 1);
    observation |= 8;

    let tlb_frame = kernel_frame(0xe2, 0, 0);
    let mut tlb_context = context();
    tlb_context.shootdown_epoch = 9;
    tlb_context.acknowledge_required = true;
    let tlb = scalar_dispatch_checked(
        state(),
        &tlb_frame,
        &args(&tlb_frame),
        &tlb_context,
        &cpu(),
        machine(),
    );
    assert_eq!(tlb.control, CONTROL_RETURN);
    assert_eq!(tlb.action_code, 6);
    assert_eq!(tlb.machine.tlb_epoch, 9);
    assert_eq!(tlb.machine.tlb_acknowledgements, 1);
    observation |= 16;

    let quarantine_frame = kernel_frame(0x21, 0, 0);
    let quarantine = scalar_dispatch_checked(
        state(),
        &quarantine_frame,
        &args(&quarantine_frame),
        &context(),
        &cpu(),
        machine(),
    );
    assert_eq!(quarantine.control, CONTROL_RETURN);
    assert_eq!(quarantine.action_code, 8);
    assert_eq!(quarantine.machine.quarantined_vector, 0x21);
    observation |= 32;

    let panic_frame = kernel_frame(14, 0, 0x3000);
    let panic = scalar_dispatch_checked(
        state(),
        &panic_frame,
        &args(&panic_frame),
        &context(),
        &cpu(),
        machine(),
    );
    assert_eq!(panic.control, CONTROL_FAIL_STOP);
    assert_eq!(panic.action_code, 10);
    assert!(panic.machine.crash_latched);
    assert_eq!(panic.machine.crash_reason, 5);
    observation |= 64;

    let mut bad_cpu = cpu();
    bad_cpu.lock_held = false;
    let bad_lock = scalar_dispatch_checked(
        state(),
        &page_frame,
        &args(&page_frame),
        &context(),
        &bad_cpu,
        machine(),
    );
    assert_eq!(bad_lock.control, CONTROL_FAIL_STOP);
    assert!(!bad_lock.bridge_valid && !bad_lock.policy_invoked);
    assert_eq!(bad_lock.machine.crash_reason, 100);
    observation |= 128;

    let mut bad_args = args(&page_frame);
    bad_args.cr2 ^= 1;
    let mismatch = scalar_dispatch_checked(
        state(),
        &page_frame,
        &bad_args,
        &context(),
        &cpu(),
        machine(),
    );
    assert_eq!(mismatch.control, CONTROL_FAIL_STOP);
    assert!(!mismatch.bridge_valid && !mismatch.policy_invoked);
    assert_eq!(mismatch.machine.crash_reason, 101);
    observation |= 256;

    let mut no_irq = cpu();
    no_irq.irq_backend_ready = false;
    let backend = scalar_dispatch_checked(
        state(),
        &irq_frame,
        &args(&irq_frame),
        &irq_context,
        &no_irq,
        machine(),
    );
    assert_eq!(backend.control, CONTROL_FAIL_STOP);
    assert!(backend.bridge_valid && backend.policy_invoked);
    assert!(!backend.action_committed);
    assert_eq!(backend.machine.crash_reason, 103);
    assert_eq!(backend.policy_state.irq_deliveries, 0);
    assert!(backend.policy_state.panic_latched);
    observation |= 512;

    let mut overflow_machine = machine();
    overflow_machine.tlb_acknowledgements = u64::MAX;
    let overflow = scalar_dispatch_checked(
        state(),
        &tlb_frame,
        &args(&tlb_frame),
        &tlb_context,
        &cpu(),
        overflow_machine,
    );
    assert_eq!(overflow.control, CONTROL_FAIL_STOP);
    assert_eq!(overflow.machine.crash_reason, 103);
    assert!(!overflow.action_committed);
    assert_eq!(overflow.policy_state.last_tlb_epoch, 0);
    assert!(overflow.policy_state.panic_latched);
    observation |= 1024;

    assert_eq!(observation, 2047);
    println!(
        "M1_EXCEPTION_SCALAR_OK scenarios=11 observation={observation} controls=return,schedule,fail-stop actions=fault,terminate,timer,irq,tlb,quarantine,panic"
    );
}
