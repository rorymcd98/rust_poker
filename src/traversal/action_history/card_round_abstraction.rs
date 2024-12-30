use std::fmt::Display;

use crate::{evaluate::generate_tables::remaining_hand_types::HandType, models::card::Card};

use super::{
    board_abstraction::BoardAbstraction,
    card_abstraction::{
        get_connected_card_abstraction, get_straight_abstraction, ConnectedCardsAbstraction,
        FlushAbstraction, StraightAbstraction,
    },
};

pub type CardRoundAbstractionSerialised = Vec<u8>;

pub struct CardRoundAbstraction {
    pub board_abstraction: BoardAbstraction,
    pub connected_cards_abstraction: Option<ConnectedCardsAbstraction>,
    pub straight_abstraction: Option<StraightAbstraction>,
    pub flush_abstraction: Option<FlushAbstraction>,
}

impl Display for CardRoundAbstraction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let connected_cards_display = match &self.connected_cards_abstraction {
            Some(connected_cards) => format!("{}", connected_cards),
            None => "None".to_string(),
        };

        let straight_cards_display = match &self.straight_abstraction {
            Some(straight) => format!("{}", straight),
            None => "None".to_string(),
        };

        let flush_cards_display = match &self.flush_abstraction {
            Some(flush) => format!("{}", flush),
            None => "None".to_string(),
        };


        write!(
            f,
            "Board {}\nConnected Cards: {}\nStraight: {}\nFlush: {}",
            self.board_abstraction,
            connected_cards_display,
            straight_cards_display,
            flush_cards_display
        )
    }
}

impl CardRoundAbstraction {
    pub fn new(hole_cards: &[Card; 2], board_cards: &[Card]) -> CardRoundAbstraction {
        let board_abstraction = BoardAbstraction::new(board_cards);
        let connected_cards_abstraction = get_connected_card_abstraction(hole_cards, board_cards);
        let straight_abstraction = get_straight_abstraction(hole_cards, board_cards);
        let flush_abstraction = None;

        CardRoundAbstraction {
            board_abstraction,
            connected_cards_abstraction,
            straight_abstraction,
            flush_abstraction,
        }
    }

    pub fn serialise(&self) -> CardRoundAbstractionSerialised {
        let mut serialised = vec![];
        serialised.push(self.board_abstraction.max_consecutive_cards);
        serialised.push(self.board_abstraction.suit_count_abstraction);
        let hand_type_serialised = match self.board_abstraction.board_hand_type {
            HandType::Pair(_) => 1,
            HandType::TwoPair(_, _) => 2,
            HandType::ThreeOfAKind(_) => 3,
            HandType::FullHouse(_, _) => 4,
            HandType::FourOfAKind(_) => 5,
            HandType::None => 0,
        };
        serialised.push(hand_type_serialised);

        if let Some(connected_cards_abstraction) = &self.connected_cards_abstraction {
            serialised.push(connected_cards_abstraction.serialise());
        } else {
            serialised.push(0);
        }

        if let Some(straight_abstraction) = &self.straight_abstraction {
            serialised.push(straight_abstraction.serialise());
        } else {
            serialised.push(0);
        }

        if let Some(flush_abstraction) = &self.flush_abstraction {
            serialised.push(flush_abstraction.serialise());
        } else {
            serialised.push(0);
        }

        serialised
    }
}
