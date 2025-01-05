use crate::config::*;

use std::f32;

use rand::Rng;

use crate::{thread_utils::with_rng, traversal::action_history::action::DEFAULT_ACTION_COUNT};

use super::strategy_trait::Strategy;

// Provides an action index given the current strategy
pub fn sample_strategy(strategy: &[f32], actions: usize) -> usize {
    with_rng(|rng| {
        let mut r = rng.gen_range(0.0..1.0);
        for (i, &prob) in strategy.iter().enumerate() {
            r -= prob;
            if r <= 0.0 {
                return i;
            }
        }
        actions - 1 // Fallback in case of floating point precision issues
    })
}

/// Constants for the strategy according to the Discounted CFR paper
const ALPHA: f32 = 1.5;
const BETA: f32 = 0.85;
const GAMMA: f32 = 2.0;

/// Strategy struct to hold the current strategy and the sum of all strategies
#[derive(Clone, Debug)]
pub struct TrainingStrategy {
    pub actions: usize,
    pub strategy_sum: [f32; DEFAULT_ACTION_COUNT],
    pub regrets_sum: [f32; DEFAULT_ACTION_COUNT],
}

impl Strategy for TrainingStrategy {
    fn new(actions: usize) -> TrainingStrategy {
        TrainingStrategy {
            actions,
            strategy_sum: [0f32; DEFAULT_ACTION_COUNT],
            regrets_sum: [0f32; DEFAULT_ACTION_COUNT],
        }
    }

    fn get_current_strategy(&self, iteration: usize) -> [f32; DEFAULT_ACTION_COUNT] {
        if iteration < MIN_SAMPLING_ITERATION_CUTOFF {
            self.threshold_strategy_update()
        } else {
            self.vanilla_strategy_update()
        }
    }

    fn from_existing_strategy(actions: usize, strategy: [f32; DEFAULT_ACTION_COUNT]) -> Self {
        TrainingStrategy {
            actions: actions,
            strategy_sum: strategy, // TODO - I did this when removing the current strategy ... not sure if correct
            regrets_sum: [0f32; DEFAULT_ACTION_COUNT],
        }
    }

    fn get_actions(&self) -> usize {
        self.actions
    }
}

impl TrainingStrategy {
    // update the regrets
    pub fn update_strategy(
        &mut self,
        strategy_utility: f32,
        action_utilities: &Vec<f32>,
        iteration: usize,
    ) {
        // println!("updating strategy with utilities, {} . {:?}", strategy_utility, action_utilities);
        let iterf = iteration as f32;

        // Temper the existing regrets sum according to DCRF
        let iter_coeff = iterf.powf(ALPHA);
        let positive_regret_mulitipler = iter_coeff / (iter_coeff + 1.0);
        let iter_coeff = iterf.powf(BETA);
        let negative_regret_mulitiplier = iter_coeff / (iter_coeff + 1.0);

        for a in 0..self.actions {
            self.regrets_sum[a] += action_utilities[a] - strategy_utility;
            if self.regrets_sum[a] > 0.0 {
                self.regrets_sum[a] *= positive_regret_mulitipler;
            } else {
                self.regrets_sum[a] *= negative_regret_mulitiplier;
            }
        }

        if iteration > MIN_SAMPLING_ITERATION_CUTOFF {
            self.update_strategy_sum_iter(iterf);
        }
    }

    // Updates the strategy sum based on the strategy calculated in the last iteration
    fn update_strategy_sum_iter(&mut self, iteration: f32) {
        let current_strategy = self.get_current_strategy(iteration as usize);
        for index in 0..self.actions {
            // Add a DCRF weighted strategy to the strategy sum
            let contribution = current_strategy[index] * ((iteration / (iteration + 1.0)).powf(GAMMA)); // Weighted according to the iteration using DCRF
            self.strategy_sum[index] += contribution;
        }
    }

    // Used in later iterations to concentrate sample space
    fn vanilla_strategy_update(&self) -> [f32; DEFAULT_ACTION_COUNT] {
        let mut return_strategy = [0.0; DEFAULT_ACTION_COUNT];
        let mut normalizing_sum = 0.0;
        for r in 0..self.actions {
            normalizing_sum += self.regrets_sum[r].max(0.0);
        }

        if normalizing_sum > 0.0 {
            for a in 0..self.actions {
                return_strategy[a] = self.regrets_sum[a].max(0.0) / normalizing_sum;
            }
        } else {
            // If the normalizing sum is <= 0, then we have to assign equal probability to all actions
            for a in 0..self.actions {
                return_strategy[a] = 1.0 / self.actions as f32;
            }
        };
        return_strategy
    }

    // Used in early exploration to ensure that all actions are sampled
    fn threshold_strategy_update(&self) -> [f32; DEFAULT_ACTION_COUNT] {
        let mut return_strategy = [0.0; DEFAULT_ACTION_COUNT];
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
                return_strategy[a] = f32::max(self.regrets_sum[a], new_min) / normalizing_sum;
            }
        } else {
            // If the normalizing sum is <= 0, then we have to assign equal probability to all actions
            for a in 0..self.actions {
                return_strategy[a] = 1.0 / self.actions as f32;
            }
        };
        return_strategy
    }
}

// If R(a) is below C, sample the strategy with probability:
// K / [K + C - R(a)]
// K is positive, C is negative
