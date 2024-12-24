use crate::Card;
use crate::models::card::Suit;
use crate::models::Player;
use crate::traversal::action::Action;
use super::{action, action_history, strategy};
use super::strategy::{strategy_map::StrategyMap, strategy_branch::StrategyBranch};
use super::action_history::ActionHistory;
struct GameStateHelper {
    action_history: ActionHistory,
    traverser_pot: u8,
    opponent_pot: u8,
    traverser_hand: [Card; 2],
    opponent_hand: [Card; 2],
    current_player: Player,
    small_blind_player: Player,
    board: Vec<Card>,
    bets_this_round: u8,
}

impl GameStateHelper {
    pub fn new(action_history: ActionHistory, traverser_hand: [Card; 2], current_turn: Player, small_blind_player: Player) -> GameStateHelper {
        GameStateHelper {
            action_history,
            traverser_pot: 0,
            opponent_pot: 0,
            traverser_hand,
            opponent_hand: [Card::default(), Card::default()],
            current_player: current_turn,
            small_blind_player,
            board: Vec::<Card>::new(),
            bets_this_round: 0,
        }
    }

    pub fn get_current_player(&self) -> &Player {
        &self.current_player
    }

    pub fn switch_current_player(&mut self) {
        self.current_player = self.current_player.get_opposite();
    }

    pub fn set_current_player_to_small_blind(&mut self) {
        self.current_player = self.small_blind_player.clone();
    }

    pub fn set_current_player_to_big_blind(&mut self) {
        self.current_player = self.small_blind_player.get_opposite();
    }

    pub fn get_combined_hands(&self) -> Vec<Card> {
        let mut combined_hand = self.traverser_hand.to_vec();
        combined_hand.extend_from_slice(&self.opponent_hand);
        combined_hand
    }

    pub fn get_combined_cards(&self) -> Vec<Card> {
        let mut combined_cards = self.board.to_vec();
        combined_cards.extend_from_slice(&self.get_combined_hands());
        combined_cards
    }

    pub fn get_num_available_actions(&self) -> usize {
        match self.bets_this_round {
            4 => 2,
            _ => 3,
        }
    }

    pub fn is_round_over(&self) -> bool {
        // TODO - implement this
        false
    }

    pub fn bet(&mut self) {
        self.action_history.history.push(Action::Bet);
        match self.current_player {
            Player::Traverser => {
                self.traverser_pot += 2;
                self.bets_this_round += 1;
            },
            Player::Opponent => {
                self.opponent_pot += 2;
                self.bets_this_round += 1;
            },
        }
    }

    pub fn unbet(&mut self) {
        self.action_history.history.pop();
        match self.current_player {
            Player::Traverser => {
                self.traverser_pot -= 2;
                self.bets_this_round -= 1;
            },
            Player::Opponent => {
                self.opponent_pot -= 2;
                self.bets_this_round -= 1;
            },
        };
    }

    pub fn get_current_player_pot(&self) -> u8 {
        match self.current_player {
            Player::Traverser => self.traverser_pot,
            Player::Opponent => self.opponent_pot,
        }
    }

    pub fn call(&mut self) {
        self.action_history.history.push(Action::Call);
        match self.current_player {
            Player::Traverser => {
                self.traverser_pot = self.opponent_pot;
            },
            Player::Opponent => {
                self.opponent_pot = self.traverser_pot;
            },
        };
    }

    pub fn uncall(&mut self, previous_pot: u8) {
        self.action_history.history.pop();
        match self.current_player {
            Player::Traverser => {
                self.traverser_pot = previous_pot;
            },
            Player::Opponent => {
                self.opponent_pot = previous_pot;
            },
        };
    }

    pub fn checkfold(&mut self) {
        self.action_history.history.push(Action::CheckFold);
    }

    pub fn uncheckfold(&mut self) {
        self.action_history.history.pop();
    }
}

struct TreeTraverser {
    strategy_map: StrategyMap,
    iterations: usize,
}

pub fn new() -> TreeTraverser {
    TreeTraverser {
        strategy_map: StrategyMap::new(),
        iterations: 1000,
    }
}

impl TreeTraverser {
    pub fn BeginTreeTraversal(&mut self){
        let players = vec![Player::Traverser, Player::Opponent];
    
        for player in players { // Alternate between who is small blind
            let action_history = &mut ActionHistory::new(player.clone(), vec![]);
            for card_combo in Card::all_suited_combos(Suit::Spades) {
                let card_combo_print = card_combo.clone();
                // This defines our branch - our hole cards and whether we're the traverser
                let traverser_cards = [card_combo.0, card_combo.1];
                let strategy_branch = self.strategy_map.get_or_create_strategy_branch(action_history.serialise());
                
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

struct BranchTraverser<'a> {
    strategy_branch: &'a mut StrategyBranch,
    iteration: usize,
    game_state: GameStateHelper,
}

impl<'a> BranchTraverser<'a> {
    pub fn new(strategy_branch: &mut StrategyBranch, game_state: GameStateHelper) -> BranchTraverser {
        BranchTraverser {
            strategy_branch,
            iteration: 1,
            game_state,
        }
    }

    pub fn begin_traversal(&mut self) -> f64 {
    
        let players = vec![Player::Traverser, Player::Opponent];

        let mut opponent_cards = Card::get_n_more_cards(&self.game_state.traverser_hand.to_vec(), 2);
        opponent_cards.sort();
        self.game_state.opponent_hand = [opponent_cards[0].clone(), opponent_cards[1].clone()];
        
        self.game_state.action_history.history.push(Action::Deal(opponent_cards[0].clone()));
        self.game_state.action_history.history.push(Action::Deal(opponent_cards[1].clone()));
        let utility = self.traverse_action();
        self.game_state.action_history.history.pop();
        self.game_state.action_history.history.pop();
        utility
    }

    fn traverse_action(&mut self) -> f64 {
        self.game_state.action_history.current_player = self.game_state.get_current_player().clone();
    
        // Isolate the mutable borrow of `strategy_branch`
        let strategy_key = self.game_state.action_history.serialise();
        let num_available_actions = self.game_state.get_num_available_actions() as usize;
        let strategy = self.strategy_branch.get_or_create_strategy(strategy_key, num_available_actions);
        
        if self.game_state.current_player.clone().is_opponent() {
            let sampled_action = strategy.sample_strategy();

            let pot_before_action = self.game_state.get_current_player_pot();
            match sampled_action {
                0 => self.game_state.bet(),
                // 1 => self.game_state.call(),
                // 2 => self.game_state.checkfold(),
                _ => panic!("Invalid action"),
            };
            let previous_player = self.game_state.current_player.clone();
            self.game_state.switch_current_player();

            // Propagate up the randomly selected action multiplied by the chance of selecting it
            let utility = self.traverse_action();

            match sampled_action {
                0 => self.game_state.unbet(),
                1 => self.game_state.uncall(pot_before_action),
                2 => self.game_state.checkfold(),
                _ => panic!("Invalid action"),
            };

            self.game_state.current_player = previous_player; // not neccessary but for clarity

            utility
        } else {
            let mut utility = 0.0;
            let mut utilities = vec![0.0; num_available_actions];

            for action in 0..num_available_actions {
                let previous_player = self.game_state.current_player.clone();
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
    
                self.game_state.current_player = previous_player;
            }
            
            for action in 0..num_available_actions {
                utility += utilities[action];
            }

            strategy.update_strategy(utility, utilities, self.iteration.clone());

            utility
        }
    }
    

    fn traverse_flop(&mut self) -> f64 {
        let mut three_more_cards = Card::get_n_more_cards(&self.game_state.get_combined_cards(), 3);
        three_more_cards.sort();
        for card in three_more_cards {
            self.game_state.board.push(card.clone());
            self.game_state.action_history.history.push(Action::Deal(card));
        }
        self.game_state.set_current_player_to_big_blind();
        let utility = self.traverse_action();
        for _ in 0..3 {
            self.game_state.board.pop();
            self.game_state.action_history.history.pop();
        }
        utility
    }

    fn traverse_deal(&mut self) -> f64 {
        let card = Card::get_one_more_card(&self.game_state.get_combined_cards());
        
        self.game_state.board.push(card.clone());
        self.game_state.action_history.history.push(Action::Deal(card));
        
        let utility = self.traverse_action();

        self.game_state.board.pop();
        self.game_state.action_history.history.pop();
        utility
    }
}