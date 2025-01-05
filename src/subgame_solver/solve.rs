use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::sync::Arc;

use dashmap::DashMap;
use rust_poker::config::{BIG_BLIND, BLUEPRINT_FOLDER};

use crate::evaluate::evaluate_hand::HandEvaluator;
use crate::models::card::{all_pocket_pairs, all_rank_combos, NineCardDeal, Rank};
use crate::models::{Card, Player, Suit};
use crate::traversal::action_history::action::Action;
use crate::traversal::action_history::game_abstraction::{convert_deal_into_abstraction, to_string_game_abstraction, GameAbstraction};
use crate::traversal::game_state::game_state_helper::{GameStateHelper, EVALUATOR};
use crate::traversal::game_state::terminal_state::TerminalState;
use crate::traversal::main_train::{get_all_combos_by_blind, get_unique_cards};
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
    let mut strategy_hub = StrategyHub::from_map(deserialise_strategy_hub::<PlayStrategy>(BLUEPRINT_FOLDER).unwrap()).unwrap();
    let player_hand = 
    [Card::new(Suit::Spades, Rank::King), Card::new(Suit::Clubs, Rank::King)];


    let sb_key = StrategyHubKey {
        low_rank: player_hand[0].rank,
        high_rank: player_hand[1].rank,
        is_suited: player_hand[0].suit == player_hand[1].suit,
        is_sb: true,
    };

    let mut sb_strategy_branch = strategy_hub.sb_in_store.remove(&sb_key).unwrap().1; // TODO - provide a method for this? 

    const PLAY_ITERATIONS: usize = 10000;
    let opponent_bb_elements = get_all_combos_by_blind(false);
    
    // Look for the gifts give on the Bet node
    let action_history = &vec![
        Action::Bet,  
        // Action::Deal(Card::new(Suit::Clubs, Rank::Two)), Action::Deal(Card::new(Suit::Spades, Rank::Three)), Action::Deal(Card::new(Suit::Clubs, Rank::Four)),
        // Action::CheckFold, Action::Bet, Action::Call, 
        // Action::Deal(Card::new(Suit::Clubs, Rank::Five)),
        // Action::CheckFold, Action::Bet, Action::Call, 
        // Action::Deal(Card::new(Suit::Clubs, Rank::Six)),
        // Action::CheckFold, Action::Bet, Action::Bet,
    ];
    let nodes_of_interest = create_action_histories_which_can_gift_us(&action_history, false);
    
    let gift_map: Arc<DashMap<Vec<Action>, CalculateBestValueNode>> = Arc::new(DashMap::new());
    let cbv_map = Arc::<DashMap<StrategyHubKey, f32>>::new(DashMap::new());
    // for node in nodes_of_interest {
    //     gift_map.insert(node, CalculateBestValueNode { weighted_value_sum: 0.0, total_reach: 0.0 });
    // }

    // Create a map for the subgame
    
    let margin_map: Arc<DashMap<Vec<Action>, f32>> = Arc::new(DashMap::new());

    // let mut bb_strategy_branch = strategy_hub.bb_in_store.remove(&opponent_bb_elements[0]).unwrap().1;
    // bb_strategy_branch.print_stats();
    // bb_strategy_branch.map.iter().for_each(|(key, value)| {
    //     if key.starts_with(&[0,2,1]) {
    //         println!("Key: {:?}, Value: {:?}", key, value.get_current_strategy(1));
    //     } 
    // });
    // return; 

    for bb_key in opponent_bb_elements {
        let mut bb_strategy_branch = strategy_hub.bb_in_store.remove(&bb_key).unwrap().1;
        
        let mut total_utility = 0.0;

        let sb_player = Player::Traverser;
        let game_state = &mut convert_actions_to_game_state(&action_history, Player::Opponent);
        let cards_dealt = game_state.cards_dealt.get();
        println!("Existing cards: {:?}", cards_dealt);
        let current_board_cards = game_state.cards[4..(4 + cards_dealt as usize)].to_vec();
        
        let cards = get_unique_cards(&sb_key, &bb_key); // TODO - Extend this so it actually avoids unique cards
        let mut existing_cards = vec![cards[0], cards[1], cards[2], cards[3]];
        // EVALUATOR
        existing_cards.extend(current_board_cards.iter());
        let new_cards = Card::get_n_more_cards(&existing_cards, 5 - cards_dealt as usize);
        existing_cards.extend(new_cards);
        let deal = &existing_cards.try_into().expect("Expected a slice of length 9");
        let game_abstraction = convert_deal_into_abstraction(*deal, sb_player);
        game_state.game_abstraction = game_abstraction;
        
        println!("Current player {}, small blind player {}", game_state.current_player.get(), game_state.small_blind_player);
        if (game_state.current_player.get() != Player::Traverser) {
            panic!("Expected the traverser to be the current player");
        }
        if (game_state.current_player.get() == game_state.small_blind_player) {
            panic!("Expected the bblind to be the current player");
        }

        let abstraction = game_state.serialise_history_with_current_player();
        println!("Abstraction: {:?}", abstraction);
        let hol1 = bb_strategy_branch.strategy_hub_key.low_rank;
        let hole2 = bb_strategy_branch.strategy_hub_key.high_rank;
        let suited = bb_strategy_branch.strategy_hub_key.is_suited;
        let is_sb = bb_strategy_branch.strategy_hub_key.is_sb;
        println!("Abstraction: {:?}", to_string_game_abstraction(hol1, hole2, suited, is_sb, &abstraction));

        let strategy = bb_strategy_branch.get_strategy(&abstraction);
        if strategy.is_none() {
            panic!("Expected a strategy");
        } else {
            println!("Foudn STARETGHASD")
        }



        
        // for _ in 0..PLAY_ITERATIONS {
        //     let cards = get_unique_cards(&sb_key, &bb_key); // TODO - Extend this so it actually avoids unique cards
        //     let mut existing_cards = vec![cards[0], cards[1], cards[2], cards[3]];
        //     // EVALUATOR
        //     existing_cards.extend(current_board_cards.iter());
        //     let new_cards = Card::get_n_more_cards(&existing_cards, 5 - cards_dealt as usize);
        //     existing_cards.extend(new_cards);

        //     game_state.winner = EVALUATOR.evaluate_deal(&existing_cards.try_into().expect("Expected a slice of length 9"));

        //     let mut branch_traverser = SolvingBranchTraverser::new(
        //         &mut sb_strategy_branch,
        //         &mut bb_strategy_branch,
        //         game_state,
        //         Arc::clone(&gift_map),
        //     );
        //     let utility = branch_traverser.begin_traversal();
        //     total_utility += utility;
        // }

        // println!("Average utility for {} vs {}: {}", sb_key, bb_key,  total_utility / (PLAY_ITERATIONS as f32));
        // bb_strategy_branch.print_stats();
        // strategy_hub.bb_in_store.insert(bb_key, bb_strategy_branch);
    }

    sb_strategy_branch.print_stats();

}

struct SolvingBranchTraverser<'a> {
    game_state: &'a mut GameStateHelper,
    sb_strategy_branch: &'a mut StrategyBranch<PlayStrategy>,
    bb_strategy_branch: &'a mut StrategyBranch<PlayStrategy>,
    current_reach: Cell<f32>,
    cbr_map: Arc<DashMap<Vec<Action>, CalculateBestValueNode>>,
    action_histories: RefCell<Vec<Action>>,
}

impl<'a> SolvingBranchTraverser<'a> {
    pub fn new(
        sb_strategy_branch: &'a mut StrategyBranch<PlayStrategy>,
        bb_strategy_branch: &'a mut StrategyBranch<PlayStrategy>,
        game_state: &'a mut GameStateHelper,
        cbr_map: Arc<DashMap<Vec<Action>, CalculateBestValueNode>>,
    ) -> SolvingBranchTraverser<'a> {
        SolvingBranchTraverser {
            sb_strategy_branch,
            bb_strategy_branch,
            game_state,
            current_reach: Cell::new(1.0), // TODO - Consider a better type for this
            cbr_map,
            action_histories: RefCell::new(vec![]),
        }
    }

    pub fn begin_traversal(&mut self) -> f32 {
        self.traverse_action()
    }

    fn traverse_action(&mut self) -> f32 {
        match self.game_state.check_round_terminal() {
            TerminalState::Showdown => return self.game_state.evaluate_showdown(),
            TerminalState::Fold => return self.game_state.evaluate_fold(),
            TerminalState::RoundOver => {
                if self.game_state.is_preflop() {
                    return self.traverse_flop();
                }
                return self.traverse_deal();
            }
            TerminalState::None => (),
        };

        let num_available_actions = self.game_state.get_num_available_actions();

        let pot_before_action = self.game_state.get_current_player_pot();
        let bets_before_action = self.game_state.bets_this_round.get();
        let previous_player = self.game_state.current_player.get();
        let checks_before = self.game_state.checks_this_round.get();

        let strategy = self.get_strategy();
        let current_strategy = strategy.get_current_strategy(0);
        let reach_now = self.current_reach.get();

        let mut max_cbv: f32 = 0.0;
        let mut action_cbvs = vec![0.0; num_available_actions];
        for action in 0..num_available_actions {
            if current_strategy[action] < 0.01 {
                continue;
            }
            // TODO - move this into the traverse_chosen_action method
            self.current_reach.set(reach_now * current_strategy[action]);
            action_cbvs[action] = reach_now * self.traverse_chosen_action(action, previous_player, pot_before_action, bets_before_action, checks_before);
            max_cbv = max_cbv.max(action_cbvs[action]);
        }
        self.current_reach.set(reach_now);
        max_cbv
    }

    fn traverse_chosen_action(&mut self, action: usize, acting_player: Player, current_pot: u8, current_bets: u8, current_checks: u8) -> f32 {
        match action {
            0 => {
                self.game_state.checkfold();
                self.action_histories.borrow_mut().push(Action::CheckFold);
            },
            1 => {
                self.game_state.call_or_bet();
                self.action_histories.borrow_mut().push(Action::Call);
            },
            2 => {
                self.game_state.bet();
                self.action_histories.borrow_mut().push(Action::Bet);
            },
            _ => panic!("Invalid action"),
        };
        self.game_state.switch_current_player();
        let utility = self.traverse_action();
        self.game_state.undo(
            acting_player,
            current_pot,
            current_bets,
            current_checks,
        );
        self.action_histories.borrow_mut().pop();
        utility
    }

    fn traverse_flop(&mut self) -> f32 {
        let previous_player = self.game_state.current_player.get();
        let previous_bets = self.game_state.bets_this_round.get();
        let check_before = self.game_state.checks_this_round.get();
        let flop_cards = self.game_state.cards[4..7].to_vec();
        for card in flop_cards {
            self.action_histories.borrow_mut().push(Action::Deal(card));
        }
        self.game_state.deal_flop();
        let utility = self.traverse_action();
        for _ in 0..3 {
            self.action_histories.borrow_mut().pop();
        }
        self.game_state
            .undeal_flop(previous_bets, previous_player, check_before);
        utility
    }

    fn traverse_deal(&mut self) -> f32 {
        let previous_player = self.game_state.current_player.get();
        let previous_bets = self.game_state.bets_this_round.get();
        let checks_before = self.game_state.checks_this_round.get();
        let cards_dealt = self.game_state.cards_dealt.get();
        let card = self.game_state.cards[4 + cards_dealt as usize];
        self.action_histories.borrow_mut().push(Action::Deal(card));
        self.game_state.deal();
        let utility = self.traverse_action();
        self.action_histories.borrow_mut().pop();
        self.game_state
            .undeal(previous_bets, previous_player, checks_before);
        utility
    }

    fn get_strategy(&mut self) -> PlayStrategy {
        let strategy_branch = if self.game_state.current_player.get() == self.game_state.small_blind_player {
            &mut self.sb_strategy_branch
        } else {
            &mut self.bb_strategy_branch
        };
        strategy_branch.get_strategy_or_default(&self.game_state.serialise_history_with_current_player(), self.game_state.get_num_available_actions())
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