use contracts::invariant;

#[invariant(self.value > 0)]
struct Positive {
    value: i32,
}

fn main() {
    let _ = Positive { value: 1 };
}
