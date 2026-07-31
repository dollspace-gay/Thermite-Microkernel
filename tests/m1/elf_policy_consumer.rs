fn main() {
    let observed = tmk_elf_policy::elf_policy_shell::elf_policy_observation();
    assert_eq!(observed, 127);
    println!("M1_ELF_POLICY_OK observation={observed}");
}
