use core::fmt;

mod rwlock_nb;
pub use rwlock_nb::*;

pub type LockResult<Guard> = Result<Guard, PoisonError<Guard>>;

pub type TryLockResult<Guard> = Result<Guard, TryLockError<Guard>>;

/// NOT YET IMPLEMENTED
#[allow(dead_code)]
pub struct PoisonError<T> {
    guard: T,
}

impl<T> fmt::Debug for PoisonError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PoisonError")
    }
}

#[derive(Debug)]
pub enum TryLockError<T> {
    Poisoned(PoisonError<T>),
    WouldBlock,
}

impl<T> From<PoisonError<T>> for TryLockError<T> {
    #[inline]
    fn from(err: PoisonError<T>) -> TryLockError<T> {
        TryLockError::Poisoned(err)
    }
}
