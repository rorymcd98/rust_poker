use std::collections::HashMap;
use super::strategy_branch::InfoNode;
use super::strategy_branch::StrategyBranch;

pub struct StrategyMap {
    map: HashMap<InfoNode, StrategyBranch>,
}

impl StrategyMap {
    pub fn new() -> StrategyMap {
        StrategyMap {
            map: HashMap::new(),
        }
    }

    pub fn get_or_create_strategy_branch(&mut self, info_set: InfoNode) -> &mut StrategyBranch {
        self.map.entry(info_set).or_insert(StrategyBranch::new())
    }

    pub fn insert_strategy_branch(&mut self, info_set: InfoNode, strategy_branch: StrategyBranch) {
        self.map.insert(info_set, strategy_branch);
    }
}