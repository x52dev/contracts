use contracts::ensures;

#[ensures(ret -> *value == old(*value) + 1)]
#[ensures(!ret -> *value == old(*value))]
fn increment_if(value: &mut u32, enabled: bool) -> bool {
    if enabled {
        *value += 1;
        true
    } else {
        false
    }
}

fn main() {
    let mut value = 1;

    assert!(increment_if(&mut value, true));
    assert!(!increment_if(&mut value, false));
}
