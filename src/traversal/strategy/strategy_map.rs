use super::strategy_branch::StrategyBranch;
use std::collections::HashMap;

pub struct StrategyMap {
    map: HashMap<u8, StrategyBranch>,
}

impl StrategyMap {
    pub fn new() -> StrategyMap {
        StrategyMap {
            map: HashMap::new(),
        }
    }

    pub fn get_or_create_strategy_branch(&mut self, info_set: u8) -> &mut StrategyBranch {
        self.map.entry(info_set).or_default()
    }

    pub fn insert_strategy_branch(&mut self, info_set: u8, strategy_branch: StrategyBranch) {
        self.map.insert(info_set, strategy_branch);
    }
}
