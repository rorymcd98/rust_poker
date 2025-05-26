use lazy_static::lazy_static;

use super::super::action_history::game_abstraction::GameAbstraction;
use super::terminal_state::TerminalState;
use crate::config::*;
use crate::evaluate::evaluate_hand::{HandEvaluator, HandEvaluatorLookup};
use crate::models::card::{cards_string, NineCardDeal};
use crate::models::Card;
use crate::models::Player;
use crate::traversal::action_history::action::Action;
use crate::traversal::action_history::game_abstraction::{
    convert_deal_into_abstraction, GameAbstractionSerialised,
};
use std::cell::{LazyCell, OnceCell};
use std::fmt::Display;

lazy_static! {
    pub static ref EVALUATOR: HandEvaluatorLookup = HandEvaluatorLookup::new();
}

#[derive(Clone)]
pub struct GameStateHelper {
    pub game_abstraction: GameAbstraction,
    pub traverser_pot: u8,
    pub opponent_pot: u8,
    pub cards: [Card; 9],
    pub cards_dealt: u8,
    pub current_player: Player,
    pub small_blind_player: Player,
    pub big_blind_player: Player,
    pub bets_this_round: u8,
    pub winner: Option<Option<Player>>,
    pub checks_this_round: u8,
    pub folded: bool,
}

impl GameStateHelper {
    pub fn new(nine_card_deal: NineCardDeal, small_blind_player: Player) -> GameStateHelper {
        GameStateHelper {
            game_abstraction: convert_deal_into_abstraction(nine_card_deal),
            traverser_pot: match small_blind_player {
                Player::Traverser => SMALL_BLIND_SIZE,
                Player::Opponent => BIG_BLIND_SIZE,
            },
            opponent_pot: match small_blind_player {
                Player::Opponent => SMALL_BLIND_SIZE,
                Player::Traverser => BIG_BLIND_SIZE,
            },
            cards: nine_card_deal,
            cards_dealt: 0,
            current_player: small_blind_player,
            small_blind_player,
            big_blind_player: small_blind_player.get_opposite(),
            bets_this_round: 0,
            winner: None, // We will evaluate this on showdown
            checks_this_round: 0,
            folded: false,
        }
    }

    pub fn get_current_player(&self) -> Player {
        self.current_player
    }

    pub fn switch_current_player(&mut self) {
        self.current_player = self.current_player.get_opposite();
    }

    pub fn set_current_player_to_big_blind(&mut self) {
        self.current_player = self.big_blind_player;
    }

    pub fn is_preflop(&self) -> bool {
        self.cards_dealt == 0
    }

    pub fn is_preturn(&self) -> bool {
        self.cards_dealt < 4
    }

    pub fn is_river(&self) -> bool {
        self.cards_dealt == 5
    }

    pub fn get_num_available_actions(&self) -> usize {
        if self.get_current_player_pot() == SMALL_BLIND_SIZE {
            return 3; // we're preflop
        }
        match self.bets_this_round {
            0 => 2,
            MAX_RAISES => 2,
            _ => 3,
        }
    }

    pub fn get_current_player_cards(&self) -> [Card; 2] {
        match self.current_player {
            Player::Traverser => [self.cards[0], self.cards[1]],
            Player::Opponent => [self.cards[2], self.cards[3]],
        }
    }

    #[allow(dead_code)]
    pub fn get_non_current_player_cards(&self) -> [Card; 2] {
        match self.current_player {
            Player::Opponent => [self.cards[0], self.cards[1]],
            Player::Traverser => [self.cards[2], self.cards[3]],
        }
    }

    pub fn serialise_history_with_current_player(&self) -> GameAbstractionSerialised {
        self.game_abstraction.get_abstraction(
            (self.cards_dealt).saturating_sub(2) as usize,
            self.get_current_player_pot(),
            self.bets_this_round,
            &self.current_player,
        )
    }

    // TODO - massively refactor this method
    pub fn check_round_terminal(&self) -> TerminalState {
        if self.checks_this_round == 2 {
            return if self.is_river() {
                TerminalState::Showdown
            } else {
                TerminalState::RoundOver
            };
        }

        let terminal_state = if self.opponent_pot == self.traverser_pot {
            // If the pots are equal and there have been bets then this is a showdown / round over
            if self.bets_this_round > 0 {
                TerminalState::Showdown
            } else {
                // Otherwise its the first action
                TerminalState::None
            }
        } else if self.get_current_player_pot() < self.get_non_current_player_pot() {
            // If the pots are unequal and we have less in the pot, then it's our turn
            TerminalState::None
        } else {
            if !self.folded {
                panic!("Folding logic error");
            }
            // Otherwise the opponent just folded
            TerminalState::Fold
        };
        if !self.is_river() {
            match terminal_state {
                TerminalState::Fold => TerminalState::Fold,
                TerminalState::Showdown => TerminalState::RoundOver,
                _ => TerminalState::None,
            }
        } else {
            terminal_state
        }
    }

    // If we're at showdown, we lose our pot, or gain the opponent's pot
    pub fn evaluate_showdown(&mut self) -> f64 {
        let winner = self
            .winner
            .get_or_insert_with(|| EVALUATOR.evaluate_nine(&self.cards));
        match winner {
            Some(Player::Traverser) => self.opponent_pot as f64,
            Some(Player::Opponent) => -(self.traverser_pot as f64),
            None => 0.0,
        }
    }

    // If we're at fold, the other player has just folded, so the traverser should get their pot
    pub fn evaluate_fold(&self) -> f64 {
        // validate_history(&self.action_history.borrow().history);
        match self.current_player {
            Player::Traverser => self.opponent_pot as f64,
            Player::Opponent => -(self.traverser_pot as f64),
        }
    }

    pub fn bet(&mut self) -> Action {
        self.bets_this_round += 1;
        let raise = if self.is_preturn() {
            // In limit hold'em, typically the pre-flop and flop use a 'small bet' (1bb), the later rounds use a 'big bet' (2bb)
            BIG_BLIND_SIZE
        } else {
            BIG_BLIND_SIZE * 2
        };
        match self.current_player {
            Player::Traverser => {
                self.traverser_pot = self.opponent_pot + raise;
            }
            Player::Opponent => {
                self.opponent_pot = self.traverser_pot + raise;
            }
        };
        Action::Bet
    }

    pub fn get_current_player_pot(&self) -> u8 {
        match self.current_player {
            Player::Traverser => self.traverser_pot,
            Player::Opponent => self.opponent_pot,
        }
    }

    pub fn get_non_current_player_pot(&self) -> u8 {
        match self.current_player {
            Player::Traverser => self.opponent_pot,
            Player::Opponent => self.traverser_pot,
        }
    }

    pub fn call_or_bet(&mut self) -> Action {
        if self.get_current_player_pot() == SMALL_BLIND_SIZE {
            // Handle the first preflop call
            self.checks_this_round += 1; // We increment this so that any follow-up check terminates the round
            return self.call();
        }
        match self.bets_this_round {
            0 => self.bet(), // Handles the start of betting rounds
            _ => self.call(),
        }
    }

    pub fn call(&mut self) -> Action {
        match self.current_player {
            Player::Traverser => {
                self.traverser_pot = self.opponent_pot;
            }
            Player::Opponent => {
                self.opponent_pot = self.traverser_pot;
            }
        };
        Action::Call
    }

    pub fn checkfold(&mut self) {
        if self.bets_this_round == 0 && self.get_current_player_pot() != SMALL_BLIND_SIZE {
            self.checks_this_round += 1;
        } else {
            self.folded = true;
        }
    }

    // Set the state back to a previous action
    pub fn undo(
        &mut self,
        acting_player: Player,
        previous_pot: u8,
        previous_bets: u8,
        previous_checks: u8,
    ) {
        match acting_player {
            Player::Traverser => {
                self.traverser_pot = previous_pot;
            }
            Player::Opponent => {
                self.opponent_pot = previous_pot;
            }
        };
        self.bets_this_round = previous_bets;
        self.current_player = acting_player;
        self.checks_this_round = previous_checks; // TODO - think of a more optimal way to determine checks
        self.folded = false; // TODO - Currently I'm using folded to validate my main folding logic  (pot A < pot B)
    }

    // implement deal and undeal
    pub fn deal(&mut self) {
        self.cards_dealt += 1;
        self.checks_this_round = 0;
        self.bets_this_round = 0;
        self.set_current_player_to_big_blind();
    }

    pub fn undeal(&mut self, previous_bets: u8, previous_player: Player, previous_checks: u8) {
        self.cards_dealt -= 1;
        self.bets_this_round = previous_bets;
        self.current_player = previous_player;
        self.checks_this_round = previous_checks;
    }

    pub fn deal_flop(&mut self) {
        self.cards_dealt = 3;
        self.bets_this_round = 0;
        self.checks_this_round = 0;
        self.set_current_player_to_big_blind();
    }

    pub fn undeal_flop(&mut self, previous_bets: u8, previous_player: Player, previous_checks: u8) {
        self.cards_dealt = 0;
        self.bets_this_round = previous_bets;
        self.current_player = previous_player;
        self.checks_this_round = previous_checks;
    }
}

impl Display for GameStateHelper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Current state: Player Cards: {} Cards dealt: {} Current player: {}\nTraverser pot: {} Opponent pot: {} Bets this round: {} Checks this round: {}",
            cards_string(&self.get_current_player_cards()),
            cards_string(&self.cards[4..4+self.cards_dealt as usize]),
            self.get_current_player(),
            self.traverser_pot,
            self.opponent_pot,
            self.bets_this_round,
            self.checks_this_round
        )
    }
}

// TODO - testing: check if the game state is terminal after performing a variety of actions, etc.
