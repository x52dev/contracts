#[allow(unused)] // compile-only test
#[test]
fn gl_issue_11() {
    use contracts::ensures;

    struct Test;

    impl Test {
        pub fn contains_key(&self, key: &str) -> bool {
            todo!()
        }

        #[ensures(self.contains_key(key) -> ret.is_some())]
        #[ensures(!self.contains_key(key) -> ret.is_none())]
        pub fn get_mut(&mut self, key: &str) -> Option<&mut u8> {
            None
        }
    }
}

#[allow(unused)] // compile-only test
#[test]
fn gl_issue_16() {
    use std::fmt::Debug;

    use contracts::debug_ensures;

    trait Sortable<T: Ord + Debug> {
        fn insertion_sort(&mut self);
    }

    // TODO(MSRV 1.82): remove and use std's [T].is_sorted()
    fn is_sorted<T: Ord>(s: &[T]) -> bool {
        s.windows(2).all(|w| w[0] <= w[1])
    }

    impl<T: Ord + Debug> Sortable<T> for Vec<T> {
        #[debug_ensures(is_sorted(self))]
        fn insertion_sort(&mut self) {
            assert!(self[0] < self[1]);
        }
    }
}

#[allow(unused, clippy::assertions_on_constants)] // compile-only test
#[test]
fn gl_issue_17() {
    use std::future::pending;

    use contracts::ensures;

    #[ensures(true)]
    async fn foo() {
        pending::<()>().await;
    }
}

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

#[allow(unused)] // compile-only test
#[test]
fn gl_issue_41() {
    use contracts::requires;

    fn foo(f: impl Fn(i32) -> i32) -> i32 {
        // no-op
        f(-10)
    }

    #[requires(true)]
    fn bar() {
        let y = foo(|x: i32| {
            if x < 0 {
                return 0;
            }
            x
        });
        assert_eq!(y, 0);
    }
}
