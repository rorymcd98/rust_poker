use std::cell::RefCell;
use rand::Rng;

use crate::traversal::action::{self, DEFAULT_ACTION_COUNT};

/// Strategy struct to hold the current strategy and the sum of all strategies
pub struct Strategy {
    pub current_strategy: Vec<f64>,
    strategy_sum: Vec<f64>,
    regrets: Vec<f64>,
    actions: usize,
}

/// Constants for the strategy according to the Discounted CFR paper
const ALPHA: f64 = 1.5;
const BETA: f64 = 0.0;
const GAMMA: f64 = 2.0;

impl Strategy {
    /// Create a new Strategy struct
    pub fn new(actions: usize) -> Strategy {
        let mut current_strategy = vec![0.0; DEFAULT_ACTION_COUNT];
        for i in 0..actions {
            current_strategy[i] = 1.0 / actions as f64;
        }
        Strategy {
            current_strategy: vec![0.0; DEFAULT_ACTION_COUNT],
            strategy_sum: current_strategy,
            regrets: vec![0.0; DEFAULT_ACTION_COUNT],
            actions: actions,
        }
    }

    // update the regres
    pub fn update_strategy(&mut self, strategy_utility: f64, action_utilities: Vec<f64>, iter: usize){
        for a in 0..self.actions {
            self.regrets[a] += action_utilities[a] - strategy_utility;
        }
        self.update_current_strategy();
        self.update_strategy_sum_iter(iter);
    }

    // Update the current strategy based on the instantenous regrets
    fn update_current_strategy(&mut self) {
        let mut normalizing_sum = 0.0;

        for r in 0..self.actions {
            self.regrets[r] = f64::max(self.regrets[r], 0.0);
            normalizing_sum += self.current_strategy[r];
        }

        if normalizing_sum > 0.0 {
            for a in 0..self.actions {            
                self.current_strategy[a] = self.regrets[a] / normalizing_sum;
            }
        } else {
            // If the normalizing sum is <= 0, then we have to assign equal probability to all actions
            for a in 0..self.actions {
                self.current_strategy[a] = 1.0 / self.actions as f64;
            }
        };
    }

    /// Updates the current strategy, discounting earlier iterations and favouring later ones
    fn update_strategy_sum_iter(&mut self, iter: usize) {
        // first update the strategy sum

        let iter = iter as f64;
        for index in 0..self.actions {
            // normalise the existing strategy_sum
            let current_strategy_sum = self.strategy_sum[index];
            if current_strategy_sum > 0.0 {
                let iter_coeff = iter.powf(ALPHA);
                let factor = iter_coeff / (1.0 + iter_coeff);
            self.strategy_sum[index] *= factor;
            } else {
                let iter_coeff = iter.powf(BETA);
                let factor = iter_coeff / (1.0 + iter_coeff);
                self.strategy_sum[index] *= factor;
            }
            
            // then add the new contribution calculated on this iteration
            let contribution = self.current_strategy[index] * ((iter / iter + 1.0).powf(GAMMA)); // Weighted according to the iteration using DCRF
            
            self.strategy_sum[index] += contribution; 
        }
    }

    // Provides an action index given the current strategy
    pub fn sample_strategy(&self) -> usize {
        let mut rng = rand::thread_rng();
        let mut action = 0;
        let mut r = rng.gen_range(0.0..1.0);
        while r > 0.0 {
            r -= self.current_strategy[action];
            action += 1;
        }
        action - 1
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_get_current_strategy_sum() {
//         let mut strategy = Strategy::new(3);
//         strategy.current_strategy = vec![0.2, 0.3, 0.5];
//         let current_strategy = strategy;
//         let sum: f64 = current_strategy.iter().sum();
//         assert!((sum - 1.0).abs() < 1e-6, "Sum of strategy probabilities is not approximately 1.0");
//     }
// }
