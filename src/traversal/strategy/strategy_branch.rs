use std::collections::HashMap;
use crate::traversal::action::Action;

use super::strategy::Strategy;

pub type InfoNode = Vec<u8>;

#[derive(Default)]
pub struct StrategyBranch {
    map: HashMap<InfoNode, Strategy>,
}

impl StrategyBranch {
    pub fn new() -> StrategyBranch {
        StrategyBranch {
            map: HashMap::new(),
        }
    }

    pub fn get_or_create_strategy(&mut self, info_set: InfoNode, actions: usize) -> &mut Strategy {
        self.map.entry(info_set).or_insert(Strategy::new(actions))
    }

    // TODO - implement serialisation of the strategy branch into two streams
    // TODO - implement deserialisation of two streams into strategy branch
}

pub struct StrategyBranchStreamIterator<'a> {
    byte_stream_iterator: std::slice::Iter<'a, f64>,
}

impl<'a> Iterator for StrategyBranchStreamIterator<'a> {

    type Item = Strategy;

    fn next(&mut self) -> Option<Strategy> {
        let first = self.byte_stream_iterator.next();
        if first.is_none() {
            return None;
        }

        // for now we can assume that every strategy is length 3
        let mut strategy = Strategy::new(3);
        strategy.current_strategy[0] = *first.unwrap();

        for i in 1..3 {
            match self.byte_stream_iterator.next() {
                Some(current_strategy) => {
                    strategy.current_strategy[i] = *current_strategy;
                },
                None => panic!("Not enough bytes to deserialise strategy"),
            }
        }

        for i in 0..3 {
            match self.byte_stream_iterator.next() {
                Some(current_strategy) => {
                    strategy.current_strategy[i] = *current_strategy;
                },
                None => panic!("Not enough bytes to deserialise strategy sum"),
            }
        }

        Some(strategy)
    }
}