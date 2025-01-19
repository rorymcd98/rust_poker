use core::str;
use std::{collections::HashMap, fmt::Display};
use crate::{models::card::Rank, traversal::action_history::game_abstraction::{to_string_game_abstraction, GameAbstractionSerialised}};

use super::strategy_trait::Strategy;

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Default, Clone, Debug)]
pub struct StrategyHubKey {
    pub low_rank: Rank,
    pub high_rank: Rank,
    pub is_suited: bool,
    pub is_sb: bool,
}

impl Display for StrategyHubKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}{}{}",
            self.low_rank,
            self.high_rank,
            if self.is_suited { "s" } else { "o" },
            if self.is_sb { "SB" } else { "BB" }
        )
    }
}

#[derive(Debug, Default)]
pub struct StrategyBranch<TStrategy> {
    pub strategy_hub_key: StrategyHubKey,
    pub map: HashMap<GameAbstractionSerialised, TStrategy>,
    
    // For debugging
    pub new_generated: usize,
}

impl<TStrategy: Strategy + Clone> StrategyBranch<TStrategy> {
    pub fn new(strategy_key: StrategyHubKey) -> StrategyBranch<TStrategy> {
        StrategyBranch {
            strategy_hub_key: strategy_key,
            map: HashMap::new(),
            new_generated: 0,
        }
    }

    pub fn get_or_create_strategy(
        &mut self,
        info_set: GameAbstractionSerialised,
        num_actions: usize,
    ) -> &mut TStrategy {
        self.map
            .entry(info_set)
            .or_insert_with(|| {
                self.new_generated += 1;
                TStrategy::new(num_actions)
            })
    }

    pub fn get_strategy(
        &self,
        info_set: &GameAbstractionSerialised,
    ) -> Option<&TStrategy> {
        match self.map.get(info_set) {
            Some(strategy) => Some(strategy),
            None => None,
        }
    }

    pub fn get_strategy_or_default(
        &mut self,
        info_set: &GameAbstractionSerialised,
        num_actions: usize,
    ) -> TStrategy {
        match self.map.get(info_set) {
            Some(strategy) => strategy.clone(),
            None => {
                self.new_generated += 1;
                TStrategy::new(num_actions)
            },
        }
    }

    #[allow(dead_code)]
    pub fn print_stats(&self) {
        let mut size_in_mb = 0;
        for (info_set, strategy) in self.map.iter() {
            size_in_mb += std::mem::size_of_val(info_set) + std::mem::size_of_val(strategy);
        }
        println!(
            "Strategy branch for {}, elements: {} (new = {}) size: {} MB,",
            self.strategy_hub_key,
            self.map.len(),
            self.new_generated,
            size_in_mb / 1024 / 1024
        );
    }
}