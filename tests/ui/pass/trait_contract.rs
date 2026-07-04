use contracts::*;

#[contract_trait]
trait Scale {
    #[requires(factor > 0)]
    #[ensures(ret >= self.base())]
    fn scale(&self, factor: u32) -> u32;

    fn base(&self) -> u32;
}

struct Multiplier(u32);

#[contract_trait]
impl Scale for Multiplier {
    fn scale(&self, factor: u32) -> u32 {
        self.0 * factor
    }

    fn base(&self) -> u32 {
        self.0
    }
}

fn main() {
    let multiplier = Multiplier(4);

    assert_eq!(multiplier.scale(2), 8);
}
