use crate::traversal::action_history::action::DEFAULT_ACTION_COUNT;

use super::{strategy_trait::Strategy, training_strategy::{self, TrainingStrategy}};

#[derive(Debug)]
pub struct PlayStrategy {
    pub actions: usize,
    pub play_strategy: [f32; DEFAULT_ACTION_COUNT],
}

impl Strategy for PlayStrategy {
    fn new(_actions: usize) -> PlayStrategy {
        panic!("Strategy wasn't generate at train time, oh no!")
    }

    fn get_current_strategy(&self, _iteration: usize) -> [f32; DEFAULT_ACTION_COUNT] {
        self.play_strategy
    }

    fn from_existing_strategy(actions: usize, strategy: [f32; DEFAULT_ACTION_COUNT]) -> PlayStrategy {
        PlayStrategy { actions, play_strategy: strategy }
    }

    fn get_actions(&self) -> usize {
        self.actions
    }
}

impl PlayStrategy {
    pub fn from_train_strategy(train_strategy: TrainingStrategy) -> PlayStrategy {       
        let mut normalizing_sum = 0.0;
        let mut return_strategy = [0f32; DEFAULT_ACTION_COUNT];

        for r in 0..DEFAULT_ACTION_COUNT {
            normalizing_sum += train_strategy.strategy_sum[r];
        }

        if normalizing_sum > 0.0 {
            for a in 0..DEFAULT_ACTION_COUNT {
                return_strategy[a] = train_strategy.strategy_sum[a] / normalizing_sum;
            }
        } else {
            // If the normalizing sum is <= 0, then we have to assign equal probability to all actions
            for a in 0..DEFAULT_ACTION_COUNT {
                return_strategy[a] = 1.0 / train_strategy.actions as f32;
            }
        };
        PlayStrategy { actions: train_strategy.actions, play_strategy: return_strategy }
    }

    pub fn serialise(&self) -> Vec<f32> {
        self.play_strategy.to_vec()
    }
}