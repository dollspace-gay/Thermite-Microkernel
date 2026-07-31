fn main() {
    let observed = tmk_firmware_policy::firmware_policy_shell::firmware_policy_observation();
    assert_eq!(observed, 255);
    println!("M1_FIRMWARE_POLICY_OK observation={observed}");
}
