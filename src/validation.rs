use std::collections::HashMap;

use crate::{config::BLUEPRINT_FOLDER, models::{card::{all_pocket_pairs, all_rank_combos, new_random_nine_card_game_with}, Card, Player, Suit}, traversal::{action_history::{action::DEFAULT_ACTION_COUNT, game_abstraction::{convert_deal_into_abstraction, GameAbstractionSerialised}}, strategy::{play_strategy::PlayStrategy, strategy_branch::{StrategyBranch, StrategyHubKey}, strategy_hub::deserialise_strategy_hub, strategy_trait::Strategy, training_strategy::TrainingStrategy}}};

pub fn validate_strategies(){
    let mut strategy_map: HashMap<StrategyHubKey, StrategyBranch<TrainingStrategy>> = deserialise_strategy_hub(BLUEPRINT_FOLDER).unwrap();
    validate_strategy_map::<TrainingStrategy>(&mut strategy_map);
}

pub fn validate_strategy_map<TStrategy: Strategy>(strategy_map: &HashMap<StrategyHubKey, StrategyBranch<TrainingStrategy>>) {
    for abstraction in generate_preflop_abstractions() {
        let strategy_branch = strategy_map.get(&abstraction.0).unwrap();
        strategy_branch.print_stats();
        let default_strategy = TrainingStrategy::new(DEFAULT_ACTION_COUNT);
        let strategy = strategy_branch.get_strategy(&abstraction.1).unwrap_or(&default_strategy);
        println!("{}: strat {:?}, regrets {:?}", abstraction.0, PlayStrategy::from_train_strategy(strategy.clone()).get_current_strategy(100000), strategy.regrets_sum);
    }
}


// Generate all offsuit SB abstractions
pub fn generate_preflop_abstractions() -> Vec<(StrategyHubKey, GameAbstractionSerialised)> {
    let mut game_abstractions = Vec::new();
    let mut combos = all_rank_combos();
    combos.extend(all_pocket_pairs());
    for cards in combos {
        let card1 = Card::new(Suit::Spades, cards.0);
        let card2 = Card::new(Suit::Clubs, cards.1);
        let deal = new_random_nine_card_game_with(card1, card2, Card::default(), Card::default());
        let game_abstraction = convert_deal_into_abstraction(deal);
        let key = StrategyHubKey {
            low_rank: card1.rank,
            high_rank: card2.rank,
            is_suited: false,
            is_sb: true,
        };
        // The first preflop action
        let serialised = game_abstraction.get_abstraction(0, 2, 1, &Player::Traverser);
        game_abstractions.push((key, serialised));
    }
    game_abstractions
}
