pub const BLUEPRINT_FOLDER: &str = "./blueprint/";

/// Game rules
pub const SMALL_BLIND: u8 = 1;
pub const BIG_BLIND: u8 = 2;
pub const MAX_RAISES: u8 = 4; // Number of raises allowed per round

/// Training configuration
pub const TRAIN_ITERATIONS: usize = 500_000;
pub const ITERATION_UPDATES: usize = TRAIN_ITERATIONS / 10;
pub const NUM_THREADS: usize = 12; // Effectively mulitiplies the iterations for training, but allows for greater concurrency

/// Playtime configuration
pub const PLAY_ITERATIONS: usize = 10; // number of iterations for subgame solving

/// Strategy
pub const MIN_SAMPLING_ITERATION_CUTOFF: usize = TRAIN_ITERATIONS / 2; // Iterations before we apply the min-sampling rule
pub const MIN_SAMPLING_CUTOFF: f64 = 0.01; // Min sampling rule - we ensure that all actions are sampled at least 1% (e.g.) of the time
