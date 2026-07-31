extern crate tmk_probe;

fn main() {
    let state = 0x5a5a_1234_9876_ffff_u64;
    let event = 0x00ff_00ff_00ff_00ff_u64;
    let expected = state ^ event;
    let actual = tmk_probe::transition_probe(state, event);
    assert_eq!(actual, expected);
    println!("M0_FORGE_PROBE_OK:{actual:016x}");
}
