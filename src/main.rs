use core::num;
use std::collections::{HashMap, HashSet};

use itertools::Itertools;
use models::{card::Rank, Card, Suit};
use rust_poker::validation::validate_strategies;
use subgame_solver::solve::solve_cbr_utilties;
use traversal::{main_train::begin_tree_train_traversal, strategy::strategy_branch::StrategyHubKey};
use validation::validate_strategy_map;

mod evaluate;
mod models;
mod thread_utils;
mod traversal;
mod config;
mod misc_tests;
mod subgame_solver;
mod validation;

fn main() {
        // begin_tree_train_traversal();
        // validate_strategies();
        solve_cbr_utilties();
}