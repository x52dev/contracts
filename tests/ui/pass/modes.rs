use contracts::*;

#[test_requires(input.len() < 8)]
#[debug_ensures(ret.len() == input.len())]
fn copy(input: &[u8]) -> Vec<u8> {
    input.to_vec()
}

fn main() {
    let _ = copy(&[1, 2, 3]);
}
