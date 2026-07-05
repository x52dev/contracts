use contracts::ensures;

#[ensures(false)]
fn checked() {}

fn main() {
    checked();
}
