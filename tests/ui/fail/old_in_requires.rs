use contracts::requires;

#[requires(old(x) == x)]
fn checked(x: i32) -> i32 {
    x
}

fn main() {
    let _ = checked(1);
}
