use std::collections::HashMap;
use crate::{models::card::Rank, traversal::action_history::game_abstraction::GameAbstractionSerialised};

use super::strategy_trait::Strategy;

#[derive(PartialEq, Eq, Hash, Default, Clone, Debug)]
pub struct StrategyHubKey {
    pub low_rank: Rank,
    pub high_rank: Rank,
    pub is_suited: bool,
    pub is_sb: bool,
}

#[derive(Debug, Default)]
pub struct StrategyBranch<TStrategy> {
    pub strategy_hub_key: StrategyHubKey,
    pub map: HashMap<GameAbstractionSerialised, TStrategy>,
}

impl<TStrategy: Strategy> StrategyBranch<TStrategy> {
    pub fn new(strategy_map_element: StrategyHubKey) -> StrategyBranch<TStrategy> {
        StrategyBranch {
            strategy_hub_key: strategy_map_element,
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
}