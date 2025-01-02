use std::collections::HashMap;
use crate::{models::card::Rank, traversal::action_history::game_abstraction::GameAbstractionSerialised};

use super::strategy_trait::Strategy;

#[derive(PartialEq, Eq, Hash, Default, Clone, Debug)]
pub struct StrategyHubElement {
    pub low_rank: Rank,
    pub high_rank: Rank,
    pub is_suited: bool,
    pub is_sb: bool,
}

#[derive(Debug)]
pub struct StrategyBranch<TStrategy> {
    pub strategy_hub_element: StrategyHubElement,
    map: HashMap<GameAbstractionSerialised, TStrategy>,
}

impl<TStrategy: Strategy> StrategyBranch<TStrategy> {
    pub fn new(strategy_map_element: StrategyHubElement) -> StrategyBranch<TStrategy> {
        StrategyBranch {
            strategy_hub_element: strategy_map_element,
            map: HashMap::new(),
        }
    }

    pub fn get_or_create_strategy(
        &mut self,
        info_set: GameAbstractionSerialised,
        num_actions: usize,
    ) -> &mut TStrategy {
        self.map
            .entry(info_set)
            .or_insert_with(|| TStrategy::new(num_actions))
    }

    #[allow(dead_code)]
    pub fn print_stats(&self) {
        let mut size_in_mb = 0;
        for (info_set, strategy) in self.map.iter() {
            size_in_mb += std::mem::size_of_val(info_set) + std::mem::size_of_val(strategy);
        }
        println!(
            "Strategy branch, elements: {} size: {} MB",
            self.map.len(),
            size_in_mb / 1024 / 1024
        );
    }

    // TODO - implement serialisation of the strategy branch into two streams
    // TODO - implement deserialisation of two streams into strategy branch
}



// pub struct StrategyBranchStreamIterator<'a> {
//     byte_stream_iterator: std::slice::Iter<'a, f32>,
// }

// impl Iterator for StrategyBranchStreamIterator<'_> {
//     type Item = Strategy;

//     fn next(&mut self) -> Option<Strategy> {
//         let first = self.byte_stream_iterator.next();
//         first?;

//         // for now we can assume that every strategy is length 3
//         let mut strategy = Strategy::new(3);
//         strategy.current_strategy[0] = *first.unwrap();

//         for i in 1..3 {
//             match self.byte_stream_iterator.next() {
//                 Some(current_strategy) => {
//                     strategy.current_strategy[i] = *current_strategy;
//                 }
//                 None => panic!("Not enough bytes to deserialise strategy"),
//             }
//         }

//         for i in 0..3 {
//             match self.byte_stream_iterator.next() {
//                 Some(current_strategy) => {
//                     strategy.current_strategy[i] = *current_strategy;
//                 }
//                 None => panic!("Not enough bytes to deserialise strategy sum"),
//             }
//         }

//         Some(strategy)
//     }
// }
