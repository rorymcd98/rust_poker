use crate::models::Card;
use crate::models::Suit;
use crate::models::Player;
use crate::traversal::action::Action;
use super::{action, action_history, strategy};
use super::strategy::{strategy_map::StrategyMap, strategy_branch::StrategyBranch};
use super::action_history::ActionHistory;
use std::cell::{Cell, RefCell};

struct GameStateHelper {
    action_history: RefCell<ActionHistory>,
    traverser_pot: Cell<u8>,
    opponent_pot: Cell<u8>,
    traverser_hand: [Card; 2],
    opponent_hand: [Card; 2],
    current_player: Cell<Player>,
    small_blind_player: Player,
    board: RefCell<Vec<Card>>,
    bets_this_round: Cell<u8>,
}

impl GameStateHelper {
    pub fn new(action_history: ActionHistory, traverser_hand: [Card; 2], current_turn: Player, small_blind_player: Player) -> GameStateHelper {
        GameStateHelper {
            action_history: RefCell::new(action_history),
            traverser_pot: Cell::new(0),
            opponent_pot: Cell::new(0),
            traverser_hand,
            opponent_hand: [Card::default(), Card::default()],
            current_player: Cell::new(current_turn),
            small_blind_player,
            board: RefCell::new(Vec::<Card>::new()),
            bets_this_round: Cell::new(0),
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

    pub fn get_combined_hands(&self) -> Vec<Card> {
        let mut combined_hand = self.traverser_hand.to_vec();
        combined_hand.extend_from_slice(&self.opponent_hand);
        combined_hand
    }

    pub fn get_combined_cards(&self) -> Vec<Card> {
        let mut combined_cards = self.board.borrow().clone();
        combined_cards.extend_from_slice(&self.get_combined_hands());
        combined_cards
    }

    pub fn get_num_available_actions(&self) -> usize {
        match self.bets_this_round.get() {
            4 => 2,
            _ => 3,
        }
    }

    pub fn is_round_over(&self) -> bool {
        let action_history = &self.action_history.borrow().history;
        let last_two_actions = &action_history[action_history.len() - 2..];
        match last_two_actions {
            [Action::CheckFold, Action::CheckFold] => true,
            [Action::Call, Action::CheckFold] => true,
            [Action::Bet, Action::CheckFold] => true,
            [Action::Call, Action::Call] => panic!("Invalid state - should not have two calls in a row"), 
            [Action::Bet, Action::Call] => true,
            _ => false,
        }
    }

    pub fn is_terminal_state(&self) -> bool {
        let action_history = &self.action_history.borrow().history;
        let last_two_actions = &action_history[action_history.len() - 2..];
        // TODO - handle the preflop edge case
        let is_river = self.board.borrow().len() == 5;
        is_river || match last_two_actions {
            [Action::Bet, Action::CheckFold] => true,
            _ => false,
        }
    }

    fn evaluate_terminal_state(&self) -> f64 {
        0.0
    }

    fn evaluate_shodown(&self) -> f64 {
        0.0
    }

    fn evaluate_terminal_fold(&self) -> f64 {
        match self.current_player.get() {
            Player::Traverser => -(self.traverser_pot.get() as f64),
            Player::Opponent => self.opponent_pot.get() as f64,
        }
    }

    pub fn bet(&self) {
        self.action_history.borrow_mut().history.push(Action::Bet);
        match self.current_player.get() {
            Player::Traverser => {
                self.traverser_pot.set(self.traverser_pot.get() + 2);
                self.bets_this_round.set(self.bets_this_round.get() + 1);
            },
            Player::Opponent => {
                self.opponent_pot.set(self.opponent_pot.get() + 2);
                self.bets_this_round.set(self.bets_this_round.get() + 1);
            },
        }
    }

    pub fn unbet(&self) {
        self.action_history.borrow_mut().history.pop();
        match self.current_player.get() {
            Player::Traverser => {
                self.traverser_pot.set(self.traverser_pot.get() - 2);
                self.bets_this_round.set(self.bets_this_round.get() - 1);
            },
            Player::Opponent => {
                self.opponent_pot.set(self.opponent_pot.get() - 2);
                self.bets_this_round.set(self.bets_this_round.get() - 1);
            },
        };
    }

    pub fn get_current_player_pot(&self) -> u8 {
        match self.current_player.get() {
            Player::Traverser => self.traverser_pot.get(),
            Player::Opponent => self.opponent_pot.get(),
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
}

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

    pub fn BeginTreeTraversal(self){
        let players = vec![Player::Traverser, Player::Opponent];
    
        for player in players { // Alternate between who is small blind
            let action_history = &mut ActionHistory::new(player.clone(), vec![]);
            for card_combo in Card::all_suited_combos(Suit::Spades) {
                let card_combo_print = card_combo.clone();
                // This defines our branch - our hole cards and whether we're the traverser
                let traverser_cards = [card_combo.0, card_combo.1];
                let strategy_branch = StrategyBranch::new();
                
                action_history.history.push(Action::Deal(traverser_cards[0].clone()));
                action_history.history.push(Action::Deal(traverser_cards[1].clone()));

                let game_state = GameStateHelper::new(action_history.clone(), traverser_cards, player.clone(), player.clone());
                
                let mut branch_traverser = BranchTraverser::new(strategy_branch, game_state);
                let utility = branch_traverser.begin_traversal();
                println!("Utility of card {:?}: {}", card_combo_print, utility);
            }
        }
    }
}

struct BranchTraverser {
    strategy_branch: RefCell::<StrategyBranch>,
    iteration: Cell::<usize>,
    game_state: GameStateHelper,
}

impl BranchTraverser {
    pub fn new(strategy_branch: StrategyBranch, game_state: GameStateHelper) -> BranchTraverser {
        BranchTraverser {
            strategy_branch: RefCell::new(strategy_branch),
            iteration: Cell::new(1),
            game_state,
        }
    }

    pub fn begin_traversal(&mut self) -> f64 {
        let mut opponent_cards = Card::get_n_more_cards(&self.game_state.traverser_hand.to_vec(), 2);
        opponent_cards.sort();
        self.game_state.opponent_hand = [opponent_cards[0].clone(), opponent_cards[1].clone()];
        
        self.game_state.action_history.borrow_mut().history.push(Action::Deal(opponent_cards[0].clone()));
        self.game_state.action_history.borrow_mut().history.push(Action::Deal(opponent_cards[1].clone()));
        let utility = self.traverse_action();
        self.game_state.action_history.borrow_mut().history.pop();
        self.game_state.action_history.borrow_mut().history.pop();
        utility
    }

    fn traverse_action(&self) -> f64 {
        if self.game_state.is_round_over(){
            if self.game_state.is_terminal_state(){
                return self.game_state.evaluate_terminal_state();
            }
            self.game_state.set_current_player_to_small_blind();
            return self.traverse_deal();
        }

        self.game_state.action_history.borrow_mut().current_player = self.game_state.get_current_player();
    
        // Isolate the mutable borrow of `strategy_branch`
        let strategy_key = self.game_state.action_history.borrow().serialise();
        let num_available_actions = self.game_state.get_num_available_actions() as usize;
        let mut strategy_branch = self.strategy_branch.borrow_mut();
        let strategy = strategy_branch.get_or_create_strategy(strategy_key, num_available_actions);
        
        if self.game_state.current_player.get().is_opponent() {
            let sampled_action = strategy.sample_strategy();

            let pot_before_action = self.game_state.get_current_player_pot();
            match sampled_action {
                0 => self.game_state.bet(),
                1 => self.game_state.call(),
                2 => self.game_state.checkfold(),
                _ => panic!("Invalid action"),
            };
            let previous_player = self.game_state.current_player.get();
            self.game_state.switch_current_player();

            // Propagate up the randomly selected action multiplied by the chance of selecting it
            let utility = self.traverse_action();

            match sampled_action {
                0 => self.game_state.unbet(),
                1 => self.game_state.uncall(pot_before_action),
                2 => self.game_state.uncheckfold(),
                _ => panic!("Invalid action"),
            };

            self.game_state.current_player.set(previous_player); // not neccessary but for clarity

            utility
        } else {
            let mut utility = 0.0;
            let mut utilities = vec![0.0; num_available_actions];

            for action in 0..num_available_actions {
                let previous_player = self.game_state.current_player.get();
                let pot_before_action = self.game_state.get_current_player_pot();
                match action {
                    0 => self.game_state.bet(),
                    1 => self.game_state.call(),
                    2 => self.game_state.checkfold(),
                    _ => panic!("Invalid action"),
                };

                utilities[action] = self.traverse_action() * strategy.current_strategy[action];

                match action {
                    0 => self.game_state.unbet(),
                    1 => self.game_state.uncall(pot_before_action),
                    2 => self.game_state.checkfold(),
                    _ => panic!("Invalid action"),
                };
    
                self.game_state.current_player.set(previous_player);
            }
            
            for action in 0..num_available_actions {
                utility += utilities[action];
            }

            strategy.update_strategy(utility, utilities, self.iteration.get());

            utility
        }
    }
    

    fn traverse_flop(&self) -> f64 {
        let mut three_more_cards = Card::get_n_more_cards(&self.game_state.get_combined_cards(), 3);
        three_more_cards.sort();
        for card in three_more_cards {
            self.game_state.board.borrow_mut().push(card.clone());
            self.game_state.action_history.borrow_mut().history.push(Action::Deal(card));
        }
        self.game_state.set_current_player_to_big_blind();
        let utility = self.traverse_action();
        for _ in 0..3 {
            self.game_state.board.borrow_mut().pop();
            self.game_state.action_history.borrow_mut().history.pop();
        }
        utility
    }

    fn traverse_deal(&self) -> f64 {
        let card = Card::get_one_more_card(&self.game_state.get_combined_cards());
        
        self.game_state.board.borrow_mut().push(card.clone());
        self.game_state.action_history.borrow_mut().history.push(Action::Deal(card));
        
        let utility = self.traverse_action();

        self.game_state.board.borrow_mut().pop();
        self.game_state.action_history.borrow_mut().history.pop();
        utility
    }
}