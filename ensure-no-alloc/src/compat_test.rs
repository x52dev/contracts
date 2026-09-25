use contracts::*;

#[contract(
    requires(value < u32::MAX, "Increment must not overflow"),
    ensures(ret == value + 1),
)]
fn increment(value: u32) -> u32 {
    value + 1
}

struct Counter(u32);

#[invariant(self.0 < 100)]
impl Counter {
    #[debug_requires(self.0 < 99)]
    #[debug_ensures(self.0 == old(self.0) + 1)]
    fn increment(&mut self) {
        self.0 = increment(self.0);
    }
}

#[contract_trait]
trait ReadCounter {
    #[ensures(ret < 100)]
    fn read(&self) -> u32;
}

#[contract_trait]
impl ReadCounter for Counter {
    fn read(&self) -> u32 {
        self.0
    }
}

#[test_requires(value < 100)]
#[test_ensures(ret == value)]
#[test_invariant(value < 100)]
fn identity(value: u32) -> u32 {
    value
}

pub fn exercise() {
    let mut counter = Counter(0);
    counter.increment();
    assert_eq!(identity(counter.read()), 1);
}
