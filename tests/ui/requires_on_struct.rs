use contracts::requires;

#[requires(true)]
struct NotAFunction;

fn main() {
    let _ = NotAFunction;
}
