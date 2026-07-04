use contracts::*;

#[requires(x > 0)]
#[ensures(ret > x)]
fn increment(x: i32) -> i32 {
    x + 1
}

struct Counter {
    value: i32,
}

impl Counter {
    #[invariant(self.value >= 0)]
    fn bump(&mut self) {
        self.value += 1;
    }
}

fn main() {
    assert_eq!(increment(1), 2);

    let mut counter = Counter { value: 0 };
    counter.bump();
}
