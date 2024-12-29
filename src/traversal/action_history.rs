use core::panic;

use itertools::Itertools;

use crate::traversal::action::Action;

pub fn validate_history(history: &Vec<Action>) {
    let mut seen = vec![];

    match history[0] {
        Action::Deal(card) => {
            seen.push(card);
        },
        _ => panic!("First action should be a deal: {:?}", history),
    }
    match history[1] {
        Action::Deal(card) => {
            seen.push(card);
            if card.serialise() <= seen[0].serialise() {
                panic!("Hole cards should be sorted for InfoSet serialisation: {:?}", history);
            }
        },
        _ => panic!("Second action should be a deal: {:?}", history),
    }


    if history.len() < 3 {
        panic!("Not enough actions in history: {:?}", history);
    }

    if history.len() > 3 && history[2].is_checkfold() {
        panic!("SB preflop checkfold should be impossible: {:?}", history);
    }

    let mut prev = history[1].clone();
    let mut prev_prev = history[0].clone();
    let mut deal_count = 0;
    let mut bets_this_turn = 0;

    fn is_round_over(prev_prev: &Action, prev: &Action) -> bool {
        match [prev_prev, prev] {
            [Action::CheckFold, Action::CheckFold] => true,
            [Action::Bet, Action::Call] => true,
            [Action::Call, Action::CheckFold] => true,
            _ => false,
        }
    }

    for action in &history[2..] {
        match action {
            Action::Deal(card) => {
                bets_this_turn = 0;
                seen.push(card.clone());
                deal_count += 1;
                match deal_count {
                    1 => {
                        if prev.is_deal() {
                            panic!("No actions occurred preflop: {:?}", history);
                        }
                    },
                    2 => {
                        if !prev.is_deal() {
                            panic!("Only 1 flop card dealt: {:?}", history);
                        }
                    },
                    3 => {
                        if !prev.is_deal() {
                            panic!("Only 2 flop card dealt: {:?}", history);
                        }
                    },
                    4 => {
                        if prev.is_deal() {
                            panic!("No action occured on the flop: {:?}", history);
                        }
                    },
                    5 => {
                        if prev.is_deal() {
                            panic!("No actions occurred on the turn: {:?}", history);
                        }
                    },
                    _ => panic!("Too many deals: {:?}", history),
                }
                if deal_count > 5 {
                    panic!("Too many deals: {:?}", history);
                }

                if prev_prev.is_bet() && prev.is_checkfold() {
                    panic!("Shouldn't be dealing, opponent just folded: {:?}", history);
                }
                if (deal_count == 1 || deal_count > 3) && !is_round_over(&prev_prev, &prev) {
                    panic!("Shouldn't be dealing, round wasn't over ({:?}, {:?}) : {:?}", prev_prev, prev, history);
                }
            },
            Action::CheckFold => {
                if bets_this_turn > 0 && prev.is_checkfold() {
                    panic!("Double checkfold after bet: {:?}", history);
                }
            },
            Action::Call => {
                if prev.is_call() {
                    panic!("Double call: {:?}", history);
                }
            },
            Action::Bet => {
                bets_this_turn += 1;
                if bets_this_turn > 4 {
                    panic!("Too many bets this turn: {:?}", history);
                }
            },
        }
        prev_prev = prev.clone();
        prev = action.clone();
    }
    let seen_len = seen.len();
    let unique_seen_len = seen.iter().unique().collect::<Vec<_>>().len();
    if seen_len != unique_seen_len {
        while seen.len() > 0 {
            let cand = seen.pop();
            if seen.contains(&cand.unwrap()) {
                panic!("Duplicate card ({}) in history: {:?}", cand.unwrap(), history);
            }
        }
    }
}

// A live record of the game state that also acts as a key to the various strategies
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionHistory {
    pub history: Vec::<Action>, // TODO - Try to make this a mutable reference
}

impl ActionHistory {
    pub fn new(history: Vec<Action>) -> ActionHistory {
        ActionHistory {
            history,
        }
    }

    pub fn set_hole_cards(&mut self, hole_card_one: Action, hole_card_two: Action) {
        self.history[0] = hole_card_one;
        self.history[1] = hole_card_two;
    }

    /// Serialisation
    /// Currently we are using a whole u8 to represent an action
    /// When we come back to optimise this, we can use either 2 or 8 bits to represent an action
    pub fn serialise(&self) -> Vec<u8> {
        let is_special = self.history.len() == 3 && self.history[2].is_call(); // In the case of a preflop bb call

        let mut serialised_history = Vec::new();

        for action in &*self.history {
            serialised_history.push(action.serialise());
        }
        let last_action = self.history.last().unwrap();

        if is_special {
            serialised_history.push(Action::DEAL_BYTE);
        } else {
            serialised_history.push(Self::get_terminal_serialisation(last_action));
        }
        serialised_history
    }

    // TODO - this needs to be reconfisdered for the strategy_branch
    // Create an 'impossible' 2-bit or 8-bit sequence indicating to the deserialiser that the history is complete
    fn get_terminal_serialisation(action: &Action) -> u8 {
        match action {
            Action::CheckFold => Action::CALL_BYTE, // can't Call after CheckFold
            Action::Call => Action::BET_BYTE, // can't Call after a Bet (except in the case of a preflop bb call)  
            Action::Bet => Action::DEAL_BYTE, // can't Deal after a Bet
            Action::Deal(_) => Action::DEAL_BYTE, // can't Deal after a Deal (flop is handled by deserialiser)
        }
    }
}

pub struct ActionHistoryByteStreamIterator<'a> {
    byte_stream_iterator: std::slice::Iter<'a, u8>,
}

impl ActionHistoryByteStreamIterator<'_> {

    pub fn new<'a>(byte_stream: &'a Vec<u8>) -> ActionHistoryByteStreamIterator<'a> {
        ActionHistoryByteStreamIterator {
            byte_stream_iterator: byte_stream.iter(),
        }
    }

    // Check if the current action is an 'impossible' action given the previous action
    fn is_terminal_serialiastion(prev_action: &Action, current_action: &Action) -> bool {
        match prev_action {
            Action::CheckFold => match current_action {
                Action::Call => true,
                _ => false,
            },
            Action::Call => match current_action {
                Action::Bet => true, // except preflop bb call
                _ => false,
            },
            Action::Bet => match current_action {
                Action::Deal(_) => true,
                _ => false,
            },
            Action::Deal(_) => match current_action {
                Action::Deal(_) => true,
                _ => false,
            },
        }
    }
}

impl<'a> Iterator for ActionHistoryByteStreamIterator<'a> {

    type Item = ActionHistory;

    fn next(&mut self) -> Option<ActionHistory> {
        let mut history = Vec::<Action>::new();

        // Hole cards
        let hole_card_one = Action::deserialise(self.byte_stream_iterator.next().expect("First hole card failed to be iterated to"));
        let hole_card_two = Action::deserialise(self.byte_stream_iterator.next().expect("Second hole card failed to be iterated to"));
    
        assert!(hole_card_one.is_deal(), "Hole card one should be a card");
        assert!(hole_card_two.is_deal(), "Hole card two should be a card");
    
        history.push(hole_card_one);
        history.push(hole_card_two);
    
        let first_action = Action::deserialise(self.byte_stream_iterator.next().expect("First action failed to be iterated to"));
        if first_action.is_deal() {
            return Some(ActionHistory::new(history)); // Return remaining iterator
        }
        history.push(first_action);
    
        let second_action = Action::deserialise(self.byte_stream_iterator.next().expect("Second action failed to be iterated to"));
        if history.last().unwrap().is_call() && second_action.is_deal() {
            return Some(ActionHistory::new(history)); // Return remaining iterator
        }
        history.push(second_action);
    
        while let Some(byte) = self.byte_stream_iterator.next() {
            let action = Action::deserialise(&byte);
            if Self::is_terminal_serialiastion(&history.last().unwrap(), &action) {
                return Some(ActionHistory::new(history));
            } else if action.is_deal() {
                if Self::is_terminal_serialiastion(&history.last().unwrap(), &action) {
                    return Some(ActionHistory::new(history));
                }
                history.push(action);
    
                let second_flop_card = Action::deserialise(self.byte_stream_iterator.next().expect("Second flop card failed to be iterated to"));
                let third_flop_card = Action::deserialise(self.byte_stream_iterator.next().expect("Third flop card failed to be iterated to"));
    
                assert!(second_flop_card.is_deal(), "Flop card two should be a card");
                assert!(third_flop_card.is_deal(), "Flop card three should be a card");

                history.push(second_flop_card);
                history.push(third_flop_card);
                break;
            }  else {
                history.push(action);
            }
        }
    
        while let Some(byte) = self.byte_stream_iterator.next() {
            let action = Action::deserialise(byte);
            if Self::is_terminal_serialiastion(history.last().unwrap(), &action) {
                return Some(ActionHistory::new(history));
            }
            history.push(action);
        }
    
        panic!("Deserialisation failed! We should have found a terminal action by now");
    }
}


#[cfg(test)]
mod tests {
    use crate::models::card::{Rank, Suit, Card};

    use super::*;
    use rstest::rstest;

    fn validate_history(history: Vec<Action>) {
        let action_history = ActionHistory::new(history);
        let serialised = action_history.serialise();
        let mut action_history_iterator = ActionHistoryByteStreamIterator::new(&serialised);

        let deserialised_history = action_history_iterator.next().unwrap();
        assert_eq!(deserialised_history, action_history);    
    }

    #[test]
    fn test_serialise_history_currentplayer() {
        let history = vec![
            Action::Deal(Card::new(Suit::Spades, Rank::Two)),
            Action::Deal(Card::new(Suit::Hearts, Rank::Three)),
        ];
        validate_history(history);
    }

    #[test]
    fn test_serialise_history_preflop() {
        let history = vec![
            Action::Deal(Card::new(Suit::Spades, Rank::Two)),
            Action::Deal(Card::new(Suit::Hearts, Rank::Three)),
            Action::Call,
            Action::Bet,
            Action::Bet,
            Action::Call,
            Action::CheckFold,
        ];
        validate_history(history);
    }

    #[test]
    fn test_serialise_history_flop() {
        let history = vec![
            Action::Deal(Card::new(Suit::Spades, Rank::Two)),
            Action::Deal(Card::new(Suit::Hearts, Rank::Three)),
            Action::Call,
            Action::Bet,
            Action::Bet,
            Action::Bet,
            Action::Bet,
            Action::Call,
            Action::Deal(Card::new(Suit::Clubs, Rank::Four)),
            Action::Deal(Card::new(Suit::Diamonds, Rank::Five)),
            Action::Deal(Card::new(Suit::Spades, Rank::Six)),
        ];
        validate_history(history);
    }


    #[test]
    fn test_serialise_history_flop_play() {
        let history = vec![
            Action::Deal(Card::new(Suit::Spades, Rank::Two)),
            Action::Deal(Card::new(Suit::Hearts, Rank::Three)),
            Action::Call,
            Action::Bet,
            Action::Bet,
            Action::Bet,
            Action::Bet,
            Action::Call,
            Action::Deal(Card::new(Suit::Clubs, Rank::Four)),
            Action::Deal(Card::new(Suit::Diamonds, Rank::Five)),
            Action::Deal(Card::new(Suit::Spades, Rank::Six)),
            Action::CheckFold,
            Action::Bet,
            Action::Bet,
            Action::Call,
        ];
        validate_history(history);
    }

    #[test]
    fn test_serialise_history_turn() {
        let history = vec![
            Action::Deal(Card::new(Suit::Spades, Rank::Two)),
            Action::Deal(Card::new(Suit::Hearts, Rank::Three)),
            Action::Call,
            Action::Bet,
            Action::Bet,
            Action::Bet,
            Action::Bet,
            Action::Call,
            Action::Deal(Card::new(Suit::Clubs, Rank::Four)),
            Action::Deal(Card::new(Suit::Diamonds, Rank::Five)),
            Action::Deal(Card::new(Suit::Spades, Rank::Six)),
            Action::CheckFold,
            Action::Bet,
            Action::Bet,
            Action::Bet,
            Action::Bet,
            Action::Call,
            Action::Deal(Card::new(Suit::Clubs, Rank::Eight)),
        ];
        validate_history(history);
    }

    #[test]
    fn test_serialise_history_river() {
        let history = vec![
            Action::Deal(Card::new(Suit::Spades, Rank::Two)),
            Action::Deal(Card::new(Suit::Hearts, Rank::Three)),
            Action::Call,
            Action::Bet,
            Action::Bet,
            Action::Bet,
            Action::Bet,
            Action::Call,
            Action::Deal(Card::new(Suit::Clubs, Rank::Four)),
            Action::Deal(Card::new(Suit::Diamonds, Rank::Five)),
            Action::Deal(Card::new(Suit::Spades, Rank::Six)),
            Action::CheckFold,
            Action::Bet,
            Action::Bet,
            Action::Bet,
            Action::Bet,
            Action::Call,
            Action::Deal(Card::new(Suit::Clubs, Rank::Eight)),
            Action::CheckFold,
            Action::Bet,
            Action::Bet,
            Action::Bet,
            Action::Bet,
            Action::Call,
            Action::Deal(Card::new(Suit::Clubs, Rank::Eight))
        ];
        validate_history(history);
    }

    #[test]
    fn test_serialise_history_reveal() {
        let history = vec![
            Action::Deal(Card::new(Suit::Spades, Rank::Two)),
            Action::Deal(Card::new(Suit::Hearts, Rank::Three)),
            Action::Call,
            Action::Bet,
            Action::Bet,
            Action::Bet,
            Action::Bet,
            Action::Call,
            Action::Deal(Card::new(Suit::Clubs, Rank::Four)),
            Action::Deal(Card::new(Suit::Diamonds, Rank::Five)),
            Action::Deal(Card::new(Suit::Spades, Rank::Six)),
            Action::CheckFold,
            Action::Bet,
            Action::Bet,
            Action::Bet,
            Action::Bet,
            Action::Call,
            Action::Deal(Card::new(Suit::Clubs, Rank::Eight)),
            Action::CheckFold,
            Action::Bet,
            Action::Bet,
            Action::Bet,
            Action::Bet,
            Action::Call,
            Action::Deal(Card::new(Suit::Clubs, Rank::Eight)),
            Action::CheckFold,
            Action::Bet,
            Action::Bet,
            Action::Bet,
            Action::Bet,
            Action::Call,
        ];
        validate_history(history);
    }

    #[test]
    fn test_serialise_multiple_histories() {
        let history1 = vec![
            Action::Deal(Card::new(Suit::Spades, Rank::Two)),
            Action::Deal(Card::new(Suit::Hearts, Rank::Three)),
            Action::Call,
            Action::Bet,
        ];

        let history2 = vec![
            Action::Deal(Card::new(Suit::Clubs, Rank::Two)),
            Action::Deal(Card::new(Suit::Diamonds, Rank::Three)),
            Action::Call,
            Action::Bet,
            Action::Bet,
            Action::Bet,
        ];

        let history3 = vec![
            Action::Deal(Card::new(Suit::Spades, Rank::Two)),
            Action::Deal(Card::new(Suit::Hearts, Rank::Three)),
            Action::Call,
            Action::Bet,
            Action::Bet,
            Action::Bet,
            Action::Bet,
            Action::Call,
            Action::Deal(Card::new(Suit::Clubs, Rank::Four)),
            Action::Deal(Card::new(Suit::Diamonds, Rank::Five)),
            Action::Deal(Card::new(Suit::Spades, Rank::Six)),
            Action::CheckFold,
            Action::Bet,
            Action::Bet,
        ];

        let mut serialised1 = ActionHistory::new(history1.clone()).serialise();
        let serialised2 = ActionHistory::new(history2.clone()).serialise();
        let serialised3 = ActionHistory::new( history3.clone()).serialise();

        serialised1.extend(serialised2);
        serialised1.extend(serialised3);


        let mut action_history_iterator = ActionHistoryByteStreamIterator::new(&serialised1);

        let deserialised_history1 = action_history_iterator.next().unwrap();
        assert_eq!(deserialised_history1, ActionHistory::new(history1));

        let deserialised_history2 = action_history_iterator.next().unwrap();
        assert_eq!(deserialised_history2, ActionHistory::new(history2));

        let deserialised_history3 = action_history_iterator.next().unwrap();
        assert_eq!(deserialised_history3, ActionHistory::new(history3));
    }
}
