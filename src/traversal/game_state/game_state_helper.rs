use lazy_static::lazy_static;

use crate::config::*;
use crate::evaluate::evaluate_hand::{HandEvaluator, HandEvaluatorLookup};
use crate::traversal::action_history::game_abstraction::{GameAbstractionSerialised, convert_deal_into_abstraction};
use crate::models::card::{cards_string, NineCardDeal};
use crate::models::Player;
use crate::models::Card;
use std::cell::Cell;
use super::super::action_history::game_abstraction::GameAbstraction;
use super::terminal_state::TerminalState;

lazy_static! {
    pub static ref EVALUATOR: HandEvaluatorLookup = HandEvaluatorLookup::new();
}

pub struct GameStateHelper {
    pub game_abstraction: GameAbstraction,
    pub traverser_pot: Cell<u8>,
    pub opponent_pot: Cell<u8>,
    pub cards: [Card; 9],
    pub cards_dealt: Cell<u8>,
    pub current_player: Cell<Player>,
    pub small_blind_player: Player,
    pub big_blind_player: Player,
    pub bets_this_round: Cell<u8>,
    pub winner: Option<Player>,
    pub checks_this_round: Cell<u8>,
}

impl GameStateHelper {
    pub fn new(nine_card_deal: NineCardDeal, small_blind_player: Player) -> GameStateHelper {
        GameStateHelper {
            game_abstraction: convert_deal_into_abstraction(nine_card_deal, small_blind_player),
            traverser_pot: Cell::new(if small_blind_player == Player::Traverser {
                SMALL_BLIND
            } else {
                BIG_BLIND
            }),
            opponent_pot: Cell::new(if small_blind_player == Player::Opponent {
                SMALL_BLIND
            } else {
                BIG_BLIND
            }),
            cards: nine_card_deal,
            cards_dealt: Cell::new(0),
            current_player: Cell::new(small_blind_player),
            small_blind_player,
            big_blind_player: small_blind_player.get_opposite(),
            bets_this_round: Cell::new(0),
            winner: EVALUATOR.evaluate_deal(&nine_card_deal),
            checks_this_round: Cell::new(0),
        }
    }

    pub fn get_current_player(&self) -> Player {
        self.current_player.get()
    }

    pub fn switch_current_player(&self) {
        self.current_player
            .set(self.current_player.get().get_opposite());
    }

    pub fn set_current_player_to_small_blind(&self) {
        self.current_player.set(self.small_blind_player);
    }

    pub fn set_current_player_to_big_blind(&self) {
        self.current_player.set(self.big_blind_player);
    }

    pub fn get_flop(&self) -> [Card; 3] {
        [self.cards[4], self.cards[5], self.cards[6]]
    }

    pub fn is_preflop(&self) -> bool {
        self.cards_dealt.get() == 0
    }

    pub fn is_flop(&self) -> bool {
        self.cards_dealt.get() == 3
    }

    pub fn is_turn(&self) -> bool {
        self.cards_dealt.get() == 4
    }

    pub fn is_river(&self) -> bool {
        self.cards_dealt.get() == 5
    }

    pub fn get_num_available_actions(&self) -> usize {
        if self.get_current_player_pot() == 1 {
            return 3; // we're preflop
        }
        match self.bets_this_round.get() {
            0 => 2,
            MAX_RAISES => 2,
            _ => 3,
        }
    }

    pub fn get_current_player_cards(&self) -> [Card; 2] {
        match self.current_player.get() {
            Player::Traverser => [self.cards[0], self.cards[1]],
            Player::Opponent => [self.cards[2], self.cards[3]],
        }
    }

    pub fn get_non_current_player_cards(&self) -> [Card; 2] {
        match self.current_player.get() {
            Player::Opponent => [self.cards[0], self.cards[1]],
            Player::Traverser => [self.cards[2], self.cards[3]],
        }
    }

    pub fn serialise_history_with_current_player(&self) -> GameAbstractionSerialised {
        println!("Serialising with current player: {:?}", self.current_player.get());
        println!("Cards dealt: {}", self.cards_dealt.get());
        
        let res = self.game_abstraction.get_abstraction(
            (self.cards_dealt.get()).saturating_sub(2) as usize,
            self.get_current_player_pot(),
            self.bets_this_round.get(),
            self.current_player.get(),
        );
        if self.current_player.get() != self.small_blind_player && self.current_player.get().is_traverser() && self.bets_this_round.get() == 0 && self.get_current_player_pot() == 2 && self.checks_this_round.get() == 0 && self.cards_dealt.get() == 0 {
        // println!("Preflop {:?} , cards {}", res, cards_string(&self.get_current_player_cards()));
        }
        res 
    }

    pub fn check_round_terminal(&self) -> TerminalState {
        if self.checks_this_round.get() == 2 {
            return TerminalState::Showdown;
        }

        let terminal_state = if self.opponent_pot.get() == self.traverser_pot.get() {
            // If the pots are equal and there have been bets then this is a showdown / round over
            if self.bets_this_round.get() > 0 {
                TerminalState::Showdown
            } else {
                // Otherwise its the first action
                TerminalState::None
            }
        } else {
            if self.get_current_player_pot() < self.get_non_current_player_pot() {
                // If the pots are unequal and we have less in the pot, then it's our turn
                TerminalState::None
            } else {
                // Otherwise the opponent just folded
                TerminalState::Fold
            }
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
    pub fn evaluate_showdown(&self) -> f32 {
        match self.winner {
            Some(Player::Traverser) => self.opponent_pot.get() as f32,
            Some(Player::Opponent) => -(self.traverser_pot.get() as f32),
            None => 0.0,
        }
    }

    // If we're at fold, the other player has just folded, so the traverser should get their pot
    pub fn evaluate_fold(&self) -> f32 {
        // validate_history(&self.action_history.borrow().history);
        match self.current_player.get() {
            Player::Traverser => self.opponent_pot.get() as f32,
            Player::Opponent => -(self.traverser_pot.get() as f32),
        }
    }

    pub fn bet(&self) {
        self.bets_this_round.set(self.bets_this_round.get() + 1);
        let raise = if self.is_preflop() {
            BIG_BLIND
        } else {
            BIG_BLIND * 2
        };
        match self.current_player.get() {
            Player::Traverser => {
                self.traverser_pot.set(self.opponent_pot.get() + raise);
            }
            Player::Opponent => {
                self.opponent_pot.set(self.traverser_pot.get() + raise);
            }
        }
    }

    pub fn get_current_player_pot(&self) -> u8 {
        match self.current_player.get() {
            Player::Traverser => self.traverser_pot.get(),
            Player::Opponent => self.opponent_pot.get(),
        }
    }

    pub fn get_non_current_player_pot(&self) -> u8 {
        match self.current_player.get() {
            Player::Traverser => self.opponent_pot.get(),
            Player::Opponent => self.traverser_pot.get(),
        }
    }

    pub fn call_or_bet(&self) {
        if self.get_current_player_pot() == SMALL_BLIND {
            return self.call();
        }
        match self.bets_this_round.get() {
            0 => self.bet(), // Handles the start of betting rounds
            _ => self.call(),
        }
    }

    pub fn call(&self) {
        if self.get_current_player_pot() == 2 {
            self.checkfold(); // Pseudo check
        }
        match self.current_player.get() {
            Player::Traverser => {
                self.traverser_pot.set(self.opponent_pot.get());
            }
            Player::Opponent => {
                self.opponent_pot.set(self.traverser_pot.get());
            }
        };
    }

    pub fn checkfold(&self) {
        if self.bets_this_round.get() == 0 {
            self.checks_this_round.set(self.checks_this_round.get() + 1);
        }
    }

    // Set the state back to a previous action
    pub fn undo(
        &self,
        acting_player: Player,
        previous_pot: u8,
        previous_bets: u8,
        previous_checks: u8,
    ) {
        match acting_player {
            Player::Traverser => {
                self.traverser_pot.set(previous_pot);
            }
            Player::Opponent => {
                self.opponent_pot.set(previous_pot);
            }
        };
        self.bets_this_round.set(previous_bets);
        self.current_player.set(acting_player);
        self.checks_this_round.set(previous_checks); // TODO - think of a more optimal way to determine checks
    }

    // implement deal and undeal
    pub fn deal(&self) {
        self.cards_dealt.set(self.cards_dealt.get() + 1);
        self.checks_this_round.set(0);
        self.bets_this_round.set(0);
        self.set_current_player_to_big_blind();
    }

    pub fn undeal(&self, previous_bets: u8, previous_player: Player, previous_checks: u8) {
        self.cards_dealt.set(self.cards_dealt.get() - 1);
        self.bets_this_round.set(previous_bets);
        self.current_player.set(previous_player);
        self.checks_this_round.set(previous_checks);
    }

    pub fn deal_flop(&self) {
        self.cards_dealt.set(3);
        self.bets_this_round.set(0);
        self.checks_this_round.set(0);
        self.set_current_player_to_big_blind();
    }

    pub fn undeal_flop(&self, previous_bets: u8, previous_player: Player, previous_checks: u8) {
        self.cards_dealt.set(0);
        self.bets_this_round.set(previous_bets);
        self.current_player.set(previous_player);
        self.checks_this_round.set(previous_checks);
    }

    pub fn current_state_as_string(&self) -> String{
        format!(
            "Current state: Player Cards: {} Cards dealt: {} Current player: {}\nTraverser pot: {} Opponent pot: {} Bets this round: {} Checks this round: {}",
            cards_string(&self.get_current_player_cards()),
            {
                cards_string(&self.cards[4..4+self.cards_dealt.get() as usize])
            },
            self.get_current_player(),
            self.traverser_pot.get(),
            self.opponent_pot.get(),
            self.bets_this_round.get(),
            self.checks_this_round.get()
        )
    }
}