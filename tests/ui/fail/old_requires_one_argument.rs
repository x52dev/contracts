use contracts::ensures;

#[ensures(old(x, y) == ret)]
fn add(x: i32, y: i32) -> i32 {
    x + y
}

fn main() {
    let _ = add(1, 2);
}
