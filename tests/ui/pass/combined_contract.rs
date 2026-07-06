use contracts::contract;

#[contract(
    requires(x >= 0, "input is non-negative"),
    debug_invariant(x >= 0),
    ensures(ret >= x),
    debug_ensures(ret == x + 1),
)]
fn incr(x: i32) -> i32 {
    x + 1
}

fn main() {
    assert_eq!(incr(1), 2);
}
