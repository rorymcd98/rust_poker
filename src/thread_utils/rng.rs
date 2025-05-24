use rand::{rngs::SmallRng, SeedableRng};
use std::cell::RefCell;

thread_local! {
    static RNG: RefCell::<SmallRng> = RefCell::new(SmallRng::from_entropy());
}

/// Lightweight rng object instantiated per-thread
pub fn with_rng<T, F: FnOnce(&mut SmallRng) -> T>(f: F) -> T {
    RNG.with(|rng| f(&mut rng.borrow_mut()))
}
