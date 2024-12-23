use std::cell::RefCell;

/// Strategy struct to hold the current strategy and the sum of all strategies
pub struct Strategy {
    pub current_strategy: Vec<f64>,
    pub strategy_sum: RefCell::<Vec<f64>>,
}

/// Constants for the strategy according to the Discounted CFR paper
const ALPHA: f64 = 1.5;
const BETA: f64 = 0.0;
const GAMMA: f64 = 2.0;

impl Strategy {
    /// Create a new Strategy struct
    pub fn new(actions: usize) -> Strategy {
        Strategy {
            current_strategy: vec![0.0; actions],
            strategy_sum: RefCell::new(vec![0.0; actions]),
        }
    }

    /// Get the current strategy and updates it on the struct
    pub fn get_current_strategy(&mut self) -> Vec<f64> {
        let mut normalizing_sum = 0.0;
        for a in 0..self.current_strategy.len() {
            if self.current_strategy[a] > 0.0 {
                self.current_strategy[a] = self.current_strategy[a];
            } else {
                self.current_strategy[a] = 0.0;
            }
            normalizing_sum += self.current_strategy[a];
        }

        if normalizing_sum > 0.0 {
            for a in 0..self.current_strategy.len() {            
                self.current_strategy[a] /= normalizing_sum;
            }
        } else {
            // If the normalizing sum is <= 0, then we have to assign equal probability to all actions
            for a in 0..self.current_strategy.len() {
                self.current_strategy[a] = 1.0 / self.current_strategy.len() as f64;
            }
        }
        self.current_strategy.clone()
    }

    /// Updates the current strategy, discounting earlier iterations and favouring later ones
    pub fn update_strategy_sum_iter(&mut self, index: usize, iter: usize) {
        // first update the strategy sum

        // normalise the existing strategy_sum
        let iter = iter as f64;
        let current_strategy_sum = self.strategy_sum.borrow()[index];
        if current_strategy_sum > 0.0 {
            let iter_coeff = iter.powf(ALPHA);
            let factor = iter_coeff / (1.0 + iter_coeff);
            self.strategy_sum.borrow_mut()[index] *= factor;
        } else {
            let iter_coeff = iter.powf(BETA);
            let factor = iter_coeff / (1.0 + iter_coeff);
            self.strategy_sum.borrow_mut()[index] *= factor;
        }

        // then add the new contribution calculated on this iteration
        let contribution = self.current_strategy[index] * ((iter / iter + 1.0).powf(GAMMA)); // Weighted according to the iteration using DCRF
        
        self.strategy_sum.borrow_mut()[index] += self.current_strategy[index] + contribution; 
    }
}