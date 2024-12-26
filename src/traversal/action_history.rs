use crate::traversal::action::Action;
use crate::models::card::Card;
use crate::models::card::Rank;
use crate::models::card::Suit;
use crate::models::player::Player;

// A live record of the game state that also acts as a key to the various strategies
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionHistory {
    pub current_player: Player,
    pub history: Vec::<Action>, // TODO - Try to make this a mutable reference
}

impl ActionHistory {
    pub fn new(current_player: Player, history: Vec<Action>) -> ActionHistory {
        ActionHistory {
            current_player: current_player,
            history: history,
        }
    }

    /// Serialisation
    /// Currently we are using a whole u8 to represent an action
    /// When we come back to optimise this, we can use either 2 or 8 bits to represent an action
    pub fn serialise(&self) -> Vec<u8> {
        let is_special = self.history.len() == 3 && self.history[2].is_call(); // In the case of a preflop bb call

        let mut serialised_history = Vec::new();
        serialised_history.push(self.current_player.serialise());

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
        let current_player_byte = self.byte_stream_iterator.next();
        if current_player_byte.is_none() {
            return None;
        }
        
        let current_player = Player::deserialise(current_player_byte.unwrap());
    
        // Hole cards
        let hole_card_one = Action::deserialise(self.byte_stream_iterator.next().expect("First hole card failed to be iterated to"));
        let hole_card_two = Action::deserialise(self.byte_stream_iterator.next().expect("Second hole card failed to be iterated to"));
    
        assert!(hole_card_one.is_deal(), "Hole card one should be a card");
        assert!(hole_card_two.is_deal(), "Hole card two should be a card");
    
        history.push(hole_card_one);
        history.push(hole_card_two);
    
        let first_action = Action::deserialise(self.byte_stream_iterator.next().expect("First action failed to be iterated to"));
        if first_action.is_deal() {
            return Some(ActionHistory::new(current_player, history)); // Return remaining iterator
        }
        history.push(first_action);
    
        let second_action = Action::deserialise(self.byte_stream_iterator.next().expect("Second action failed to be iterated to"));
        if history.last().unwrap().is_call() && second_action.is_deal() {
            return Some(ActionHistory::new(current_player, history)); // Return remaining iterator
        }
        history.push(second_action);
    
        while let Some(byte) = self.byte_stream_iterator.next() {
            let action = Action::deserialise(&byte);
            if Self::is_terminal_serialiastion(&history.last().unwrap(), &action) {
                return Some(ActionHistory::new(current_player, history));
            } else if action.is_deal() {
                if Self::is_terminal_serialiastion(&history.last().unwrap(), &action) {
                    return Some(ActionHistory::new(current_player, history));
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
                return Some(ActionHistory::new(current_player, history));
            }
            history.push(action);
        }
    
        panic!("Deserialisation failed! We should have found a terminal action by now");
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn validate_history(history: Vec<Action>) {
        let action_history = ActionHistory::new(Player::Opponent, history);
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

        let mut serialised1 = ActionHistory::new(Player::Opponent, history1.clone()).serialise();
        let serialised2 = ActionHistory::new(Player::Traverser, history2.clone()).serialise();
        let serialised3 = ActionHistory::new(Player::Opponent, history3.clone()).serialise();

        serialised1.extend(serialised2);
        serialised1.extend(serialised3);


        let mut action_history_iterator = ActionHistoryByteStreamIterator::new(&serialised1);

        let deserialised_history1 = action_history_iterator.next().unwrap();
        assert_eq!(deserialised_history1, ActionHistory::new(Player::Opponent, history1));

        let deserialised_history2 = action_history_iterator.next().unwrap();
        assert_eq!(deserialised_history2, ActionHistory::new(Player::Traverser, history2));

        let deserialised_history3 = action_history_iterator.next().unwrap();
        assert_eq!(deserialised_history3, ActionHistory::new(Player::Opponent, history3));
    }



    #[rstest]
    #[case(Player::Traverser)]
    #[case(Player::Opponent)]
    fn maintains_player_state(#[case] player: Player) {
        let history = vec![
            Action::Deal(Card::new(Suit::Spades, Rank::Three)),
            Action::Deal(Card::new(Suit::Spades, Rank::Three)),
        ];
        let action_history = ActionHistory::new(player.clone(), history.clone());
        let serialised = action_history.serialise();

        let mut action_history_iterator = ActionHistoryByteStreamIterator::new(&serialised);
        let deserialised_history = action_history_iterator.next().unwrap();
        assert_eq!(deserialised_history.current_player, player.clone());
        assert_ne!(deserialised_history.current_player, player.get_opposite());
    }
}
