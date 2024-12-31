use std::{f32, vec};

use rand::Rng;

use crate::{thread_utils::with_rng, traversal::action_history::action::DEFAULT_ACTION_COUNT};

/// Constants for the strategy according to the Discounted CFR paper
const ALPHA: f32 = 1.5;
const BETA: f32 = 0.85;
const GAMMA: f32 = 2.0;

const MIN_SAMPLING_CUTOFF: f32 = 0.01;
const MIN_SAMPLING_ITERATION_CUTOFF: usize = 5000; // After 5000 iterations we don't set a minimum threshold on sampling

/// Strategy struct to hold the current strategy and the sum of all strategies
#[derive(Clone)]
pub struct Strategy {
    pub current_strategy: Vec<f32>, // TODO - Don't hold this in the struct
    strategy_sum: Vec<f32>,
    pub regrets_sum: Vec<f32>,
    pub actions: usize,
    _play_strategy_calculated: bool,
}

impl Strategy {
    /// Create a new Strategy struct
    pub fn new(actions: usize) -> Strategy {
        let mut current_strategy = vec![0.0; DEFAULT_ACTION_COUNT];
        for i in 0..actions {
            current_strategy[i] = 1.0 / actions as f32;
        }
        Strategy {
            current_strategy: current_strategy.clone(),
            strategy_sum: current_strategy,
            regrets_sum: vec![0.0; DEFAULT_ACTION_COUNT],
            actions,
            _play_strategy_calculated: false,
        }
    }

    // update the regrets
    pub fn update_strategy(
        &mut self,
        strategy_utility: f32,
        action_utilities: Vec<f32>,
        iter: usize,
    ) {
        let iterf = iter as f32;

        // normalise the existing regrets sum TODO - move this into a struct which is passed through every iteration
        let iter_coeff = iterf.powf(ALPHA);
        let positive_regret_mulitipler = iter_coeff / (iter_coeff + 1.0);
        let iter_coeff = iterf.powf(BETA);
        let negative_regret_mulitiplier = iter_coeff / (iter_coeff + 1.0);

        for a in 0..self.actions {
            self.regrets_sum[a] += (action_utilities[a] - strategy_utility);
            if self.regrets_sum[a] > 0.0 {
                self.regrets_sum[a] *= positive_regret_mulitipler;
            } else {
                self.regrets_sum[a] *= negative_regret_mulitiplier;
            }
        }
        self.update_current_strategy(iter);
        self.update_strategy_sum_iter(iterf);
    }

    // Update the current strategy based on the instantenous regrets
    fn update_current_strategy(&mut self, iteration: usize) {
        if iteration < MIN_SAMPLING_ITERATION_CUTOFF {
            self.threshold_strategy_update();
        } else {
            self.vanilla_strategy_update();
        }
    }

    // Used in later iterations to concentrate sample space
    fn vanilla_strategy_update(&mut self) {
        let mut normalizing_sum = 0.0;
        for r in 0..self.actions {
            normalizing_sum += self.regrets_sum[r].max(0.0);
        }

        if normalizing_sum > 0.0 {
            for a in 0..self.actions {
                self.current_strategy[a] = self.regrets_sum[a].max(0.0) / normalizing_sum;
            }
        } else {
            // If the normalizing sum is <= 0, then we have to assign equal probability to all actions
            for a in 0..self.actions {
                self.current_strategy[a] = 1.0 / self.actions as f32;
            }
        };
    }

    // Used in early exploration to ensure that all actions are sampled
    fn threshold_strategy_update(&mut self) {
        let mut num_zero = 0;
        let mut normalizing_sum = 0.0;
        for r in 0..self.actions {
            if self.regrets_sum[r] <= 0.0 {
                num_zero += 1;
            } else {
                normalizing_sum += self.regrets_sum[r];
            }
        }

        let new_min = normalizing_sum / ((1.0 / MIN_SAMPLING_CUTOFF) - num_zero as f32);
        normalizing_sum += num_zero as f32 * new_min;

        if normalizing_sum > 0.0 {
            for a in 0..self.actions {
                self.current_strategy[a] = f32::max(self.regrets_sum[a], new_min) / normalizing_sum;
            }
        } else {
            // If the normalizing sum is <= 0, then we have to assign equal probability to all actions
            for a in 0..self.actions {
                self.current_strategy[a] = 1.0 / self.actions as f32;
            }
        };
    }

    // Updates the current strategy, discounting earlier iterations and favouring later ones
    fn update_strategy_sum_iter(&mut self, iter: f32) {
        for index in 0..self.actions {
            // Add a DCRF weighted strategy to the strategy sum
            let contribution = self.current_strategy[index] * ((iter / (iter + 1.0)).powf(GAMMA)); // Weighted according to the iteration using DCRF
            self.strategy_sum[index] += contribution;
        }
    }

    // Provides an action index given the current strategy
    pub fn sample_strategy(&mut self, playing: bool) -> usize {
        self.calculate_play_strategy(playing);
        with_rng(|rng| {
            let mut r = rng.gen_range(0.0..1.0);
            for (i, &prob) in self.current_strategy.iter().enumerate() {
                r -= prob;
                if r <= 0.0 {
                    return i;
                }
            }
            self.actions - 1 // Fallback in case of floating point precision issues
        })
    }

    pub fn get_strategy(&mut self, playing: bool) -> Vec<f32> {
        self.calculate_play_strategy(playing);
        self.current_strategy.clone()
    }

    pub fn calculate_play_strategy(&mut self, playing: bool) {
        if !playing || self._play_strategy_calculated {
            return;
        }
        let mut normalizing_sum = 0.0;

        for r in 0..self.actions {
            normalizing_sum += self.strategy_sum[r];
        }

        if normalizing_sum > 0.0 {
            for a in 0..self.actions {
                self.current_strategy[a] = self.strategy_sum[a] / normalizing_sum;
            }
        } else {
            // If the normalizing sum is <= 0, then we have to assign equal probability to all actions
            for a in 0..self.actions {
                self.current_strategy[a] = 1.0 / self.actions as f32;
            }
        };

        self._play_strategy_calculated = true;
    }
}

// If R(a) is below C, sample the strategy with probability:
// K / [K + C - R(a)]
// K is positive, C is negative
