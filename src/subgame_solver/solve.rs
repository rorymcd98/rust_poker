use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::sync::Arc;
use std::vec;

use dashmap::DashMap;
use itertools::Itertools;
use rand::distributions::weighted;
use rust_poker::config::{BIG_BLIND, BLUEPRINT_FOLDER};

use crate::evaluate::evaluate_hand::HandEvaluator;
use crate::models::card::{all_pocket_pairs, all_rank_combos, NineCardDeal, Rank};
use crate::models::{Card, Player, Suit};
use crate::traversal::action_history::action::{self, Action, DEFAULT_ACTION_COUNT};
use crate::traversal::action_history::action_history::ActionHistory;
use crate::traversal::action_history::game_abstraction::{self, convert_deal_into_abstraction, to_string_game_abstraction, GameAbstraction, GameAbstractionSerialised};
use crate::traversal::game_state::game_state_helper::{GameStateHelper, EVALUATOR};
use crate::traversal::game_state::terminal_state::TerminalState;
use crate::traversal::main_train::{get_all_combos_by_blind, get_unique_cards};
use crate::traversal::strategy;
use crate::traversal::strategy::play_strategy::PlayStrategy;
use crate::traversal::strategy::strategy_branch::{StrategyBranch, StrategyHubKey};
use crate::traversal::strategy::strategy_hub::{deserialise_strategy_hub, StrategyHub};
use crate::traversal::strategy::strategy_trait::Strategy;

/// When calculating the gifts, we must TODO finish this comment 
struct CalculateBestValueNode {
    pub weighted_value_sum: f32,
    pub total_reach: f32,
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
        Action::Bet,  Action::Bet, Action::Bet, Action::Call,
        Action::Deal(Card::new(Suit::Clubs, Rank::Two)), Action::Deal(Card::new(Suit::Spades, Rank::Three)), Action::Deal(Card::new(Suit::Clubs, Rank::Four)),
        Action::CheckFold, //Action::Bet, Action::Call, 
        //Action::Deal(Card::new(Suit::Diamonds, Rank::Five)),
        //Action::CheckFold,// Action::Bet, Action::Call, 
        //Action::Deal(Card::new(Suit::Clubs, Rank::Six)),
        //Action::Bet, //Action::Bet, Action::Bet//, Action::Bet, Action::Call
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

    let mut all_game_abstractions = Vec::with_capacity(potential_card_histories.len());
    let mut all_evaluations = Vec::with_capacity(potential_card_histories.len());
    let mut strategy_hub_keys = Vec::with_capacity(potential_card_histories.len());

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
        strategy_hub_keys.push(key);
    }

    // println!("{}", game_state.to_string());
    //  return;

    let mut cbv_solver = CbvSolver{
        traverser_strategy: traverser_strategy_branch,
        opponent_strategy_map,
        game_state,
        evaluations: all_evaluations,
        game_abstractions: all_game_abstractions,
        strategy_hub_keys,

        trav_seen: Cell::new(0),
        opp_seen: Cell::new(0),
        trav_not_seen: Cell::new(0),
        opp_not_seen: Cell::new(0),
    };

    let utility = cbv_solver.calculate_cbv(&action_history);
    println!("Utility: {}", utility);
}

type CbvReturn = Vec<f32>; // Utilitiy for each deal

struct CbvSolver<'a> {
    traverser_strategy: StrategyBranch<PlayStrategy>,
    opponent_strategy_map: DashMap<StrategyHubKey, StrategyBranch<PlayStrategy>>,
    game_state: &'a mut GameStateHelper,
    evaluations: Vec<Option<Player>>,
    game_abstractions: Vec<GameAbstraction>,
    strategy_hub_keys: Vec<StrategyHubKey>,

    trav_seen: Cell<u32>,
    opp_seen: Cell<u32>,
    trav_not_seen: Cell<u32>,
    opp_not_seen: Cell<u32>,
}

impl<'a> CbvSolver<'a> {
    pub fn calculate_cbv(&mut self, action_history: &Vec<Action>) -> f32 { // TODO - move Vec<Action> to the struct
        let initial_reaches = self.calculate_initial_reaches(action_history);
        // println!("Initial reaches: {:?}", initial_reaches);
        // return 0.0;
        let traverser_utility = self.traverse_action(&initial_reaches); // need to weight by reaches
        let res = traverser_utility.iter().sum::<f32>() / initial_reaches.iter().sum::<f32>();
        println!("Utility: {}", res);
        println!("Trav seen: {}, Trav not seen: {}", self.trav_seen.get(), self.trav_not_seen.get());
        println!("Opp seen: {}, Opp not seen: {}", self.opp_seen.get(), self.opp_not_seen.get());
        res
    }

    fn calculate_initial_reaches(&self, action_history: &Vec<Action>) -> Vec<f32> {
        let mut reaches = vec![1.0; self.game_abstractions.len()];
        let game_state = GameStateHelper::new(self.game_state.cards, self.game_state.small_blind_player);
        game_state.cards_dealt.set(0);

        for action in action_history {
            let round = (game_state.cards_dealt.get()).saturating_sub(2) as usize;
            let current_player_pot = game_state.get_current_player_pot();
            let bets_this_round = game_state.bets_this_round.get();
            let num_available_actions = game_state.get_num_available_actions();

            println!("player {}, action {}, round {}, current_player_pot {}, bets_this_round {}, num_available_actions {}", game_state.get_current_player(), action,  round, current_player_pot, bets_this_round, num_available_actions);

            match action {
                Action::Deal(_) => {
                    game_state.deal();
                },
                Action::Bet => {
                    match game_state.current_player.get() {
                        Player::Traverser => {
                            // The traverser strategy is the same up until we begin branching - hence deal_index 0 (we could use any deal index here)
                            let strategy = self.get_traverser_strategy(0, round, current_player_pot, bets_this_round, num_available_actions);
                            for deal_index in 0..self.game_abstractions.len() {
                                let action_probability = strategy[2.max(num_available_actions-1)];
                                reaches[deal_index] *= action_probability;
                            }
                        },
                        Player::Opponent => {
                            for deal_index in 0..self.game_abstractions.len() {
                                let strategy = self.get_opponent_strategy(deal_index, round, current_player_pot, bets_this_round, num_available_actions);
                                let action_probability = strategy[2.max(num_available_actions-1)];
                                reaches[deal_index] *= action_probability;
                            }
                        }
                    }
                    game_state.bet();
                    game_state.switch_current_player();
                },
                Action::Call => {
                    match game_state.current_player.get() {
                        Player::Traverser => {
                            // ditto
                            let strategy = self.get_traverser_strategy(0, round, current_player_pot, bets_this_round, num_available_actions);
                            for deal_index in 0..self.game_abstractions.len() {
                                let action_probability = strategy[1];
                                reaches[deal_index] *= action_probability;
                            }
                        },
                        Player::Opponent => {
                            for deal_index in 0..self.game_abstractions.len() {
                                let strategy = self.get_opponent_strategy(deal_index, round, current_player_pot, bets_this_round, num_available_actions);
                                //println!("Strategy: {:?}", strategy);
                                let action_probability = strategy[1];
                                reaches[deal_index] *= action_probability;
                            }
                        }
                    }
                    game_state.call();
                    game_state.switch_current_player();
                },
                Action::CheckFold => {
                    match game_state.current_player.get() {
                        Player::Traverser => {
                            // ditto
                            let strategy = self.get_traverser_strategy(0, round, current_player_pot, bets_this_round, num_available_actions);
                            for deal_index in 0..self.game_abstractions.len() {
                                let action_probability = strategy[0];
                                reaches[deal_index] *= action_probability;
                            }
                        },
                        Player::Opponent => {
                            for deal_index in 0..self.game_abstractions.len() {
                                let strategy = self.get_opponent_strategy(deal_index, round, current_player_pot, bets_this_round, num_available_actions);
                                //println!("Strategy: {:?}", strategy);
                                let action_probability = strategy[0];
                                reaches[deal_index] *= action_probability;
                            }
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

    pub fn evaluate_showdown(&self, deal_index: usize, traverser_pot: f32, opponent_pot: f32) -> f32 {
        let winner = self.evaluations[deal_index];
        match winner {
            Some(Player::Traverser) => opponent_pot,
            Some(Player::Opponent) => -traverser_pot,
            None => 0.0,
        }
    }
    
    pub fn evaluate_fold(&self, current_player: &Player, traverser_pot: f32, opponent_pot: f32) -> f32 {
        match current_player {
            Player::Traverser => opponent_pot,
            Player::Opponent => -traverser_pot,
        }
    }

    // If Traverser, next reaches in the action probs
    // If Opponent, next reaches is dependent on the strategy for each hc
    fn traverse_action(&mut self, reaches: &Vec<f32>) -> CbvReturn {
        let num_available_actions = self.game_state.get_num_available_actions();

        // Cache these
        let traverser_pot = self.game_state.traverser_pot.get() as f32;
        let opponent_pot = self.game_state.opponent_pot.get() as f32;
        let current_player = self.game_state.current_player.get();

        let mut cbv_return = Vec::with_capacity(self.game_abstractions.len());
        
        match self.game_state.check_round_terminal() {
            TerminalState::Showdown => {
                for deal_index in 0..self.game_abstractions.len() {
                    let showdown_utility = self.evaluate_showdown(deal_index, traverser_pot, opponent_pot);
                    cbv_return.push(showdown_utility * reaches[deal_index]);
                };
                return cbv_return;
            },
            TerminalState::Fold => {
                // Don't need to iterate over the game abstractions here because:
                // (n*A + n*B + n*C) / (A + B + C) = n
                // Where n is the (constant for this node) fold utility for this infoset and A, B, C are the reaches
                cbv_return = vec![self.evaluate_fold(&current_player, traverser_pot, opponent_pot); self.game_abstractions.len()];
                return cbv_return;
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

    fn action_value(&mut self, reaches: &Vec<f32>, num_available_actions: usize, current_player: Player, round: usize, bets_this_round: u8, pot_before_action: u8, bets_before_action: u8, previous_player: Player, checks_before: u8) -> CbvReturn {
        let mut reaches_for_actions = vec![vec![0.0; self.game_abstractions.len()]; num_available_actions];
        let mut action_probabilities = vec![[0f32; DEFAULT_ACTION_COUNT]; self.game_abstractions.len()];
        
        // TODO - Not sure if its right to calculate the reaches the same for opponent and traverser here
        for deal_index in 0..self.game_abstractions.len() {
            let strategy = match current_player {
                Player::Traverser => self.get_traverser_strategy(deal_index, round, pot_before_action, bets_this_round, num_available_actions),
                Player::Opponent => self.get_opponent_strategy(deal_index, round, pot_before_action, bets_this_round, num_available_actions),
            };
            for action in 0..num_available_actions {
                // store the reaches for each action
                reaches_for_actions[action][deal_index] = reaches[deal_index] * strategy[action];
            }
            action_probabilities[deal_index] = strategy;
        }
        
        let mut action_abstraction_utilities = vec![Vec::with_capacity(self.game_abstractions.len()); num_available_actions]; // TODO - Should use option
        for action in 0..num_available_actions {
            // TODO - We could prune here if next_reaches sum is close to 0
            let next_reaches = &reaches_for_actions[action];
            action_abstraction_utilities[action] = self.traverse_chosen_action(next_reaches, action, previous_player, pot_before_action, bets_before_action, checks_before);
        }

        if self.game_state.current_player.get() == Player::Traverser {
            let mut max_utility = f32::NEG_INFINITY;
            let mut best_action_utilities = vec![0.0; self.game_abstractions.len()];
            let mut best_action_count = 0;

            for action in 0..num_available_actions {
                let mut reach_utility = 0.0;
                let mut total_reach = 0.0;
                let abstraction_utilities = &action_abstraction_utilities[action];
                for deal_index in 0..self.game_abstractions.len() {
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
                    for deal_index in 0..self.game_abstractions.len() {
                        best_action_utilities[deal_index] += abstraction_utilities[deal_index];
                    }
                    best_action_count += 1;
                }
            }

            if best_action_count > 1 {
                for deal_index in 0..self.game_abstractions.len() {
                    best_action_utilities[deal_index] /= best_action_count as f32;
                }
            }
            return best_action_utilities;
        } else {
            let mut action_utilities = vec![0.0; self.game_abstractions.len()];
            for action in 0..num_available_actions {
                let abstraction_utilities = &action_abstraction_utilities[action];
                for deal_index in 0..self.game_abstractions.len() {
                    action_utilities[deal_index] += abstraction_utilities[deal_index] * action_probabilities[deal_index][action];
                }
            }
            return action_utilities;
        };
    }

    fn traverse_chosen_action(&mut self, reaches: &Vec<f32>, action: usize, acting_player: Player, current_pot: u8, current_bets: u8, current_checks: u8) -> CbvReturn {
        self.game_state.switch_current_player();
        match action {
            0 => self.game_state.checkfold(),
            1 => self.game_state.call_or_bet(),
            2 => self.game_state.bet(),
            _ => {}
        };
        let utility = self.traverse_action(reaches);
        self.game_state.undo(
            acting_player,
            current_pot,
            current_bets,
            current_checks,
        );
        utility
    }

    fn traverse_flop(&mut self, reaches: &Vec<f32>) -> CbvReturn {
        let previous_player = self.game_state.current_player.get();
        let previous_bets = self.game_state.bets_this_round.get();
        let check_before = self.game_state.checks_this_round.get();

        self.game_state.deal_flop();
        let utility = self.traverse_action(reaches);
        self.game_state
            .undeal_flop(previous_bets, previous_player, check_before);
        utility
    }

    fn traverse_deal(&mut self, reaches: &Vec<f32>) -> CbvReturn {
        let previous_player = self.game_state.current_player.get();
        let previous_bets = self.game_state.bets_this_round.get();
        let checks_before = self.game_state.checks_this_round.get();

        self.game_state.deal();
        let utility = self.traverse_action(reaches);

        self.game_state
            .undeal(previous_bets, previous_player, checks_before);
        utility
    }

    fn get_opponent_strategy(&self, deal_index: usize, round: usize, current_player_pot: u8, bets_this_round: u8, num_available_actions: usize) -> [f32; DEFAULT_ACTION_COUNT] {
        let game_abstraction = &self.game_abstractions[deal_index];
        // TODO - massively inefficient to regenerate this every time
        let serialised_abstraction = game_abstraction.get_abstraction(
            round,
            current_player_pot,
            bets_this_round,
            &Player::Opponent, 
        );

        let strategy_hub_key = &self.strategy_hub_keys[deal_index];
        
        let strategy= self.opponent_strategy_map
        .get(&strategy_hub_key)
        .and_then(|strategy_branch| strategy_branch.get_strategy(&serialised_abstraction).cloned());
    
    
        if strategy.is_none() {
            println!("Strategy hub key: {:?}, round {}, bets {}, pot {}", strategy_hub_key, round, bets_this_round, current_player_pot);
            self.opp_not_seen.set(self.opp_not_seen.get() + 1);
        } else {
            self.opp_seen.set(self.opp_seen.get() + 1);
        }

        match strategy {
            Some(strategy) => strategy.get_current_strategy(0),
            None => [1.0 / (num_available_actions as f32); DEFAULT_ACTION_COUNT],
        }
    }

    fn get_traverser_strategy(&self, deal_index: usize, round: usize, current_player_pot: u8, bets_this_round: u8, num_available_actions: usize) -> [f32; DEFAULT_ACTION_COUNT] {
        let game_abstraction = &self.game_abstractions[deal_index];
        let serialised_abstraction = game_abstraction.get_abstraction(
            round,
            current_player_pot,
            bets_this_round,
            &Player::Traverser, 
        );

        let strategy = self.traverser_strategy
                .get_strategy(&serialised_abstraction);

        if strategy.is_none() {
            self.trav_not_seen.set(self.trav_not_seen.get() + 1);
        } else {
            self.trav_seen.set(self.trav_seen.get() + 1);
        }
        match strategy {
            Some(strategy) => strategy.get_current_strategy(0),
            None => [1.0 / (num_available_actions as f32); DEFAULT_ACTION_COUNT],
        }
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