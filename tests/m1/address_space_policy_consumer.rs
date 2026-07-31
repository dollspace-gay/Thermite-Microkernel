fn main() {
    let observed = tmk_address_space_policy::address_space_policy_shell::address_space_policy_observation();
    assert_eq!(observed, 511);
    println!("M1_ADDRESS_SPACE_POLICY_OK observation={observed}");
}
