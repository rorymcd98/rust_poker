use std::fmt::Display;

use crate::models::card::Card;

use super::{
    board_abstraction::BoardAbstraction,
    card_abstraction::{
        get_connected_card_abstraction, get_flush_abstraction, get_straight_abstraction,
        ConnectedCardsAbstraction, FlushAbstraction, StraightAbstraction,
    },
};

pub type CardRoundAbstractionSerialised = Vec<u8>;

#[derive(Default)]
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
            "Board {} Connected Cards: {} Straight: {} Flush: {}",
            self.board_abstraction,
            connected_cards_display,
            straight_cards_display,
            flush_cards_display
        )
    }
}

impl CardRoundAbstraction {
    pub fn new(hole_cards: &[Card; 2], board_cards: &[Card]) -> CardRoundAbstraction {
        debug_assert!(
            hole_cards[0].to_int() < hole_cards[1].to_int(),
            "Hole cards are not sorted: {} {}",
            hole_cards[0],
            hole_cards[1]
        );

        let board_abstraction = BoardAbstraction::new(board_cards);
        let connected_cards_abstraction = get_connected_card_abstraction(hole_cards, board_cards);
        let straight_abstraction = get_straight_abstraction(hole_cards, board_cards);
        let flush_abstraction = get_flush_abstraction(hole_cards, board_cards);

        CardRoundAbstraction {
            board_abstraction,
            connected_cards_abstraction,
            straight_abstraction,
            flush_abstraction,
        }
    }

    pub fn serialise(&self) -> CardRoundAbstractionSerialised {
        let mut serialised = self.board_abstraction.serialise();

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

    pub fn deserialise(serialised: &[u8]) -> CardRoundAbstraction {
        let board_abstraction = BoardAbstraction::deserialise(&serialised[0..3]);

        let connected_cards_abstraction = if serialised[3] == 0 {
            None
        } else {
            Some(ConnectedCardsAbstraction::deserialise(&serialised[3]))
        };

        let straight_abstraction = if serialised[4] == 0 {
            None
        } else {
            Some(StraightAbstraction::deserialise(&serialised[4]))
        };

        let flush_abstraction = if serialised[5] == 0 {
            None
        } else {
            Some(FlushAbstraction::deserialise(&serialised[6]))
        };

        CardRoundAbstraction {
            board_abstraction,
            connected_cards_abstraction,
            straight_abstraction,
            flush_abstraction,
        }
    }
}
