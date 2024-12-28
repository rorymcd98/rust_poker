use itertools::Itertools;
use lazy_static::lazy_static;

use crate::evaluate::evaluate_hand::EvaluateHand;
use crate::evaluate::evaluate_hand::HandEvaluator;
use crate::models::card::NineCardDeal;
use crate::models::card::Rank;
use crate::models::Card;
use crate::models::Suit;
use crate::models::Player;
use crate::traversal::action::Action;
use super::strategy::strategy_branch::InfoNode;
use super::strategy::{strategy_map::StrategyMap, strategy_branch::StrategyBranch};
use super::action_history::ActionHistory;
use std::cell::{Cell, RefCell};
use std::thread;
use std::sync::Arc;
use std::thread::JoinHandle;

lazy_static! {
    static ref EVALUATOR: EvaluateHand = EvaluateHand::new();
}

enum TerminalState {
    Showdown,
    Fold,
    RoundOver,
    None,
}

struct GameStateHelper {
    action_history: RefCell<ActionHistory>,
    traverser_pot: Cell<u8>,
    opponent_pot: Cell<u8>,
    cards: [Card; 9],
    cards_dealt: Cell<u8>,
    current_player: Cell<Player>,
    small_blind_player: Player,
    bets_this_round: Cell<u8>,
    winner: Option<Player>,
}



impl GameStateHelper {
    pub fn new(action_history: ActionHistory, nine_card_deal: NineCardDeal, small_blind_player: Player) -> GameStateHelper {
        GameStateHelper {
            action_history: RefCell::new(action_history),
            traverser_pot: Cell::new(if small_blind_player == Player::Traverser { 1 } else { 2 }),
            opponent_pot: Cell::new(if small_blind_player == Player::Opponent { 1 } else { 2 }),
            cards: nine_card_deal,
            cards_dealt: Cell::new(0),
            current_player: Cell::new(small_blind_player),
            small_blind_player,
            bets_this_round: Cell::new(0),
            winner: EVALUATOR.evaluate_deal(nine_card_deal),
        }
    }

    pub fn get_current_player(&self) -> Player {
        self.current_player.get()
    }

    pub fn switch_current_player(&self) {
        self.current_player.set(self.current_player.get().get_opposite());
    }

    pub fn set_current_player_to_small_blind(&self) {
        self.current_player.set(self.small_blind_player.clone());
    }

    pub fn set_current_player_to_big_blind(&self) {
        self.current_player.set(self.small_blind_player.get_opposite());
    }

    pub fn get_flop(&self) -> [Card; 3] {
        [self.cards[4], self.cards[5], self.cards[6]]
    }

    pub fn is_preflop(&self) -> bool {
        self.cards_dealt.get() == 0
    }

    pub fn is_river(&self) -> bool {
        self.cards_dealt.get() == 5
    }

    pub fn get_num_available_actions(&self) -> usize {
        match self.bets_this_round.get() {
            0 => 2,
            4 => 2,
            _ => 3,
        }
    }

    pub fn serialise_history_with_current_player(&self) -> InfoNode {
        let current_player_hole_cards = match self.current_player.get() {
            Player::Traverser => [self.cards[0], self.cards[1]],
            Player::Opponent => [self.cards[2], self.cards[3]],
        };
        // TODO - test the performance of keeping the Deal(cards) in the struct ?
        self.action_history.borrow_mut().set_hole_cards(Action::Deal(current_player_hole_cards[0]), Action::Deal(current_player_hole_cards[1]));
        self.action_history.borrow().serialise()
    }

    pub fn check_round_terminal(&self) -> TerminalState {
        let action_history = &self.action_history.borrow().history;
        let last_two_actions = &action_history[action_history.len() - 2..];
        let terminal_state = match last_two_actions {
            [Action::CheckFold, Action::CheckFold] => TerminalState::Showdown,
            [Action::Call, Action::CheckFold] => TerminalState::RoundOver,
            [Action::Bet, Action::CheckFold] => TerminalState::Fold,
            [Action::Call, Action::Call] => panic!("Invalid state - should not have two calls in a row {:?}", &self.action_history.borrow().history), 
            [Action::Bet, Action::Call] => TerminalState::Showdown,
            _ => TerminalState::None,
        };

        if self.is_river() {
            match terminal_state {
                TerminalState::None => return TerminalState::None,
                TerminalState::RoundOver => panic!("Invalid state - should not have a round over state on the river {:?}", &self.action_history.borrow().history),
                _ => return terminal_state,
            }
        } else {
            match terminal_state {
                TerminalState::Fold => return TerminalState::Fold,
                TerminalState::None => return TerminalState::None,
                _ => return TerminalState::RoundOver,
            }
        }
    }

    fn evaluate_showdown(&self) -> f64 {
        // validate_history(&self.action_history.borrow().history);
        match self.winner {
            Some(Player::Traverser) => self.opponent_pot.get() as f64,
            Some(Player::Opponent) => -(self.traverser_pot.get() as f64),
            None => 0.0,
        }
    }

    fn evaluate_terminal_fold(&self) -> f64 {
        match self.current_player.get() {
            Player::Traverser => -(self.traverser_pot.get() as f64),
            Player::Opponent => self.opponent_pot.get() as f64,
        }
    }

    pub fn bet(&self) {
        self.bets_this_round.set(self.bets_this_round.get() + 1);
        self.action_history.borrow_mut().history.push(Action::Bet);
        match self.current_player.get() {
            Player::Traverser => {
                self.traverser_pot.set(self.traverser_pot.get() + 2);
            },
            Player::Opponent => {
                self.opponent_pot.set(self.opponent_pot.get() + 2);
            },
        }
    }

    pub fn unbet(&self) {
        //println!("Bets this round {}", self.bets_this_round.get());
        self.bets_this_round.set(self.bets_this_round.get() - 1);
        self.action_history.borrow_mut().history.pop();
        match self.current_player.get() {
            Player::Traverser => {
                self.traverser_pot.set(self.traverser_pot.get() - 2);
            },
            Player::Opponent => {
                self.opponent_pot.set(self.opponent_pot.get() - 2);
            },
        };
    }

    pub fn get_current_player_pot(&self) -> u8 {
        match self.current_player.get() {
            Player::Traverser => self.traverser_pot.get(),
            Player::Opponent => self.opponent_pot.get(),
        }
    }

    fn call_or_bet(&self) {
        match self.bets_this_round.get() {
            0 => self.bet(),
            _ => self.call(),
        }
    }

    fn uncall_or_unbet(&self, previous_pot: u8, previous_bets: u8) {
        match previous_bets {
            0 => self.unbet(),
            _ => self.uncall(previous_pot),
        }
    }

    pub fn call(&self) {
        self.action_history.borrow_mut().history.push(Action::Call);
        match self.current_player.get() {
            Player::Traverser => {
                self.traverser_pot.set(self.opponent_pot.get());
            },
            Player::Opponent => {
                self.opponent_pot.set(self.traverser_pot.get());
            },
        };
    }

    pub fn uncall(&self, previous_pot: u8) {
        self.action_history.borrow_mut().history.pop();
        match self.current_player.get() {
            Player::Traverser => {
                self.traverser_pot.set(previous_pot);
            },
            Player::Opponent => {
                self.opponent_pot.set(previous_pot);
            },
        };
    }

    pub fn checkfold(&self) {
        self.action_history.borrow_mut().history.push(Action::CheckFold);
    }

    pub fn uncheckfold(&self) {
        self.action_history.borrow_mut().history.pop();
    }

    // implement deal and undeal
    pub fn deal(&self) {
        let deal = 4 + self.cards_dealt.get() as usize;
        let card = self.cards[deal];
        self.action_history.borrow_mut().history.push(Action::Deal(card));
        self.cards_dealt.set(self.cards_dealt.get() + 1);
        self.bets_this_round.set(0);
        self.set_current_player_to_big_blind();
    }

    pub fn undeal(&self, previous_bets: u8, previous_player: Player) {
        self.action_history.borrow_mut().history.pop();
        self.cards_dealt.set(self.cards_dealt.get() - 1);
        self.bets_this_round.set(previous_bets);
        self.current_player.set(previous_player);
    }

    pub fn deal_flop(&self) {
        let flop = self.get_flop();
        for card in flop {
            self.action_history.borrow_mut().history.push(Action::Deal(card));
        }
        self.cards_dealt.set( 3);
        self.bets_this_round.set(0);
        self.set_current_player_to_big_blind();
    }

    pub fn undeal_flop(&self, previous_bets: u8, previous_player: Player) {
        for _ in 0..3 {
            self.action_history.borrow_mut().history.pop();
        }
        self.cards_dealt.set(0);
        self.bets_this_round.set(previous_bets);
        self.current_player.set(previous_player);
    }
}

const ITERATIONS: usize = 10_000;
const NUM_THREADS: usize = 12;

pub struct TreeTraverser {
    strategy_map: StrategyMap,
    iterations: usize,
}

impl TreeTraverser {
    pub fn new(iterations: usize) -> TreeTraverser {
        TreeTraverser {
            strategy_map: StrategyMap::new(),
            iterations
        }
    }

    fn spawn_thread_work(combo_chunk: Vec<(Card, Card)>) -> JoinHandle<(f64, (Card, Card))> {
        thread::spawn(move ||{
            let mut highest_so_far = 0.0;
            let mut highest_combo = (Card::new(Suit::Spades, Rank::Two), Card::new(Suit::Spades, Rank::Two));

            let players = [Player::Traverser, Player::Opponent];
            for card_combo in combo_chunk {
                for player in &players { // Alternate between who is small blind
                    let mut average_utility = 0.0;
                    let card_combo_print = card_combo.clone();
                    println!("Starting card combo {:?}", card_combo_print);
                    let mut strategy_branch = StrategyBranch::new();
                    let action_history = &mut ActionHistory::new(vec![]); // TODO - optimise this
                    let traverser_cards = [card_combo.0, card_combo.1];
                    action_history.history.push(Action::Deal(traverser_cards[0].clone()));
                    action_history.history.push(Action::Deal(traverser_cards[1].clone()));
                    for iteration in 1..ITERATIONS { // Iter must start at 1
                        assert!(action_history.history.len() == 2);
                        let deal = Card::new_random_9_card_game_with(traverser_cards[0], traverser_cards[1]);
                
                        let game_state = GameStateHelper::new(action_history.clone(), deal, player.clone());
                        let mut branch_traverser = BranchTraverser::new(strategy_branch, game_state, iteration);
                
                        let result= branch_traverser.begin_traversal();
                        strategy_branch = result.1;
                        average_utility += result.0;
                    }
                    average_utility /= ITERATIONS as f64;

                    if average_utility > highest_so_far {
                        highest_so_far = average_utility;
                        highest_combo = card_combo_print;
                    }
                }
            }
            (highest_so_far, highest_combo)
        })
    }

    pub fn begin_tree_traversal(self){
        let mut highest_so_far = 0.0;
        let mut highest_combo = (Card::new(Suit::Spades, Rank::Two), Card::new(Suit::Spades, Rank::Two));
        let card_combos = Card::all_suited_combos(Suit::Spades).collect_vec();
        let chunk_size = card_combos.len() / NUM_THREADS;
        let card_combos = Arc::new(card_combos);
        let card_combos_chunks: Vec<_> = card_combos.chunks(chunk_size).collect();

        let mut handles = vec![];
        for chunk in card_combos_chunks.into_iter() {  
            handles.push(Self::spawn_thread_work(chunk.to_vec()));
        }

        for handle in handles {
            let res = handle.join().unwrap();
            if res.0 > highest_so_far {
                highest_so_far = res.0;
                highest_combo = res.1;
            }
        }

        println!("Highest average utility: {:?} with card combo {:?}", highest_so_far, highest_combo);
        }
}

struct BranchTraverser {
    strategy_branch: RefCell::<StrategyBranch>,
    iteration: usize,
    game_state: GameStateHelper,
}

impl BranchTraverser {
    pub fn new(strategy_branch: StrategyBranch, game_state: GameStateHelper, iteration: usize) -> BranchTraverser {
        BranchTraverser {
            strategy_branch: RefCell::new(strategy_branch),
            iteration,
            game_state,
        }
    }
    
    pub fn begin_traversal(&mut self) -> (f64, StrategyBranch) {        
        self.game_state.set_current_player_to_small_blind();
        let utility = self.traverse_action();
        (utility, self.strategy_branch.take())
    }

    fn traverse_action(&self) -> f64 {
        match self.game_state.check_round_terminal() {
            TerminalState::Showdown => return self.game_state.evaluate_showdown(),
            TerminalState::Fold => return self.game_state.evaluate_terminal_fold(),
            TerminalState::RoundOver => {
                if self.game_state.is_preflop() {
                    return self.traverse_flop();
                }
                return self.traverse_deal();
            }
            TerminalState::None => (),
        };

        let info_node_key = self.game_state.serialise_history_with_current_player();
        let num_available_actions = self.game_state.get_num_available_actions() as usize;
        let strategy = {
            let mut strategy_branch = self.strategy_branch.borrow_mut();
            strategy_branch.get_or_create_strategy(info_node_key.clone(), num_available_actions).clone()
        };

        let pot_before_action = self.game_state.get_current_player_pot();
        let bets_before_action = self.game_state.bets_this_round.get();
        let previous_player = self.game_state.current_player.get();

        if self.game_state.current_player.get().is_opponent() {
            let sampled_action = strategy.sample_strategy();

            match sampled_action {
                0 => self.game_state.checkfold(),
                1 => self.game_state.call_or_bet(),
                2 => self.game_state.bet(),
                _ => panic!("Invalid action"),
            };

            self.game_state.switch_current_player();
            // Propagate up the randomly selected action multiplied by the chance of selecting it
            let utility = self.traverse_action();

            match sampled_action {
                0 => self.game_state.uncheckfold(),
                1 => self.game_state.uncall_or_unbet(pot_before_action, bets_before_action),
                2 => self.game_state.unbet(),
                _ => panic!("Invalid action"),
            };

            self.game_state.current_player.set(previous_player); // not neccessary but for clarity

            utility
        } else {
            let mut utility = 0.0;
            let mut utilities = vec![0.0; num_available_actions];

            for action in 0..num_available_actions {
                let pot_before_action = self.game_state.get_current_player_pot();
                match action {
                    0 => self.game_state.checkfold(),
                    1 => self.game_state.call_or_bet(), // TODO - change this call-or-bet logic
                    2 => self.game_state.bet(),
                    _ => panic!("Invalid action"),
                };
                self.game_state.switch_current_player();
                utilities[action] = self.traverse_action() * strategy.current_strategy[action];

                match action {
                    0 => self.game_state.uncheckfold(),
                    1 => self.game_state.uncall_or_unbet(pot_before_action, bets_before_action),
                    2 => self.game_state.unbet(),
                    _ => panic!("Invalid action"),
                };
    
                self.game_state.current_player.set(previous_player);
            }
            
            for action in 0..num_available_actions {
                utility += utilities[action];
            }
            let mut strategy_branch = self.strategy_branch.borrow_mut();
            let strategy = strategy_branch.get_or_create_strategy(info_node_key, num_available_actions);
            strategy.update_strategy(utility, utilities, self.iteration);
            utility
        }
    }
    

    fn traverse_flop(&self) -> f64 {
        let previous_player = self.game_state.current_player.get();
        let previous_bets = self.game_state.bets_this_round.get();
        self.game_state.deal_flop();
        let utility = self.traverse_action();
        self.game_state.undeal_flop(previous_bets, previous_player);
        utility
    }

    fn traverse_deal(&self) -> f64 {
        let previous_player = self.game_state.current_player.get();
        let previous_bets = self.game_state.bets_this_round.get();
        self.game_state.deal();
        let utility = self.traverse_action();
        self.game_state.undeal(previous_bets, previous_player);
        utility
    }
}