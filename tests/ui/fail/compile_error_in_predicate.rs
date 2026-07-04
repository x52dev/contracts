use contracts::requires;

#[requires(compile_error!("predicate should be a boolean expression"))]
fn broken_predicate() {}

fn main() {
    broken_predicate();
}
