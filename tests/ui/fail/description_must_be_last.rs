use contracts::requires;

#[requires("x must be positive", x > 0)]
fn checked(x: i32) -> i32 {
    x
}

fn main() {
    let _ = checked(1);
}
