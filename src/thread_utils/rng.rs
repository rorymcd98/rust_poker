use std::cell::RefCell;
use rand::{rngs::SmallRng, SeedableRng};

thread_local! {
    static RNG: RefCell::<SmallRng> = RefCell::new(SmallRng::from_entropy());
}

pub fn with_rng<T, F: FnOnce(&mut SmallRng) -> T>(f: F) -> T {
    RNG.with(|rng| f(&mut rng.borrow_mut()))
}