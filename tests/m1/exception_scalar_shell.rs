use vstd::prelude::*;

pub const EXCEPTION_PREFIX_WORDS: usize = 21;
pub const EXCEPTION_USER_WORDS: usize = 23;
pub const KERNEL_CODE_SELECTOR: u64 = 0x08;
pub const USER_DATA_SELECTOR: u64 = 0x1b;
pub const USER_CODE_SELECTOR: u64 = 0x23;
pub const RETURN_RFLAGS_ALLOWED: u64 = 0x0025_0fd7;
pub const CONTROL_RETURN: u32 = 0;
pub const CONTROL_SCHEDULE: u32 = 1;
pub const CONTROL_FAIL_STOP: u32 = 2;

pub struct ScalarArguments {
    pub cr2: u64,
    pub error: u64,
    pub rip: u64,
    pub rflags: u64,
    pub user_rsp: u64,
    pub metadata: u64,
}

pub struct DispatchContext {
    pub thread: u64,
    pub thread_live: bool,
    pub fault_endpoint_valid: bool,
    pub vspace_epoch: u64,
    pub irq_bound: bool,
    pub acknowledge_required: bool,
    pub wakes_higher_priority: bool,
    pub shootdown_epoch: u64,
}

pub struct PerCpuSnapshot {
    pub cpu_id: u32,
    pub lock_held: bool,
    pub unique_state_token: bool,
    pub interrupts_masked: bool,
    pub current_thread: u64,
    pub fault_slot_ready: bool,
    pub irq_backend_ready: bool,
    pub tlb_backend_ready: bool,
    pub scheduler_ready: bool,
    pub crash_record_ready: bool,
}

pub struct MachineState {
    pub current_thread_state: u32,
    pub fault_generation: u64,
    pub fault_thread: u64,
    pub fault_vector: u32,
    pub fault_error: u64,
    pub fault_address: u64,
    pub fault_access: u32,
    pub fault_vspace_epoch: u64,
    pub timer_expiries: u64,
    pub reschedule_pending: bool,
    pub irq_masked_vector: u32,
    pub notification_vector: u32,
    pub irq_acknowledgements: u64,
    pub tlb_epoch: u64,
    pub tlb_acknowledgements: u64,
    pub quarantined_vector: u32,
    pub spurious_count: u64,
    pub crash_latched: bool,
    pub crash_reason: u32,
}

pub struct ScalarOutcome {
    pub policy_state: ExceptionState,
    pub machine: MachineState,
    pub control: u32,
    pub action_code: u32,
    pub bridge_valid: bool,
    pub policy_invoked: bool,
    pub action_committed: bool,
}

pub open spec fn spec_kernel_address(address: u64) -> bool {
    address >= 0xffff_8000_0000_0000
}

pub open spec fn spec_user_address(address: u64) -> bool {
    address <= 0x0000_7fff_ffff_ffff
}

pub open spec fn spec_return_flags(flags: u64) -> bool {
    flags & 2 == 2 && flags & !RETURN_RFLAGS_ALLOWED == 0
}

pub open spec fn spec_exception_frame_valid(words: &[u64]) -> bool {
    words.len() == EXCEPTION_PREFIX_WORDS
        && words@[16] <= 255
        && words@[19] == KERNEL_CODE_SELECTOR
        && spec_kernel_address(words@[18])
        && spec_return_flags(words@[20])
        || words.len() == EXCEPTION_USER_WORDS
            && words@[16] <= 255
            && words@[19] == USER_CODE_SELECTOR
            && spec_user_address(words@[18])
            && spec_return_flags(words@[20])
            && spec_user_address(words@[21])
            && words@[22] == USER_DATA_SELECTOR
}

pub open spec fn spec_scalar_arguments_match(words: &[u64], args: &ScalarArguments) -> bool {
    spec_exception_frame_valid(words)
        && args.cr2 == words@[14]
        && args.error == words@[17]
        && args.rip == words@[18]
        && args.rflags == words@[20]
        && (args.metadata & 0xffff_ffff) == words@[16]
        && ((args.metadata >> 32) & 0xffff) == words@[19]
        && (words.len() == EXCEPTION_USER_WORDS
            ==> args.user_rsp == words@[21]
                && ((args.metadata >> 48) & 0xffff) == words@[22])
        && (words.len() == EXCEPTION_PREFIX_WORDS
            ==> args.user_rsp == 0 && ((args.metadata >> 48) & 0xffff) == 0)
}

pub open spec fn spec_snapshot_valid(cpu: &PerCpuSnapshot, context: &DispatchContext) -> bool {
    cpu.cpu_id < 256
        && cpu.lock_held
        && cpu.unique_state_token
        && cpu.interrupts_masked
        && cpu.current_thread == context.thread
        && cpu.scheduler_ready
        && cpu.crash_record_ready
}

pub fn exception_frame_valid(words: &[u64]) -> (result: bool)
    ensures result == spec_exception_frame_valid(words),
{
    if words.len() == EXCEPTION_PREFIX_WORDS {
        words[16] <= 255
            && words[19] == KERNEL_CODE_SELECTOR
            && words[18] >= 0xffff_8000_0000_0000
            && words[20] & 2 == 2
            && words[20] & !RETURN_RFLAGS_ALLOWED == 0
    } else if words.len() == EXCEPTION_USER_WORDS {
        words[16] <= 255
            && words[19] == USER_CODE_SELECTOR
            && words[18] <= 0x0000_7fff_ffff_ffff
            && words[20] & 2 == 2
            && words[20] & !RETURN_RFLAGS_ALLOWED == 0
            && words[21] <= 0x0000_7fff_ffff_ffff
            && words[22] == USER_DATA_SELECTOR
    } else {
        false
    }
}

pub fn scalar_arguments_match(words: &[u64], args: &ScalarArguments) -> (result: bool)
    ensures result == spec_scalar_arguments_match(words, args),
{
    if !exception_frame_valid(words) {
        false
    } else {
        let prefix = args.cr2 == words[14]
            && args.error == words[17]
            && args.rip == words[18]
            && args.rflags == words[20]
            && (args.metadata & 0xffff_ffff) == words[16]
            && ((args.metadata >> 32) & 0xffff) == words[19];
        if words.len() == EXCEPTION_USER_WORDS {
            prefix
                && args.user_rsp == words[21]
                && ((args.metadata >> 48) & 0xffff) == words[22]
        } else {
            prefix && args.user_rsp == 0 && ((args.metadata >> 48) & 0xffff) == 0
        }
    }
}

pub fn normalize_exception_event(
    words: &[u64],
    context: &DispatchContext,
) -> (result: ExceptionEvent)
    ensures
        result.frame_valid == spec_exception_frame_valid(words),
        result.thread == context.thread,
        result.thread_live == context.thread_live,
        result.fault_endpoint_valid == context.fault_endpoint_valid,
        result.vspace_epoch == context.vspace_epoch,
        result.irq_bound == context.irq_bound,
        result.acknowledge_required == context.acknowledge_required,
        result.wakes_higher_priority == context.wakes_higher_priority,
        result.shootdown_epoch == context.shootdown_epoch,
{
    let valid = exception_frame_valid(words);
    ExceptionEvent {
        vector: if words.len() >= 17 && words[16] <= 255 {
            words[16] as u32
        } else {
            256
        },
        error: if words.len() >= 18 { words[17] } else { 0 },
        from_user: words.len() >= 20 && words[19] == USER_CODE_SELECTOR,
        frame_valid: valid,
        thread: context.thread,
        thread_live: context.thread_live,
        fault_endpoint_valid: context.fault_endpoint_valid,
        cr2: if words.len() >= 15 { words[14] } else { 0 },
        vspace_epoch: context.vspace_epoch,
        irq_bound: context.irq_bound,
        acknowledge_required: context.acknowledge_required,
        wakes_higher_priority: context.wakes_higher_priority,
        shootdown_epoch: context.shootdown_epoch,
    }
}

fn latch_bridge_failure(
    state: ExceptionState,
    machine: MachineState,
    reason: u32,
    bridge_valid: bool,
    policy_invoked: bool,
)
    -> (result: ScalarOutcome)
    requires reason >= 100
    ensures
        result.control == CONTROL_FAIL_STOP,
        result.bridge_valid == bridge_valid,
        result.policy_invoked == policy_invoked,
        !result.action_committed,
        result.action_code == 0,
        result.policy_state.panic_latched,
        !result.policy_state.reschedule_pending,
        result.policy_state.fault_generation == state.fault_generation,
        result.policy_state.timer_expiries == state.timer_expiries,
        result.policy_state.irq_deliveries == state.irq_deliveries,
        result.policy_state.quarantined_vectors == state.quarantined_vectors,
        result.policy_state.spurious_vectors == state.spurious_vectors,
        result.policy_state.last_tlb_epoch == state.last_tlb_epoch,
        result.machine.crash_latched,
        result.machine.crash_reason == reason,
{
    ScalarOutcome {
        policy_state: ExceptionState {
            fault_generation: state.fault_generation,
            timer_expiries: state.timer_expiries,
            irq_deliveries: state.irq_deliveries,
            quarantined_vectors: state.quarantined_vectors,
            spurious_vectors: state.spurious_vectors,
            last_tlb_epoch: state.last_tlb_epoch,
            reschedule_pending: false,
            panic_latched: true,
        },
        machine: MachineState {
            current_thread_state: machine.current_thread_state,
            fault_generation: machine.fault_generation,
            fault_thread: machine.fault_thread,
            fault_vector: machine.fault_vector,
            fault_error: machine.fault_error,
            fault_address: machine.fault_address,
            fault_access: machine.fault_access,
            fault_vspace_epoch: machine.fault_vspace_epoch,
            timer_expiries: machine.timer_expiries,
            reschedule_pending: false,
            irq_masked_vector: machine.irq_masked_vector,
            notification_vector: machine.notification_vector,
            irq_acknowledgements: machine.irq_acknowledgements,
            tlb_epoch: machine.tlb_epoch,
            tlb_acknowledgements: machine.tlb_acknowledgements,
            quarantined_vector: machine.quarantined_vector,
            spurious_count: machine.spurious_count,
            crash_latched: true,
            crash_reason: reason,
        },
        control: CONTROL_FAIL_STOP,
        action_code: 0,
        bridge_valid,
        policy_invoked,
        action_committed: false,
    }
}

pub fn execute_exception_action(
    prior_state: ExceptionState,
    policy_state: ExceptionState,
    action: ExceptionAction,
    machine: MachineState,
    cpu: &PerCpuSnapshot,
) -> (result: ScalarOutcome)
    ensures
        result.bridge_valid,
        result.policy_invoked,
        result.control <= CONTROL_FAIL_STOP,
        result.control == CONTROL_FAIL_STOP ==> result.machine.crash_latched,
        result.control != CONTROL_FAIL_STOP ==> result.action_committed,
        result.action_code == 1 ==> result.control == CONTROL_SCHEDULE
            && result.machine.current_thread_state == 1,
        result.action_code == 2 ==> result.control == CONTROL_SCHEDULE
            && result.machine.current_thread_state == 2,
        result.action_code == 3 ==> result.control == CONTROL_SCHEDULE
            && result.machine.reschedule_pending,
        result.action_code == 4 ==> result.control == CONTROL_SCHEDULE
            && result.machine.reschedule_pending,
        result.action_code == 6 || result.action_code == 7 ==>
            result.control == CONTROL_RETURN,
        result.action_code == 8 || result.action_code == 9 ==>
            result.control == CONTROL_RETURN,
        result.action_code == 10 ==> result.control == CONTROL_FAIL_STOP,
        result.action_code == 0 ==>
            result.policy_state.fault_generation == prior_state.fault_generation
            && result.policy_state.timer_expiries == prior_state.timer_expiries
            && result.policy_state.irq_deliveries == prior_state.irq_deliveries
            && result.policy_state.quarantined_vectors == prior_state.quarantined_vectors
            && result.policy_state.spurious_vectors == prior_state.spurious_vectors
            && result.policy_state.last_tlb_epoch == prior_state.last_tlb_epoch,
{
    match action {
        ExceptionAction::DeliverFault {
            generation, thread, kind, vector, error, address, access, vspace_epoch
        } => {
            if !cpu.fault_slot_ready || thread != cpu.current_thread {
                return latch_bridge_failure(prior_state, machine, 103, true, true);
            }
            ScalarOutcome {
                policy_state,
                machine: MachineState {
                    current_thread_state: 1,
                    fault_generation: generation,
                    fault_thread: thread,
                    fault_vector: vector,
                    fault_error: error,
                    fault_address: address,
                    fault_access: access,
                    fault_vspace_epoch: vspace_epoch,
                    timer_expiries: machine.timer_expiries,
                    reschedule_pending: true,
                    irq_masked_vector: machine.irq_masked_vector,
                    notification_vector: machine.notification_vector,
                    irq_acknowledgements: machine.irq_acknowledgements,
                    tlb_epoch: machine.tlb_epoch,
                    tlb_acknowledgements: machine.tlb_acknowledgements,
                    quarantined_vector: machine.quarantined_vector,
                    spurious_count: machine.spurious_count,
                    crash_latched: false,
                    crash_reason: kind,
                },
                control: CONTROL_SCHEDULE,
                action_code: 1,
                bridge_valid: true,
                policy_invoked: true,
                action_committed: true,
            }
        },
        ExceptionAction::TerminateThread { thread, vector } => {
            if thread != cpu.current_thread {
                return latch_bridge_failure(prior_state, machine, 103, true, true);
            }
            ScalarOutcome {
                policy_state,
                machine: MachineState {
                    current_thread_state: 2,
                    fault_generation: machine.fault_generation,
                    fault_thread: thread,
                    fault_vector: vector,
                    fault_error: machine.fault_error,
                    fault_address: machine.fault_address,
                    fault_access: machine.fault_access,
                    fault_vspace_epoch: machine.fault_vspace_epoch,
                    timer_expiries: machine.timer_expiries,
                    reschedule_pending: true,
                    irq_masked_vector: machine.irq_masked_vector,
                    notification_vector: machine.notification_vector,
                    irq_acknowledgements: machine.irq_acknowledgements,
                    tlb_epoch: machine.tlb_epoch,
                    tlb_acknowledgements: machine.tlb_acknowledgements,
                    quarantined_vector: machine.quarantined_vector,
                    spurious_count: machine.spurious_count,
                    crash_latched: false,
                    crash_reason: 0,
                },
                control: CONTROL_SCHEDULE, action_code: 2, bridge_valid: true,
                policy_invoked: true, action_committed: true,
            }
        },
        ExceptionAction::TimerRecorded { expiries } => ScalarOutcome {
            policy_state,
            machine: MachineState {
                current_thread_state: machine.current_thread_state,
                fault_generation: machine.fault_generation,
                fault_thread: machine.fault_thread,
                fault_vector: machine.fault_vector,
                fault_error: machine.fault_error,
                fault_address: machine.fault_address,
                fault_access: machine.fault_access,
                fault_vspace_epoch: machine.fault_vspace_epoch,
                timer_expiries: expiries,
                reschedule_pending: true,
                irq_masked_vector: machine.irq_masked_vector,
                notification_vector: machine.notification_vector,
                irq_acknowledgements: machine.irq_acknowledgements,
                tlb_epoch: machine.tlb_epoch,
                tlb_acknowledgements: machine.tlb_acknowledgements,
                quarantined_vector: machine.quarantined_vector,
                spurious_count: machine.spurious_count,
                crash_latched: false,
                crash_reason: 0,
            },
            control: CONTROL_SCHEDULE, action_code: 3, bridge_valid: true,
            policy_invoked: true, action_committed: true,
        },
        ExceptionAction::Reschedule => ScalarOutcome {
            policy_state,
            machine: MachineState { reschedule_pending: true, ..machine },
            control: CONTROL_SCHEDULE, action_code: 4, bridge_valid: true,
            policy_invoked: true, action_committed: true,
        },
        ExceptionAction::NotifyIrq { vector, masked, acknowledge, reschedule } => {
            if !cpu.irq_backend_ready || !masked
                || (acknowledge && machine.irq_acknowledgements == u64::MAX)
            {
                return latch_bridge_failure(prior_state, machine, 103, true, true);
            }
            ScalarOutcome {
                policy_state,
                machine: MachineState {
                    irq_masked_vector: vector,
                    notification_vector: vector,
                    irq_acknowledgements: machine.irq_acknowledgements
                        + if acknowledge { 1 } else { 0 },
                    reschedule_pending: reschedule,
                    ..machine
                },
                control: if reschedule { CONTROL_SCHEDULE } else { CONTROL_RETURN },
                action_code: 5, bridge_valid: true, policy_invoked: true,
                action_committed: true,
            }
        },
        ExceptionAction::TlbShootdown { epoch, acknowledge } => {
            if !cpu.tlb_backend_ready
                || (acknowledge && machine.tlb_acknowledgements == u64::MAX)
            {
                return latch_bridge_failure(prior_state, machine, 103, true, true);
            }
            ScalarOutcome {
                policy_state,
                machine: MachineState {
                    tlb_epoch: epoch,
                    tlb_acknowledgements: machine.tlb_acknowledgements
                        + if acknowledge { 1 } else { 0 },
                    ..machine
                },
                control: CONTROL_RETURN, action_code: 6, bridge_valid: true,
                policy_invoked: true, action_committed: true,
            }
        },
        ExceptionAction::StaleTlbShootdown { epoch: _, acknowledge } => {
            if !cpu.tlb_backend_ready
                || (acknowledge && machine.tlb_acknowledgements == u64::MAX)
            {
                return latch_bridge_failure(prior_state, machine, 103, true, true);
            }
            ScalarOutcome {
                policy_state,
                machine: MachineState {
                    tlb_epoch: machine.tlb_epoch,
                    tlb_acknowledgements: machine.tlb_acknowledgements
                        + if acknowledge { 1 } else { 0 },
                    ..machine
                },
                control: CONTROL_RETURN, action_code: 7, bridge_valid: true,
                policy_invoked: true, action_committed: true,
            }
        },
        ExceptionAction::Quarantine { vector, acknowledge } => {
            if !cpu.irq_backend_ready
                || (acknowledge && machine.irq_acknowledgements == u64::MAX)
            {
                return latch_bridge_failure(prior_state, machine, 103, true, true);
            }
            ScalarOutcome {
                policy_state,
                machine: MachineState {
                    irq_masked_vector: vector,
                    quarantined_vector: vector,
                    irq_acknowledgements: machine.irq_acknowledgements
                        + if acknowledge { 1 } else { 0 },
                    ..machine
                },
                control: CONTROL_RETURN, action_code: 8, bridge_valid: true,
                policy_invoked: true, action_committed: true,
            }
        },
        ExceptionAction::Spurious { count } => ScalarOutcome {
            policy_state,
            machine: MachineState { spurious_count: count, ..machine },
            control: CONTROL_RETURN, action_code: 9, bridge_valid: true,
            policy_invoked: true, action_committed: true,
        },
        ExceptionAction::Panic { reason } => ScalarOutcome {
            policy_state,
            machine: MachineState {
                reschedule_pending: false,
                crash_latched: true,
                crash_reason: reason,
                ..machine
            },
            control: CONTROL_FAIL_STOP, action_code: 10, bridge_valid: true,
            policy_invoked: true, action_committed: false,
        },
    }
}

pub fn scalar_dispatch_checked(
    state: ExceptionState,
    words: &[u64],
    args: &ScalarArguments,
    context: &DispatchContext,
    cpu: &PerCpuSnapshot,
    machine: MachineState,
) -> (result: ScalarOutcome)
    ensures
        result.control <= CONTROL_FAIL_STOP,
        result.control == CONTROL_FAIL_STOP ==> result.machine.crash_latched,
        !spec_snapshot_valid(cpu, context) ==> !result.bridge_valid
            && result.machine.crash_reason == 100,
        spec_snapshot_valid(cpu, context) && !spec_scalar_arguments_match(words, args)
            ==> !result.bridge_valid && result.machine.crash_reason == 101,
        spec_snapshot_valid(cpu, context) && spec_scalar_arguments_match(words, args)
            ==> result.policy_invoked,
{
    if !(cpu.cpu_id < 256
        && cpu.lock_held
        && cpu.unique_state_token
        && cpu.interrupts_masked
        && cpu.current_thread == context.thread
        && cpu.scheduler_ready
        && cpu.crash_record_ready)
    {
        return latch_bridge_failure(state, machine, 100, false, false);
    }
    if !scalar_arguments_match(words, args) {
        return latch_bridge_failure(state, machine, 101, false, false);
    }
    let prior_state = ExceptionState {
        fault_generation: state.fault_generation,
        timer_expiries: state.timer_expiries,
        irq_deliveries: state.irq_deliveries,
        quarantined_vectors: state.quarantined_vectors,
        spurious_vectors: state.spurious_vectors,
        last_tlb_epoch: state.last_tlb_epoch,
        reschedule_pending: state.reschedule_pending,
        panic_latched: state.panic_latched,
    };
    let event = normalize_exception_event(words, context);
    let result = exception_policy_step(state, event);
    execute_exception_action(prior_state, result.0, result.1, machine, cpu)
}
