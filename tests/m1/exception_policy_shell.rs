fn base_exception_state() -> (result: ExceptionState)
    ensures
        result.fault_generation == 0,
        result.timer_expiries == 0,
        result.irq_deliveries == 0,
        result.quarantined_vectors == 0,
        result.spurious_vectors == 0,
        result.last_tlb_epoch == 0,
        !result.reschedule_pending,
        !result.panic_latched,
{
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

fn exception_event(
    vector: u32,
    error: u64,
    from_user: bool,
    frame_valid: bool,
    thread: u64,
    thread_live: bool,
    fault_endpoint_valid: bool,
    cr2: u64,
    irq_bound: bool,
    acknowledge_required: bool,
    wakes_higher_priority: bool,
    shootdown_epoch: u64,
) -> (result: ExceptionEvent)
    ensures
        result.vector == vector,
        result.error == error,
        result.from_user == from_user,
        result.frame_valid == frame_valid,
        result.thread == thread,
        result.thread_live == thread_live,
        result.fault_endpoint_valid == fault_endpoint_valid,
        result.cr2 == cr2,
        result.vspace_epoch == 77,
        result.irq_bound == irq_bound,
        result.acknowledge_required == acknowledge_required,
        result.wakes_higher_priority == wakes_higher_priority,
        result.shootdown_epoch == shootdown_epoch,
{
    ExceptionEvent {
        vector,
        error,
        from_user,
        frame_valid,
        thread,
        thread_live,
        fault_endpoint_valid,
        cr2,
        vspace_epoch: 77,
        irq_bound,
        acknowledge_required,
        wakes_higher_priority,
        shootdown_epoch,
    }
}

pub fn exception_policy_observation() -> (result: u64)
    ensures result == 262143,
{
    assert((6u64 & 0xffff_ffff_ffff_ffe8u64) == 0u64) by(bit_vector);
    assert((6u64 & 4u64) == 4u64) by(bit_vector);
    assert((6u64 & 16u64) != 16u64) by(bit_vector);
    assert((6u64 & 2u64) == 2u64) by(bit_vector);
    assert((12u64 & 0xffff_ffff_ffff_ffe8u64) != 0u64) by(bit_vector);
    let page_fault = exception_policy_step(
        base_exception_state(),
        exception_event(14, 6, true, true, 42, true, true, 0x1234_5000,
                        false, false, false, 0),
    );
    match page_fault.1 {
        ExceptionAction::DeliverFault {
            generation,
            thread,
            kind,
            vector,
            error,
            address,
            access,
            vspace_epoch,
        } => {
            assert(generation == 1);
            assert(thread == 42);
            assert(kind == 1);
            assert(vector == 14);
            assert(error == 6);
            assert(address == 0x1234_5000);
            assert(access == 1);
            assert(vspace_epoch == 77);
        },
        _ => assert(false),
    }
    assert(page_fault.0.fault_generation == 1);
    assert(!page_fault.0.panic_latched);

    let terminate = exception_policy_step(
        base_exception_state(),
        exception_event(13, 0, true, true, 42, true, false, 0,
                        false, false, false, 0),
    );
    match terminate.1 {
        ExceptionAction::TerminateThread { thread, vector } => {
            assert(thread == 42);
            assert(vector == 13);
        },
        _ => assert(false),
    }

    let corrupt_page_fault = exception_policy_step(
        base_exception_state(),
        exception_event(14, 12, true, true, 42, true, true, 0x2000,
                        false, false, false, 0),
    );
    match corrupt_page_fault.1 {
        ExceptionAction::Panic { reason } => assert(reason == 6),
        _ => assert(false),
    }

    let kernel_page_fault = exception_policy_step(
        base_exception_state(),
        exception_event(14, 0, false, true, 42, true, true, 0x3000,
                        false, false, false, 0),
    );
    match kernel_page_fault.1 {
        ExceptionAction::Panic { reason } => assert(reason == 5),
        _ => assert(false),
    }

    let double_fault = exception_policy_step(
        base_exception_state(),
        exception_event(8, 0, false, true, 42, true, true, 0,
                        false, false, false, 0),
    );
    match double_fault.1 {
        ExceptionAction::Panic { reason } => assert(reason == 4),
        _ => assert(false),
    }

    let timer = exception_policy_step(
        base_exception_state(),
        exception_event(0xe0, 0, false, true, 42, true, true, 0,
                        false, true, false, 0),
    );
    match timer.1 {
        ExceptionAction::TimerRecorded { expiries } => assert(expiries == 1),
        _ => assert(false),
    }
    assert(timer.0.timer_expiries == 1);
    assert(timer.0.reschedule_pending);

    let reschedule = exception_policy_step(
        base_exception_state(),
        exception_event(0xe1, 0, false, true, 42, true, true, 0,
                        false, true, false, 0),
    );
    match reschedule.1 {
        ExceptionAction::Reschedule => {},
        _ => assert(false),
    }
    assert(reschedule.0.reschedule_pending);

    let irq = exception_policy_step(
        base_exception_state(),
        exception_event(0x40, 0, false, true, 42, true, true, 0,
                        true, true, true, 0),
    );
    match irq.1 {
        ExceptionAction::NotifyIrq { vector, masked, acknowledge, reschedule } => {
            assert(vector == 0x40);
            assert(masked);
            assert(acknowledge);
            assert(reschedule);
        },
        _ => assert(false),
    }
    assert(irq.0.irq_deliveries == 1);

    let unbound_irq = exception_policy_step(
        base_exception_state(),
        exception_event(0x40, 0, false, true, 42, true, true, 0,
                        false, true, false, 0),
    );
    match unbound_irq.1 {
        ExceptionAction::Quarantine { vector, acknowledge } => {
            assert(vector == 0x40);
            assert(acknowledge);
        },
        _ => assert(false),
    }
    assert(unbound_irq.0.quarantined_vectors == 1);

    let shootdown = exception_policy_step(
        base_exception_state(),
        exception_event(0xe2, 0, false, true, 42, true, true, 0,
                        false, true, false, 3),
    );
    match shootdown.1 {
        ExceptionAction::TlbShootdown { epoch, acknowledge } => {
            assert(epoch == 3);
            assert(acknowledge);
        },
        _ => assert(false),
    }
    assert(shootdown.0.last_tlb_epoch == 3);
    let stale_shootdown = exception_policy_step(
        shootdown.0,
        exception_event(0xe2, 0, false, true, 42, true, true, 0,
                        false, true, false, 2),
    );
    match stale_shootdown.1 {
        ExceptionAction::StaleTlbShootdown { epoch, acknowledge } => {
            assert(epoch == 2);
            assert(acknowledge);
        },
        _ => assert(false),
    }
    assert(stale_shootdown.0.last_tlb_epoch == 3);

    let stop = exception_policy_step(
        base_exception_state(),
        exception_event(0xe3, 0, false, true, 42, true, true, 0,
                        false, true, false, 0),
    );
    match stop.1 {
        ExceptionAction::Panic { reason } => assert(reason == 9),
        _ => assert(false),
    }

    let spurious = exception_policy_step(
        base_exception_state(),
        exception_event(0xff, 0, false, true, 42, true, true, 0,
                        false, false, false, 0),
    );
    match spurious.1 {
        ExceptionAction::Spurious { count } => assert(count == 1),
        _ => assert(false),
    }

    let bad_frame = exception_policy_step(
        base_exception_state(),
        exception_event(14, 6, true, false, 42, true, true, 0x4000,
                        false, false, false, 0),
    );
    match bad_frame.1 {
        ExceptionAction::Panic { reason } => assert(reason == 2),
        _ => assert(false),
    }

    let bad_vector = exception_policy_step(
        base_exception_state(),
        exception_event(256, 0, false, true, 42, true, true, 0,
                        false, false, false, 0),
    );
    match bad_vector.1 {
        ExceptionAction::Panic { reason } => assert(reason == 3),
        _ => assert(false),
    }

    let missing_thread = exception_policy_step(
        base_exception_state(),
        exception_event(13, 0, true, true, 0, false, true, 0,
                        false, false, false, 0),
    );
    match missing_thread.1 {
        ExceptionAction::Panic { reason } => assert(reason == 7),
        _ => assert(false),
    }

    let overflow_state = ExceptionState {
        fault_generation: 0,
        timer_expiries: 0xffff_ffff_ffff_fffe,
        irq_deliveries: 0,
        quarantined_vectors: 0,
        spurious_vectors: 0,
        last_tlb_epoch: 0,
        reschedule_pending: true,
        panic_latched: false,
    };
    let overflow = exception_policy_step(
        overflow_state,
        exception_event(0xe0, 0, false, true, 42, true, true, 0,
                        false, true, false, 0),
    );
    match overflow.1 {
        ExceptionAction::Panic { reason } => assert(reason == 8),
        _ => assert(false),
    }
    assert(overflow.0.panic_latched);
    assert(!overflow.0.reschedule_pending);

    let latched = exception_policy_step(
        overflow.0,
        exception_event(0x40, 0, false, true, 42, true, true, 0,
                        true, true, true, 0),
    );
    match latched.1 {
        ExceptionAction::Panic { reason } => assert(reason == 1),
        _ => assert(false),
    }

    262143
}
