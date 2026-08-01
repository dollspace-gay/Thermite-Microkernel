fn main() {
    let observed =
        tmk_exception_policy::exception_policy_shell::exception_policy_observation();
    assert_eq!(observed, 262143);
    println!("M1_EXCEPTION_POLICY_OK observation={observed} scenarios=18");
}
