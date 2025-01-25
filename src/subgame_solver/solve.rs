use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::vec;

use dashmap::DashMap;
use itertools::Itertools;
use rand::seq::SliceRandom;
use rust_poker::config::{BIG_BLIND, BLUEPRINT_FOLDER};

use crate::evaluate::evaluate_hand::HandEvaluator;
use crate::models::card::{NineCardDeal, Rank};
use crate::models::{Card, Player, Suit};
use crate::thread_utils::with_rng;
use crate::traversal::action_history::action::{self, Action, DEFAULT_ACTION_COUNT};
use crate::traversal::action_history::card_round_abstraction::CardRoundAbstractionSerialised;
use crate::traversal::action_history::game_abstraction::{self, convert_deal_into_abstraction, to_string_game_abstraction, GameAbstraction, GameAbstractionSerialised};
use crate::traversal::game_state::game_state_helper::{GameStateHelper, EVALUATOR};
use crate::traversal::game_state::terminal_state::TerminalState;
use crate::traversal::main_train::{get_all_combos_by_blind, get_unique_cards};
use crate::traversal::strategy::play_strategy::PlayStrategy;
use crate::traversal::strategy::strategy_branch::{StrategyBranch, StrategyHubKey};
use crate::traversal::strategy::strategy_hub::{deserialise_strategy_hub, StrategyHub};
use crate::traversal::strategy::strategy_trait::Strategy;
use std::time::Instant;

#[derive(Clone)]
struct GameTreePath {
    pub abstraction: GameAbstraction,
    pub evaluation: Option<Player>,
    pub strategy_hub_key: StrategyHubKey,
    pub weight: usize,
}

type GameTreePathKey = (StrategyHubKey, u8, [CardRoundAbstractionSerialised; 4], [CardRoundAbstractionSerialised; 4]);

impl GameTreePath {
    pub fn get_game_path_key(&self) -> GameTreePathKey {
        (
            self.strategy_hub_key.clone(),
            match self.evaluation {
                None => 0,
                Some(Player::Traverser) => 1,
                Some(Player::Opponent) => 2,
            },
            self.abstraction.traverser_round_abstractions.clone(),
            self.abstraction.opponent_round_abstractions.clone(),
        )
    }
}

struct GameStateFromActions {
    partial_deal: NineCardDeal,
    traverser_pot: u8,
    opponent_pot: u8,
    cards_dealt: u8,
    small_blind_player: Player,
    big_blind_player: Player,
    bets_this_round: u8,
    checks_this_round: u8,
    current_player: Player,
}

// Reach MaxMargin
// Generate a game state
// Look at the preceeding node 
// Generate all the game state that could have lead to this node (169 states)
// For each of these histories calculate the total gift in this manner: 
// 1. Calculate the CBV which can be done using vanilla CFR, multiplying the strategy by the utility for each action
// 2. Calculate the gift by finding the max of CBV(I) - CBV(I, a) for all a in A(I)

pub fn solve_cbr_utilties() {
    let strategy_hub = deserialise_strategy_hub::<PlayStrategy>(BLUEPRINT_FOLDER).unwrap();
    let opponent_strategy_map = strategy_hub.into_iter().collect::<DashMap<StrategyHubKey, StrategyBranch<_>>>();

    let player_hand = [Card::new(Suit::Spades, Rank::King), Card::new(Suit::Clubs, Rank::King)];


    let traverser_key = StrategyHubKey {
        low_rank: player_hand[0].rank,
        high_rank: player_hand[1].rank,
        is_suited: player_hand[0].suit == player_hand[1].suit,
        is_sb: true,
    };

    let traverser_strategy_branch = opponent_strategy_map.remove(&traverser_key).unwrap().1;

    // Look for the gifts give on the Bet node
    let action_history = &vec![
        Action::Bet,  Action::Bet, Action::Bet,Action::Bet, Action::Call,
        Action::Deal(Card::new(Suit::Clubs, Rank::Three)), Action::Deal(Card::new(Suit::Spades, Rank::Nine)), Action::Deal(Card::new(Suit::Hearts, Rank::Queen)),
        Action::CheckFold, //Action::Bet, Action::Call, 
        // Action::Deal(Card::new(Suit::Diamonds, Rank::Five)),
        // Action::CheckFold, Action::Bet, Action::Call, 
        // Action::Deal(Card::new(Suit::Clubs, Rank::Six)),
        // Action::Bet, Action::Bet, Action::Bet//, Action::Bet, Action::Call
    ];

    let sb_player = Player::Traverser;
    let game_state = &mut convert_actions_to_game_state(&action_history, sb_player);

    let mut existing_cards = action_history.iter().filter_map(|action| {
        if let Action::Deal(card) = action {
            Some(card.to_int())
        } else {
            None
        }
    }).collect_vec();

    existing_cards.push(player_hand[0].to_int());
    existing_cards.push(player_hand[1].to_int());

    let mut remaining_cards = (0..52).collect::<HashSet<_>>();
    for card in existing_cards.iter() {
        remaining_cards.remove(card);
    }

    let cards_dealt = game_state.cards_dealt.get();
    let potential_card_histories = remaining_cards.iter().permutations(2 + (5 - cards_dealt) as usize).collect::<Vec<_>>();
    let potential_card_histories: HashSet<Vec<Card>> = potential_card_histories.iter().map(|card_ints| card_ints.iter().map(|card_int| Card::from_int(**card_int)).collect()).map(|card_history: Vec<Card>| {
        let mut cards = vec![card_history[0], card_history[1]];
        cards.sort(); // Sort the first two cards
        if cards_dealt == 0 {
            let mut sorted_flop = [card_history[2], card_history[3], card_history[4]];
            sorted_flop.sort();
            cards.extend(sorted_flop);
        }
        for card in card_history.iter().skip(cards.len()) {
            cards.push(*card);
        }
        cards
    }).collect();

    let mut deal = [Card::default(); 9];
    let mut card_index = 0;
    for action in action_history {
        if let Action::Deal(card) = action {
            deal[4 + card_index] = *card;
            card_index += 1;
        }
    }

    deal[0] = player_hand[0];
    deal[1] = player_hand[1];

    // group by abstraction and outcome
    let mut all_game_abstractions = Vec::with_capacity(potential_card_histories.len());
    let mut all_evaluations = Vec::with_capacity(potential_card_histories.len());
    let mut opponent_strategy_hub_keys = Vec::with_capacity(potential_card_histories.len());

    for cards in potential_card_histories {
        deal[2] = cards[0];
        deal[3] = cards[1];
        for (i, &card) in cards.iter().enumerate().skip(2) {
            deal[i + 2 + cards_dealt as usize] = card;
        }
        
        let game_abstraction = convert_deal_into_abstraction(deal);
        all_game_abstractions.push(game_abstraction);
        all_evaluations.push(EVALUATOR.evaluate_deal(&deal));
        let key = StrategyHubKey {
            low_rank: deal[2].rank,
            high_rank: deal[3].rank,
            is_suited: deal[2].suit == deal[3].suit,
            is_sb: sb_player == Player::Opponent,
        };
        opponent_strategy_hub_keys.push(key);
    }

    let mut game_tree_paths = all_game_abstractions.iter().zip(all_evaluations.iter()).zip(opponent_strategy_hub_keys.iter()).map(|((abstraction, evaluation), key)| {
        GameTreePath {
            abstraction: abstraction.clone(),
            evaluation: *evaluation,
            strategy_hub_key: key.clone(),
            weight: 1,
        }
    }).collect::<Vec<_>>();

    // Reduce the game tree paths to unique paths (same abstraction and evaluation)
    let mut path_map: HashMap<GameTreePathKey, GameTreePath> = HashMap::new();
    println!("Game tree paths: {}", game_tree_paths.len());
    
    for path in game_tree_paths {
        path_map.entry(path.get_game_path_key()).and_modify(|existing_path| {
            if (existing_path.strategy_hub_key != path.strategy_hub_key) || (existing_path.evaluation != path.evaluation) || (existing_path.abstraction.opponent_round_abstractions != path.abstraction.opponent_round_abstractions) || (existing_path.abstraction.traverser_round_abstractions != path.abstraction.traverser_round_abstractions) {
                panic!("Strategy hub key or evaluation mismatch");
            }
            existing_path.weight += 1;
        }).or_insert(path);
    }

    let mut game_tree_paths = path_map.into_iter().map(|(_, path)| path).collect::<Vec<_>>();

    println!("Game tree paths after reduction: {}", game_tree_paths.len());
    println!("Sum of weights after reduction: {}", game_tree_paths.iter().map(|path| path.weight).sum::<usize>());

    // Sort the opponent game paths according to the abstraction to increase the likelihood of cache hits
    game_tree_paths.sort_by(|a, b| {
        a.get_game_path_key().cmp(&b.get_game_path_key())
    });

    let mut cbv_solver = CbvSolver{
        traverser_strategy: traverser_strategy_branch,
        opponent_strategy_map,
        game_state,

        game_tree_paths,

        trav_seen: Cell::new(0),
        opp_seen: Cell::new(0),
        trav_not_seen: Cell::new(0),
        opp_not_seen: Cell::new(0),
    };


    let start = Instant::now();
    let utility = cbv_solver.calculate_cbv(&action_history);
    let duration = start.elapsed();
    println!("First calculation took: {:?}", duration);

    let start = Instant::now();
    let utility = cbv_solver.calculate_cbv(&action_history);
    let duration = start.elapsed();
    println!("Second calculation took: {:?}", duration);

    let start = Instant::now();
    let utility = cbv_solver.calculate_cbv(&action_history);
    let duration = start.elapsed();
    println!("Third calculation took: {:?}", duration);
}

type CbvReturn = Vec<f64>; // Utilitiy for each deal

struct CbvSolver<'a> {
    traverser_strategy: StrategyBranch<PlayStrategy>,
    opponent_strategy_map: DashMap<StrategyHubKey, StrategyBranch<PlayStrategy>>,
    game_state: &'a mut GameStateHelper,

    game_tree_paths: Vec<GameTreePath>,

    trav_seen: Cell<u32>,
    opp_seen: Cell<u32>,
    trav_not_seen: Cell<u32>,
    opp_not_seen: Cell<u32>,
}
// println!("Trav seen: {}, Trav not seen: {}", self.trav_seen.get(), self.trav_not_seen.get());
// println!("Opp seen: {}, Opp not seen: {}", self.opp_seen.get(), self.opp_not_seen.get());

impl<'a> CbvSolver<'a> {
    pub fn calculate_cbv(&mut self, action_history: &Vec<Action>) -> f64 { // TODO - move Vec<Action> to the struct
        let initial_reaches = self.calculate_initial_reaches(action_history);
        println!("Initial reaches which are 0: {}", initial_reaches.iter().filter(|reach| **reach == 0.0).count());
        let reaches_sum = initial_reaches.iter().sum::<f64>();
        println!("reaches sum {}", reaches_sum);

        let traverser_utility = self.traverse_action(&initial_reaches); // need to weight by reaches

            
        let res = traverser_utility.iter().zip(initial_reaches.iter()).map(|(u, r)| u * r).sum::<f64>() / reaches_sum;
        println!("Utility: {}", res);
        with_rng(|rng|
            self.game_tree_paths.shuffle(rng)
        );
        res
    }

    fn calculate_initial_reaches(&self, action_history: &Vec<Action>) -> Vec<f64> {
        let path_weight_sum = self.game_tree_paths.iter().map(|path| path.weight).sum::<usize>() as f64;
        let mut reaches = self.game_tree_paths.iter().map(|path| path.weight as f64 / path_weight_sum).collect::<Vec<_>>();

        let game_state = GameStateHelper::new(self.game_state.cards, self.game_state.small_blind_player);

        for action in action_history {
            let round = (game_state.cards_dealt.get()).saturating_sub(2) as usize;
            let current_player_pot = game_state.get_current_player_pot();
            let bets_this_round = game_state.bets_this_round.get();
            let num_available_actions = game_state.get_num_available_actions();

            match action {
                Action::Deal(_) => {
                    game_state.deal();
                },
                Action::Bet => {
                    match game_state.current_player.get() {
                        Player::Traverser => {
                            let strategy = self.get_traverser_strategy(0, round, current_player_pot, bets_this_round, num_available_actions);
                            reaches.iter_mut().enumerate().filter(|(_, reach)| **reach != 0.0).for_each(|(_, reach)| {
                                let action_probability = strategy.0[2.min(num_available_actions-1)];
                                *reach *= action_probability;
                            });
                        },
                        Player::Opponent => {
                            reaches.iter_mut().enumerate().filter(|(_, reach)| **reach != 0.0).for_each(|(deal_index, reach)| {
                                let strategy = self.get_opponent_strategy(deal_index, round, current_player_pot, bets_this_round, num_available_actions);
                                let action_probability = strategy.0[2.min(num_available_actions-1)];
                                *reach *= action_probability;
                            });
                        }
                    }
                    game_state.bet();
                    game_state.switch_current_player();
                },
                Action::Call => {
                    match game_state.current_player.get() {
                        Player::Traverser => {
                            let strategy = self.get_traverser_strategy(0, round, current_player_pot, bets_this_round, num_available_actions);
                            reaches.iter_mut().enumerate().filter(|(_, reach)| **reach != 0.0).for_each(|(_, reach)| {
                                let action_probability = strategy.0[1];
                                *reach *= action_probability;
                            });
                        },
                        Player::Opponent => {
                            reaches.iter_mut().enumerate().filter(|(_, reach)| **reach != 0.0).for_each(|(deal_index, reach)| {
                                let strategy = self.get_opponent_strategy(deal_index, round, current_player_pot, bets_this_round, num_available_actions);
                                let action_probability = strategy.0[1];
                                *reach *= action_probability;
                            });
                        }
                    }
                    game_state.call();
                    game_state.switch_current_player();
                },
                Action::CheckFold => {
                    match game_state.current_player.get() {
                        Player::Traverser => {
                            // TODO - Check if the strategy is identical for all deals here - it should be
                            let strategy = self.get_traverser_strategy(0, round, current_player_pot, bets_this_round, num_available_actions);
                            reaches.iter_mut().enumerate().filter(|(_, reach)| **reach != 0.0).for_each(|(_, reach)| {
                                let action_probability = strategy.0[0];
                                *reach *= action_probability;
                            });
                        },
                        Player::Opponent => {
                            reaches.iter_mut().enumerate().filter(|(_, reach)| **reach != 0.0).for_each(|(deal_index, reach)| {
                                let strategy = self.get_opponent_strategy(deal_index, round, current_player_pot, bets_this_round, num_available_actions);
                                let action_probability = strategy.0[0];
                                *reach *= action_probability;
                            });
                        }
                    }
                    game_state.checkfold();
                    game_state.switch_current_player();
                },
                _ => {}
            }
        }
        reaches
    }

    pub fn evaluate_showdown(&self, deal_index: usize, traverser_pot: f64, opponent_pot: f64) -> f64 {
        let winner = self.game_tree_paths[deal_index].evaluation;
        match winner {
            Some(Player::Traverser) => opponent_pot,
            Some(Player::Opponent) => -traverser_pot,
            None => 0.0,
        }
    }
    
    pub fn evaluate_fold(&self, current_player: &Player, traverser_pot: f64, opponent_pot: f64) -> f64 {
        match current_player {
            Player::Traverser => opponent_pot,
            Player::Opponent => -traverser_pot,
        }
    }

    fn traverse_action(&mut self, reaches: &Vec<f64>) -> CbvReturn {
        let num_available_actions = self.game_state.get_num_available_actions();
        
        // Cache these
        let traverser_pot = self.game_state.traverser_pot.get() as f64;
        let opponent_pot = self.game_state.opponent_pot.get() as f64;
        let current_player = self.game_state.current_player.get();
        
        match self.game_state.check_round_terminal() {
            TerminalState::Showdown => {
                let mut cbv_return = reaches.clone();
                cbv_return.iter_mut().enumerate().filter(|(_, reach)| **reach != 0.0).for_each(|(deal_index, reach)| {
                    // println!("reach: {}", reach);
                    let showdown_utility = self.evaluate_showdown(deal_index, traverser_pot, opponent_pot);
                    *reach *= showdown_utility;
                });
                return cbv_return;
            },
            TerminalState::Fold => {
                // Don't need to iterate over the game abstractions here because:
                // (n*A + n*B + n*C) / (A + B + C) = n
                // Where n is the (constant for this node) fold utility for this infoset and A, B, C are the reaches
                return vec![self.evaluate_fold(&current_player, traverser_pot, opponent_pot); self.game_tree_paths.len()];
            }
            TerminalState::RoundOver => {
                if self.game_state.is_preflop() {
                    return self.traverse_flop(reaches)
                } else {
                    return self.traverse_deal(reaches)
                };
            }
            TerminalState::None => {
                let round = (self.game_state.cards_dealt.get()).saturating_sub(2) as usize;
                let bets_this_round = self.game_state.bets_this_round.get();
                let pot_before_action = self.game_state.get_current_player_pot();
                let bets_before_action = self.game_state.bets_this_round.get();
                let previous_player = self.game_state.current_player.get();
                let checks_before = self.game_state.checks_this_round.get();

                //// Here we're calculating CBV as described in Safe and Nested Subgame Solving for Imperfect-Information Games
                return self.action_value(reaches, num_available_actions, current_player, round, bets_this_round, pot_before_action, bets_before_action, previous_player, checks_before)
            },
        };
    }

    fn action_value(&mut self, reaches: &Vec<f64>, num_available_actions: usize, current_player: Player, round: usize, bets_this_round: u8, pot_before_action: u8, bets_before_action: u8, previous_player: Player, checks_before: u8) -> CbvReturn {
        let mut reaches_for_actions = vec![vec![0.0; self.game_tree_paths.len()]; num_available_actions];
        let mut action_probabilities = vec![[0f64; DEFAULT_ACTION_COUNT]; self.game_tree_paths.len()];
        
        let (mut previous_strategy, mut serialised_abstraction) = match current_player {
        Player::Traverser => self.get_traverser_strategy(0, round, pot_before_action, bets_this_round, num_available_actions),
            Player::Opponent => self.get_opponent_strategy(0, round, pot_before_action, bets_this_round, num_available_actions),
        };
        let mut previous_strategy_key = &self.game_tree_paths[0].strategy_hub_key;

        for (deal_index, reach) in reaches.iter().enumerate().filter(|(_, reach)| **reach != 0.0) {
            previous_strategy = match current_player {
                Player::Traverser => self.get_traverser_strategy_cache(deal_index, round, num_available_actions, &mut serialised_abstraction, previous_strategy),
                Player::Opponent => {
                    self.get_opponent_strategy_cache(deal_index, round, num_available_actions, previous_strategy_key, &mut serialised_abstraction, previous_strategy)
                }
            };
            previous_strategy_key = &self.game_tree_paths[deal_index].strategy_hub_key;
            let strategy = previous_strategy;

            // the safer way to get strategy
            // let strategy = match current_player {
            //     Player::Traverser => self.get_traverser_strategy(deal_index, round, pot_before_action, bets_this_round, num_available_actions),
            //     Player::Opponent => self.get_opponent_strategy(deal_index, round, pot_before_action, bets_this_round, num_available_actions),
            // }.0;

            for action in 0..num_available_actions {
                // store the reaches for each action
                reaches_for_actions[action][deal_index] = reach * strategy[action];
            }
            action_probabilities[deal_index] = strategy;
        }
        
        let mut action_abstraction_utilities = vec![Vec::with_capacity(self.game_tree_paths.len()); num_available_actions]; // TODO - Should use option
        for action in 0..num_available_actions {
            let next_reaches = &reaches_for_actions[action];
            // println!("Playing action {}/{}, player {}, round {}, pot {}, bets {}, checks {}", action, num_available_actions, current_player, round, pot_before_action, bets_before_action, checks_before);
            action_abstraction_utilities[action] = self.traverse_chosen_action(next_reaches, action, previous_player, pot_before_action, bets_before_action, checks_before);
        }

        if self.game_state.current_player.get() == Player::Traverser {
            let mut max_utility = f64::NEG_INFINITY;
            let mut best_action_utilities = vec![0.0; self.game_tree_paths.len()];
            let mut best_action_count = 0;

            for action in 0..num_available_actions {
                let mut reach_utility = 0.0;
                let mut total_reach = 0.0;
                let abstraction_utilities = &action_abstraction_utilities[action];
                for deal_index in 0..self.game_tree_paths.len() {
                    let utility = abstraction_utilities[deal_index];
                    let reach = reaches[deal_index];
                    reach_utility += reach * utility;
                    total_reach += reach;
                }
                let action_utility = reach_utility / total_reach;
                if action_utility > max_utility {
                    max_utility = action_utility;
                    best_action_utilities = abstraction_utilities.clone();
                    best_action_count = 1;
                } else if action_utility == max_utility {
                    for deal_index in 0..self.game_tree_paths.len() {
                        best_action_utilities[deal_index] += abstraction_utilities[deal_index];
                    }
                    best_action_count += 1;
                }
            }

            if best_action_count > 1 {
                for deal_index in 0..self.game_tree_paths.len() {
                    best_action_utilities[deal_index] /= best_action_count as f64;
                }
            }
            return best_action_utilities;
        } else {
            let mut action_utilities = vec![0.0; self.game_tree_paths.len()];
            for action in 0..num_available_actions {
                let abstraction_utilities = &action_abstraction_utilities[action];
                for deal_index in 0..self.game_tree_paths.len() {
                    action_utilities[deal_index] += abstraction_utilities[deal_index] * action_probabilities[deal_index][action];
                }
            }
            return action_utilities;
        };
    }

    fn traverse_chosen_action(&mut self, reaches: &Vec<f64>, action: usize, acting_player: Player, current_pot: u8, current_bets: u8, current_checks: u8) -> CbvReturn {
        match action {
            0 => self.game_state.checkfold(),
            1 => self.game_state.call_or_bet(),
            2 => self.game_state.bet(),
            _ => {}
        };
        self.game_state.switch_current_player();
        let utility = self.traverse_action(reaches);
        self.game_state.undo(
            acting_player,
            current_pot,
            current_bets,
            current_checks,
        );
        utility
    }

    fn traverse_flop(&mut self, reaches: &Vec<f64>) -> CbvReturn {
        let previous_player = self.game_state.current_player.get();
        let previous_bets = self.game_state.bets_this_round.get();
        let check_before = self.game_state.checks_this_round.get();

        self.game_state.deal_flop();
        let utility = self.traverse_action(reaches);
        self.game_state
            .undeal_flop(previous_bets, previous_player, check_before);
        utility
    }

    fn traverse_deal(&mut self, reaches: &Vec<f64>) -> CbvReturn {
        let previous_player = self.game_state.current_player.get();
        let previous_bets = self.game_state.bets_this_round.get();
        let checks_before = self.game_state.checks_this_round.get();

        self.game_state.deal();
        let utility = self.traverse_action(reaches);

        self.game_state
            .undeal(previous_bets, previous_player, checks_before);
        utility
    }

    fn get_opponent_strategy(&self, deal_index: usize, round: usize, current_player_pot: u8, bets_this_round: u8, num_available_actions: usize) -> ([f64; DEFAULT_ACTION_COUNT], GameAbstractionSerialised) {
        let game_abstraction = &self.game_tree_paths[deal_index].abstraction;
        // TODO - massively inefficient to regenerate this every time
        let serialised_abstraction = game_abstraction.get_abstraction(
            round,
            current_player_pot,
            bets_this_round,
            &Player::Opponent, 
        );

        let strategy_hub_key = &self.game_tree_paths[deal_index].strategy_hub_key;
        
        let strategy= self.opponent_strategy_map
            .get(&strategy_hub_key)
            .and_then(|strategy_branch| strategy_branch.get_strategy(&serialised_abstraction).cloned());
    
        let strategy = match strategy {
            Some(strategy) => strategy.get_current_strategy(0),
            None => [1.0 / (num_available_actions as f64); DEFAULT_ACTION_COUNT],
        };
        (strategy, serialised_abstraction)
    }

    fn get_opponent_strategy_cache(&self, deal_index: usize, round: usize, num_available_actions: usize, previous_strategy_key: &StrategyHubKey, previous_game_abstraction_ser: &mut GameAbstractionSerialised, previous_strategy: [f64; DEFAULT_ACTION_COUNT]) -> [f64; DEFAULT_ACTION_COUNT] {
        let game_abstraction = &self.game_tree_paths[deal_index].abstraction;
        // TODO - massively inefficient to regenerate this every time
        let identical = game_abstraction.replace_round_abstraction(
            previous_game_abstraction_ser,
            round,
            &Player::Opponent, 
        );

        if identical && previous_strategy_key == &self.game_tree_paths[deal_index].strategy_hub_key {
            return previous_strategy;
        }

        let strategy_hub_key = &self.game_tree_paths[deal_index].strategy_hub_key;
        
        let strategy= self.opponent_strategy_map
            .get(&strategy_hub_key)
            .and_then(|strategy_branch| strategy_branch.get_strategy(&previous_game_abstraction_ser).cloned());
    
        let strategy = match strategy {
            Some(strategy) => strategy.get_current_strategy(0),
            None => [1.0 / (num_available_actions as f64); DEFAULT_ACTION_COUNT],
        };
        strategy
    }

    fn get_traverser_strategy(&self, deal_index: usize, round: usize, current_player_pot: u8, bets_this_round: u8, num_available_actions: usize) -> ([f64; DEFAULT_ACTION_COUNT], GameAbstractionSerialised) {
        let game_abstraction = &self.game_tree_paths[deal_index].abstraction;
        let serialised_abstraction = game_abstraction.get_abstraction(
            round,
            current_player_pot,
            bets_this_round,
            &Player::Traverser, 
        );

        let strategy = self.traverser_strategy
                .get_strategy(&serialised_abstraction);

        let strategy = match strategy {
            Some(strategy) => strategy.get_current_strategy(0),
            None => [1.0 / (num_available_actions as f64); DEFAULT_ACTION_COUNT],
        };
        (strategy, serialised_abstraction)
    }

    fn get_traverser_strategy_cache(&self, deal_index: usize, round: usize, num_available_actions: usize, previous_game_abstraction_ser: &mut GameAbstractionSerialised, previous_strategy: [f64; DEFAULT_ACTION_COUNT]) -> [f64; DEFAULT_ACTION_COUNT] {
        let game_abstraction = &self.game_tree_paths[deal_index].abstraction;
        let identical = game_abstraction.replace_round_abstraction(
            previous_game_abstraction_ser,
            round,
            &Player::Traverser, 
        );

        if identical {
            return previous_strategy;
        }

        let strategy = self.traverser_strategy
                .get_strategy(&previous_game_abstraction_ser);

        match strategy {
            Some(strategy) => strategy.get_current_strategy(0),
            None => [1.0 / (num_available_actions as f64); DEFAULT_ACTION_COUNT],
        }
    }
}

fn convert_actions_to_game_state(actions: &[Action], sb_player: Player) -> GameStateHelper {
    let game_state_from_actions = actions_to_state(actions, sb_player);
    GameStateHelper {
        game_abstraction: GameAbstraction::default(),
        traverser_pot: Cell::new(game_state_from_actions.traverser_pot),
        opponent_pot: Cell::new(game_state_from_actions.opponent_pot),
        cards: game_state_from_actions.partial_deal,
        cards_dealt: Cell::new(game_state_from_actions.cards_dealt),
        current_player: Cell::new(game_state_from_actions.current_player),
        small_blind_player: game_state_from_actions.small_blind_player,
        big_blind_player: game_state_from_actions.big_blind_player,
        bets_this_round: Cell::new(game_state_from_actions.bets_this_round),
        winner: None,
        checks_this_round: Cell::new(game_state_from_actions.checks_this_round),
    }
}

fn actions_to_state(actions: &[Action], small_blind_player: Player) -> GameStateFromActions {
    let mut partial_deal = [Card::default(); 9];

    partial_deal[1] = Card::new(Suit::Spades, Rank::Ace); // Small hack because we expect the hole cards to be sorted
    partial_deal[3] = Card::new(Suit::Clubs, Rank::Ace);
    
    let mut deal_index = 4;
    let mut cards_dealt = 0;

    let mut traverser_pot = if small_blind_player == Player::Traverser { 1 } else { 2 };
    let mut opponent_pot = if small_blind_player == Player::Opponent { 1 } else { 2 };

    let mut checks_this_round = 0;
    let mut bets_this_round = 0;

    let mut current_player = small_blind_player;

    for action in actions {
        match action {
            Action::Deal(card) => {
                partial_deal[deal_index] = *card;
                current_player = small_blind_player.get_opposite();
                cards_dealt += 1;
                deal_index += 1;
                bets_this_round = 0;
                checks_this_round = 0;
            },
            Action::Bet => {
                bets_this_round += 1;
                let multiplier = if cards_dealt < 5 { 1 } else { 2 };
                if current_player == Player::Traverser {
                    traverser_pot = opponent_pot + BIG_BLIND * multiplier;
                } else {
                    opponent_pot = traverser_pot + BIG_BLIND * multiplier;
                }
                current_player = current_player.get_opposite();
            },
            Action::Call => {
                if small_blind_player == Player::Traverser {
                    traverser_pot = opponent_pot;
                } else {
                    opponent_pot = traverser_pot;
                }
                current_player = current_player.get_opposite();
            },
            Action::CheckFold => {
                if bets_this_round == 0 && traverser_pot == opponent_pot {
                    checks_this_round += 1;
                } else {
                    break;
                }
                current_player = current_player.get_opposite();
            }
            _ => {}
        }
    };
    // println!("Current player: {:?}", current_player);
    // println!("Small blind player: {}", small_blind_player);
    GameStateFromActions {
        partial_deal,
        traverser_pot,
        opponent_pot,
        cards_dealt,
        small_blind_player,
        big_blind_player: small_blind_player.get_opposite(),
        bets_this_round,
        checks_this_round,
        current_player,
    }
}


/// Calculates combinations for a single hand
fn calculate_combinations(hand: &[Card]) -> usize {
    match hand.len() {
        1 => 6,  // Pocket pair: e.g., AA has 6 combinations
        2 => {
            if hand[0].rank == hand[1].rank {
                6  // Another pocket pair (same rank)
            } else if hand[0].suit == hand[1].suit {
                4  // Suited hands (e.g., AKs)
            } else {
                12 // Offsuit hands (e.g., AKo)
            }
        }
        _ => 0, // Invalid hand size
    }
}

// Returns a list of action_histories where the opponent has just acted so we can calculate any gifts
fn create_action_histories_which_can_gift_us(actions: &[Action], looking_for_sb: bool) -> Vec<Vec<Action>> {
    let mut action_states = vec![];
    let mut current_state = vec![];
    let mut is_sb = true;
    for action in actions {
        current_state.push(action.clone());

        // If the opponent has just acted, add the current state to the list of states we will track
        if is_sb == looking_for_sb {
            action_states.push(current_state.clone());
        }
        match action {
            Action::Deal(_) => {
                is_sb = false;
            },
            _ => {
                is_sb = !is_sb;
            }
        }
    }
    action_states
}