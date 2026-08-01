use vstd::prelude::*;

pub const EXCEPTION_PREFIX_WORDS: usize = 21;
pub const EXCEPTION_USER_WORDS: usize = 23;
pub const KERNEL_CODE_SELECTOR: u64 = 0x08;
pub const USER_DATA_SELECTOR: u64 = 0x1b;
pub const USER_CODE_SELECTOR: u64 = 0x23;
pub const RETURN_RFLAGS_ALLOWED: u64 = 0x0025_0fd7;

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

pub open spec fn spec_canonical(address: u64) -> bool {
    address <= 0x0000_7fff_ffff_ffff || address >= 0xffff_8000_0000_0000
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
    words.len() == EXCEPTION_PREFIX_WORDS &&
        words@[16] <= 255 &&
        words@[19] == KERNEL_CODE_SELECTOR &&
        spec_kernel_address(words@[18]) &&
        spec_return_flags(words@[20])
    ||
    words.len() == EXCEPTION_USER_WORDS &&
        words@[16] <= 255 &&
        words@[19] == USER_CODE_SELECTOR &&
        spec_user_address(words@[18]) &&
        spec_return_flags(words@[20]) &&
        spec_user_address(words@[21]) &&
        words@[22] == USER_DATA_SELECTOR
}

pub fn exception_frame_valid(words: &[u64]) -> (result: bool)
    ensures result == spec_exception_frame_valid(words),
{
    if words.len() == EXCEPTION_PREFIX_WORDS {
        let vector = words[16];
        let rip = words[18];
        let cs = words[19];
        let flags = words[20];
        vector <= 255 && cs == KERNEL_CODE_SELECTOR &&
            rip >= 0xffff_8000_0000_0000 &&
            flags & 2 == 2 && flags & !RETURN_RFLAGS_ALLOWED == 0
    } else if words.len() == EXCEPTION_USER_WORDS {
        let vector = words[16];
        let rip = words[18];
        let cs = words[19];
        let flags = words[20];
        let rsp = words[21];
        let ss = words[22];
        vector <= 255 && cs == USER_CODE_SELECTOR &&
            rip <= 0x0000_7fff_ffff_ffff &&
            flags & 2 == 2 && flags & !RETURN_RFLAGS_ALLOWED == 0 &&
            rsp <= 0x0000_7fff_ffff_ffff && ss == USER_DATA_SELECTOR
    } else {
        false
    }
}

pub fn normalize_exception_event(
    words: &[u64],
    context: &DispatchContext,
) -> (result: ExceptionEvent)
    ensures
        result.frame_valid == spec_exception_frame_valid(words),
        words.len() >= 17 ==> result.vector == if words@[16] <= 255 {
            words@[16] as u32
        } else {
            256u32
        },
        words.len() < 17 ==> result.vector == 256,
        words.len() >= 18 ==> result.error == words@[17],
        words.len() < 18 ==> result.error == 0,
        result.from_user ==
            (words.len() >= 20 && words@[19] == USER_CODE_SELECTOR),
        result.thread == context.thread,
        result.thread_live == context.thread_live,
        result.fault_endpoint_valid == context.fault_endpoint_valid,
        words.len() >= 15 ==> result.cr2 == words@[14],
        words.len() < 15 ==> result.cr2 == 0,
        result.vspace_epoch == context.vspace_epoch,
        result.irq_bound == context.irq_bound,
        result.acknowledge_required == context.acknowledge_required,
        result.wakes_higher_priority == context.wakes_higher_priority,
        result.shootdown_epoch == context.shootdown_epoch,
{
    let valid = exception_frame_valid(words);
    let vector = if words.len() >= 17 {
        if words[16] <= 255 { words[16] as u32 } else { 256 }
    } else {
        256
    };
    let error = if words.len() >= 18 { words[17] } else { 0 };
    let from_user = words.len() >= 20 && words[19] == USER_CODE_SELECTOR;
    let cr2 = if words.len() >= 15 { words[14] } else { 0 };
    ExceptionEvent {
        vector,
        error,
        from_user,
        frame_valid: valid,
        thread: context.thread,
        thread_live: context.thread_live,
        fault_endpoint_valid: context.fault_endpoint_valid,
        cr2,
        vspace_epoch: context.vspace_epoch,
        irq_bound: context.irq_bound,
        acknowledge_required: context.acknowledge_required,
        wakes_higher_priority: context.wakes_higher_priority,
        shootdown_epoch: context.shootdown_epoch,
    }
}

pub fn dispatch_exception_frame(
    state: ExceptionState,
    words: &[u64],
    context: &DispatchContext,
) -> (result: (ExceptionState, ExceptionAction))
    ensures
        !spec_exception_frame_valid(words) ==> result.0.panic_latched,
        !spec_exception_frame_valid(words) ==> !result.0.reschedule_pending,
        !spec_exception_frame_valid(words) ==> match result.1 {
            ExceptionAction::Panic { reason } =>
                reason == if state.panic_latched { 1u32 } else { 2u32 },
            _ => false,
        },
{
    let event = normalize_exception_event(words, context);
    exception_policy_step(state, event)
}
