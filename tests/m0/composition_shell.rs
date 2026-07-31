pub struct PlatformProbeState {
    pub owner: u64,
    pub generation: u64,
    pub used_slots: u64,
}

pub open spec fn represents(model: ProbeState, platform: PlatformProbeState) -> bool {
    model.owner == platform.owner
        && model.generation == platform.generation
        && model.used_slots == platform.used_slots
}

pub fn boot_observation() -> (result: u64)
    ensures result == 1,
{
    let model_before = ProbeState {
        owner: 7,
        generation: 11,
        used_slots: 0,
    };
    let platform_before = PlatformProbeState {
        owner: 7,
        generation: 11,
        used_slots: 0,
    };
    assert(represents(model_before, platform_before));

    let accepted = composition_step(
        model_before,
        ProbeEvent {
            actor: 7,
            generation: 11,
            value: 0x544d_4b31,
        },
    );
    let platform_after = PlatformProbeState {
        owner: platform_before.owner,
        generation: platform_before.generation,
        used_slots: platform_before.used_slots + 1,
    };
    assert(represents(accepted.0, platform_after));
    match accepted.1 {
        ProbeAction::Store { owner, generation, slot, value } => {
            assert(owner == 7);
            assert(generation == 11);
            assert(slot == 0);
            assert(value == 0x544d_4b31);
        }
        ProbeAction::Reject => assert(false),
    }

    let rejected = composition_step(
        accepted.0,
        ProbeEvent {
            actor: 8,
            generation: 11,
            value: 0,
        },
    );
    assert(represents(rejected.0, platform_after));
    match rejected.1 {
        ProbeAction::Store { owner: _, generation: _, slot: _, value: _ } => assert(false),
        ProbeAction::Reject => assert(true),
    }
    1
}
