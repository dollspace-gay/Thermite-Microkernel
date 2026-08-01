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
pub const SCALAR_CORE_BLOCK_WORDS: usize = 80;

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

// Every field is a u64 so the pinned kernel codegen boundary can audit this
// block as 80 consecutive eight-byte slots before linking the assembly wrapper.
pub struct ScalarCoreBlock {
    pub slot_00: u64,
    pub slot_01: u64,
    pub slot_02: u64,
    pub slot_03: u64,
    pub slot_04: u64,
    pub slot_05: u64,
    pub slot_06: u64,
    pub slot_07: u64,
    pub slot_08: u64,
    pub slot_09: u64,
    pub slot_10: u64,
    pub slot_11: u64,
    pub slot_12: u64,
    pub slot_13: u64,
    pub slot_14: u64,
    pub slot_15: u64,
    pub slot_16: u64,
    pub slot_17: u64,
    pub slot_18: u64,
    pub slot_19: u64,
    pub slot_20: u64,
    pub slot_21: u64,
    pub slot_22: u64,
    pub slot_23: u64,
    pub slot_24: u64,
    pub slot_25: u64,
    pub slot_26: u64,
    pub slot_27: u64,
    pub slot_28: u64,
    pub slot_29: u64,
    pub slot_30: u64,
    pub slot_31: u64,
    pub slot_32: u64,
    pub slot_33: u64,
    pub slot_34: u64,
    pub slot_35: u64,
    pub slot_36: u64,
    pub slot_37: u64,
    pub slot_38: u64,
    pub slot_39: u64,
    pub slot_40: u64,
    pub slot_41: u64,
    pub slot_42: u64,
    pub slot_43: u64,
    pub slot_44: u64,
    pub slot_45: u64,
    pub slot_46: u64,
    pub slot_47: u64,
    pub slot_48: u64,
    pub slot_49: u64,
    pub slot_50: u64,
    pub slot_51: u64,
    pub slot_52: u64,
    pub slot_53: u64,
    pub slot_54: u64,
    pub slot_55: u64,
    pub slot_56: u64,
    pub slot_57: u64,
    pub slot_58: u64,
    pub slot_59: u64,
    pub slot_60: u64,
    pub slot_61: u64,
    pub slot_62: u64,
    pub slot_63: u64,
    pub slot_64: u64,
    pub slot_65: u64,
    pub slot_66: u64,
    pub slot_67: u64,
    pub slot_68: u64,
    pub slot_69: u64,
    pub slot_70: u64,
    pub slot_71: u64,
    pub slot_72: u64,
    pub slot_73: u64,
    pub slot_74: u64,
    pub slot_75: u64,
    pub slot_76: u64,
    pub slot_77: u64,
    pub slot_78: u64,
    pub slot_79: u64,
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
        result.action_code <= 10,
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
        result.action_code <= 10,
        result.control <= CONTROL_FAIL_STOP,
        result.control == CONTROL_FAIL_STOP ==> result.machine.crash_latched,
        !result.bridge_valid ==> !result.action_committed,
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

fn scalar_block_frame_valid(block: &ScalarCoreBlock) -> (result: bool)
{
    if block.slot_23 == EXCEPTION_PREFIX_WORDS as u64 {
        block.slot_16 <= 255
            && block.slot_19 == KERNEL_CODE_SELECTOR
            && block.slot_18 >= 0xffff_8000_0000_0000
            && block.slot_20 & 2 == 2
            && block.slot_20 & !RETURN_RFLAGS_ALLOWED == 0
    } else if block.slot_23 == EXCEPTION_USER_WORDS as u64 {
        block.slot_16 <= 255
            && block.slot_19 == USER_CODE_SELECTOR
            && block.slot_18 <= 0x0000_7fff_ffff_ffff
            && block.slot_20 & 2 == 2
            && block.slot_20 & !RETURN_RFLAGS_ALLOWED == 0
            && block.slot_21 <= 0x0000_7fff_ffff_ffff
            && block.slot_22 == USER_DATA_SELECTOR
    } else {
        false
    }
}

fn scalar_block_arguments_match(block: &ScalarCoreBlock, args: &ScalarArguments) -> (result: bool)
{
    if !scalar_block_frame_valid(block) {
        false
    } else {
        let prefix = args.cr2 == block.slot_14
            && args.error == block.slot_17
            && args.rip == block.slot_18
            && args.rflags == block.slot_20
            && (args.metadata & 0xffff_ffff) == block.slot_16
            && ((args.metadata >> 32) & 0xffff) == block.slot_19;
        if block.slot_23 == EXCEPTION_USER_WORDS as u64 {
            prefix
                && args.user_rsp == block.slot_21
                && ((args.metadata >> 48) & 0xffff) == block.slot_22
        } else {
            prefix && args.user_rsp == 0 && ((args.metadata >> 48) & 0xffff) == 0
        }
    }
}

fn normalize_exception_block_event(
    block: &ScalarCoreBlock,
    context: &DispatchContext,
) -> (result: ExceptionEvent)
{
    let valid = scalar_block_frame_valid(block);
    ExceptionEvent {
        vector: if block.slot_16 <= 255 { block.slot_16 as u32 } else { 256 },
        error: block.slot_17,
        from_user: block.slot_23 == EXCEPTION_USER_WORDS as u64
            && block.slot_19 == USER_CODE_SELECTOR,
        frame_valid: valid,
        thread: context.thread,
        thread_live: context.thread_live,
        fault_endpoint_valid: context.fault_endpoint_valid,
        cr2: block.slot_14,
        vspace_epoch: context.vspace_epoch,
        irq_bound: context.irq_bound,
        acknowledge_required: context.acknowledge_required,
        wakes_higher_priority: context.wakes_higher_priority,
        shootdown_epoch: context.shootdown_epoch,
    }
}

fn scalar_block_dispatch_checked(
    state: ExceptionState,
    block: &ScalarCoreBlock,
    args: &ScalarArguments,
    context: &DispatchContext,
    cpu: &PerCpuSnapshot,
    machine: MachineState,
) -> (result: ScalarOutcome)
    ensures
        result.action_code <= 10,
        result.control <= CONTROL_FAIL_STOP,
        result.control == CONTROL_FAIL_STOP ==> result.machine.crash_latched,
        !result.bridge_valid ==> !result.action_committed,
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
    if !scalar_block_arguments_match(block, args) {
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
    let event = normalize_exception_block_event(block, context);
    let result = exception_policy_step(state, event);
    execute_exception_action(prior_state, result.0, result.1, machine, cpu)
}

pub fn tmk_exception_scalar_adapter(
    block: &mut ScalarCoreBlock,
) -> (control: u32)
    ensures
        control <= CONTROL_FAIL_STOP,
        final(block).slot_75 == control as u64,
        final(block).slot_76 <= 10,
        control == CONTROL_FAIL_STOP ==> final(block).slot_73 == 1,
        final(block).slot_77 == 0 ==> final(block).slot_79 == 0,
{
    let arguments = ScalarArguments {
        cr2: block.slot_24,
        error: block.slot_25,
        rip: block.slot_26,
        rflags: block.slot_27,
        user_rsp: block.slot_28,
        metadata: block.slot_29,
    };
    let context = DispatchContext {
        thread: block.slot_30,
        thread_live: block.slot_31 != 0,
        fault_endpoint_valid: block.slot_32 != 0,
        vspace_epoch: block.slot_33,
        irq_bound: block.slot_34 != 0,
        acknowledge_required: block.slot_35 != 0,
        wakes_higher_priority: block.slot_36 != 0,
        shootdown_epoch: block.slot_37,
    };
    let cpu = PerCpuSnapshot {
        cpu_id: block.slot_38 as u32,
        lock_held: block.slot_39 != 0,
        unique_state_token: block.slot_40 != 0,
        interrupts_masked: block.slot_41 != 0,
        current_thread: block.slot_42,
        fault_slot_ready: block.slot_43 != 0,
        irq_backend_ready: block.slot_44 != 0,
        tlb_backend_ready: block.slot_45 != 0,
        scheduler_ready: block.slot_46 != 0,
        crash_record_ready: block.slot_47 != 0,
    };
    let state = ExceptionState {
        fault_generation: block.slot_48,
        timer_expiries: block.slot_49,
        irq_deliveries: block.slot_50,
        quarantined_vectors: block.slot_51,
        spurious_vectors: block.slot_52,
        last_tlb_epoch: block.slot_53,
        reschedule_pending: block.slot_54 != 0,
        panic_latched: block.slot_55 != 0,
    };
    let machine = MachineState {
        current_thread_state: block.slot_56 as u32,
        fault_generation: block.slot_57,
        fault_thread: block.slot_58,
        fault_vector: block.slot_59 as u32,
        fault_error: block.slot_60,
        fault_address: block.slot_61,
        fault_access: block.slot_62 as u32,
        fault_vspace_epoch: block.slot_63,
        timer_expiries: block.slot_64,
        reschedule_pending: block.slot_65 != 0,
        irq_masked_vector: block.slot_66 as u32,
        notification_vector: block.slot_67 as u32,
        irq_acknowledgements: block.slot_68,
        tlb_epoch: block.slot_69,
        tlb_acknowledgements: block.slot_70,
        quarantined_vector: block.slot_71 as u32,
        spurious_count: block.slot_72,
        crash_latched: block.slot_73 != 0,
        crash_reason: block.slot_74 as u32,
    };
    let outcome = scalar_block_dispatch_checked(
        state, block, &arguments, &context, &cpu, machine,
    );
    let result_control = outcome.control;
    let result_action = outcome.action_code;
    let result_bridge = outcome.bridge_valid;
    let result_policy = outcome.policy_invoked;
    let result_committed = outcome.action_committed;
    block.slot_48 = outcome.policy_state.fault_generation;
    block.slot_49 = outcome.policy_state.timer_expiries;
    block.slot_50 = outcome.policy_state.irq_deliveries;
    block.slot_51 = outcome.policy_state.quarantined_vectors;
    block.slot_52 = outcome.policy_state.spurious_vectors;
    block.slot_53 = outcome.policy_state.last_tlb_epoch;
    block.slot_54 = if outcome.policy_state.reschedule_pending { 1 } else { 0 };
    block.slot_55 = if outcome.policy_state.panic_latched { 1 } else { 0 };
    block.slot_56 = outcome.machine.current_thread_state as u64;
    block.slot_57 = outcome.machine.fault_generation;
    block.slot_58 = outcome.machine.fault_thread;
    block.slot_59 = outcome.machine.fault_vector as u64;
    block.slot_60 = outcome.machine.fault_error;
    block.slot_61 = outcome.machine.fault_address;
    block.slot_62 = outcome.machine.fault_access as u64;
    block.slot_63 = outcome.machine.fault_vspace_epoch;
    block.slot_64 = outcome.machine.timer_expiries;
    block.slot_65 = if outcome.machine.reschedule_pending { 1 } else { 0 };
    block.slot_66 = outcome.machine.irq_masked_vector as u64;
    block.slot_67 = outcome.machine.notification_vector as u64;
    block.slot_68 = outcome.machine.irq_acknowledgements;
    block.slot_69 = outcome.machine.tlb_epoch;
    block.slot_70 = outcome.machine.tlb_acknowledgements;
    block.slot_71 = outcome.machine.quarantined_vector as u64;
    block.slot_72 = outcome.machine.spurious_count;
    block.slot_73 = if outcome.machine.crash_latched { 1 } else { 0 };
    block.slot_74 = outcome.machine.crash_reason as u64;
    block.slot_75 = result_control as u64;
    block.slot_76 = result_action as u64;
    block.slot_77 = if result_bridge { 1 } else { 0 };
    block.slot_78 = if result_policy { 1 } else { 0 };
    block.slot_79 = if result_committed { 1 } else { 0 };
    result_control
}
