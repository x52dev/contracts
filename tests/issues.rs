#[test]
fn gl_issue_18() {
    use std::ops::{Div, Mul, Rem, Sub};

    // use contracts::ensures; // <- unused warning
    use contracts::requires;

    trait Zero {
        fn zero() -> Self;
    }

    impl Zero for u32 {
        fn zero() -> Self {
            0
        }
    }

    #[requires( n != T::zero() || d != T::zero() )]
    #[ensures( ret != T::zero() && n % ret == T::zero() && d % ret == T::zero() )]
    fn euclidean<T>(n: T, d: T) -> T
    where
        T: Sub<Output = T>
            + Mul<Output = T>
            + Div<Output = T>
            + Rem<Output = T>
            + Ord
            + Zero
            + Copy,
    {
        let (mut a, mut b) = (n.max(d), n.min(d));
        while b != T::zero() {
            let q: T = a / b;
            let r: T = a - q * b;
            a = b;
            b = r;
        }
        a
    }

    assert_eq!(1, euclidean(3, 4));
}
