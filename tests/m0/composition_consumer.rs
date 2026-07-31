extern crate tmk_composition_probe;

fn main() {
    assert_eq!(
        tmk_composition_probe::composition_shell::boot_observation(),
        1
    );
}
