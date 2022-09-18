use std::ops::{ControlFlow, FromResidual, Try};

/// The result of a decision on whether to remove or keep an element.
///
/// The [`From`] trait is implemented for this trait with the type `bool`,
/// where `true` is [`Retain::Keep`] and `false` is [`Retain::Remove`].
///
/// The [`Try`] trait is implemented for this type, allowing it to be used
/// with the `?` operator. [`RetainDecision::Remove`] is the value that
/// will lead to a short-circuiting early return from the function when
/// used with the `?` operator.
///
/// [`From`]: std::convert::From
/// [`Try`]: https://doc.rust-lang.org/std/ops/trait.Try.html
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum RetainDecision {
    Keep,
    Remove,
}

impl RetainDecision {
    /// Returns `true` if the decision is to keep the value.
    pub fn should_keep(self) -> bool {
        matches!(self, RetainDecision::Keep)
    }

    /// Returns `true` if the decision is to remove the value.
    pub fn should_remove(self) -> bool {
        matches!(self, RetainDecision::Remove)
    }
}

impl From<bool> for RetainDecision {
    fn from(b: bool) -> Self {
        if b {
            Self::Keep
        } else {
            Self::Remove
        }
    }
}

impl FromResidual for RetainDecision {
    fn from_residual(_: <Self as Try>::Residual) -> Self {
        RetainDecision::Remove
    }
}

impl Try for RetainDecision {
    type Output = ();
    type Residual = ();

    fn from_output(_: Self::Output) -> Self {
        RetainDecision::Keep
    }

    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        match self {
            RetainDecision::Keep => ControlFlow::Continue(()),
            RetainDecision::Remove => ControlFlow::Break(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::assert_matches::assert_matches;
    use std::sync::atomic::AtomicUsize;

    use super::*;

    #[test]
    fn test_from() {
        assert_matches!(RetainDecision::from(true), RetainDecision::Keep);
        assert_matches!(RetainDecision::from(false), RetainDecision::Remove);
        assert_matches!(Into::<RetainDecision>::into(true), RetainDecision::Keep);
        assert_matches!(Into::<RetainDecision>::into(false), RetainDecision::Remove);
    }

    #[test]
    fn test_other() {
        assert!(RetainDecision::Keep.should_keep());
        assert!(RetainDecision::Remove.should_remove());
        assert_ne!(RetainDecision::Keep, RetainDecision::Remove);

        assert_matches!(RetainDecision::from_output(()), RetainDecision::Keep);
        assert_matches!(RetainDecision::from_residual(()), RetainDecision::Remove);
    }

    #[test]
    fn test_try() {
        /*
        This is about the constellation we have with the plain 'retain-loop' in
        server implementations. Within the server implementation, we have a few
        checks, and if any of them fail, we want to remove the (for example) client,
        and do an early exit. This is, why for the RetainDecision, we can short circuit
        with the '?'-operator.

        In this test, we test, that the operator actually short circuits before the
        seventh element, and touches the first six.

        To map this test to one of the constellations described above:
        `check` is any server implementor's check, for example, if UUIDs collide.
        `exec` is the enclosing `clients.retain(...)` loop, from which we want to
        early exit, if `check` fails.
         */

        let reached_count = AtomicUsize::new(0);
        let check = |n: i32| -> RetainDecision {
            if n >= 6 {
                // 5 can still be checked, but 6 can't
                panic!("should short circuit after 5");
            }

            let _ = &reached_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

            if n < 5 {
                RetainDecision::Keep
            } else {
                RetainDecision::Remove
            }
        };
        fn exec(check: impl Fn(i32) -> RetainDecision) -> RetainDecision {
            let data = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
            for i in data {
                check(i)?;
            }
            RetainDecision::Keep
        }
        assert_matches!(exec(check), RetainDecision::Remove);
        assert_eq!(6, reached_count.load(std::sync::atomic::Ordering::SeqCst));
    }
}
