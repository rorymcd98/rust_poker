use std::fmt::{Display, Formatter};

use super::{strategy_trait::Strategy, training_strategy::TrainingStrategy};
use crate::traversal::action_history::action::DEFAULT_ACTION_COUNT;

/// A strategy used at runtime for playing / evaluating a game
#[derive(Debug, Clone)]
pub struct PlayStrategy {
    pub actions: usize,
    pub play_strategy: [f64; DEFAULT_ACTION_COUNT],
}

impl Strategy for PlayStrategy {
    fn new(actions: usize) -> PlayStrategy {
        // println!("Strategy wasn't generated at train time, oh no!");
        let mut play_strategy = [0f64; DEFAULT_ACTION_COUNT];
        for a in 0..actions {
            play_strategy[a] = 1.0 / actions as f64;
        }
        play_strategy[0] = 1.0; // TODO - Assess if this at all likely
        PlayStrategy {
            actions: 0,
            play_strategy,
        }
    }

    fn get_current_strategy(&self, _iteration: usize) -> [f64; DEFAULT_ACTION_COUNT] {
        self.play_strategy
    }

    fn from_existing_strategy(
        actions: usize,
        strategy: [f64; DEFAULT_ACTION_COUNT],
    ) -> PlayStrategy {
        PlayStrategy {
            actions,
            play_strategy: strategy,
        }
    }

    fn get_actions(&self) -> usize {
        self.actions
    }
}

impl PlayStrategy {
    pub fn from_train_strategy(train_strategy: TrainingStrategy) -> PlayStrategy {
        let mut normalizing_sum = 0.0;
        let mut return_strategy = [0f64; DEFAULT_ACTION_COUNT];

        for r in 0..train_strategy.actions {
            normalizing_sum += train_strategy.strategy_sum[r];
        }

        if normalizing_sum > 0.0 {
            for a in 0..train_strategy.actions {
                return_strategy[a] = train_strategy.strategy_sum[a] / normalizing_sum;
            }
        } else {
            // If the normalizing sum is <= 0, then we have to assign equal probability to all actions
            for a in 0..train_strategy.actions {
                return_strategy[a] = 1.0 / train_strategy.actions as f64;
            }
        };
        PlayStrategy {
            actions: train_strategy.actions,
            play_strategy: return_strategy,
        }
    }
}

impl Display for PlayStrategy {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self.actions {
                2 => format!(
                    "Check/Fold {:.1}%, Bet/Call {:.1}%",
                    self.play_strategy[0] * 100.0,
                    self.play_strategy[1] * 100.0
                ),
                3 => format!(
                    "Check/Fold {:.1}%, Call {:.1}%, Bet {:.1}%",
                    self.play_strategy[0] * 100.0,
                    self.play_strategy[1] * 100.0,
                    self.play_strategy[2] * 100.0
                ),
                _ => panic!("Unhandled number of actions"),
            }
        )
    }
}
