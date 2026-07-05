use contracts::requires;

#[requires(x >)]
fn checked(x: i32) -> i32 {
    x
}

fn main() {
    let _ = checked(1);
}
