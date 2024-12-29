use crate::models::card::Card;

use super::{board_abstraction::BoardAbstraction, card_abstraction::{get_connected_card_abstraction, get_straight_abstraction, ConnectedCardsAbstraction, FlushAbstraction, StraightAbstraction}};

pub type CardRoundAbstractionSerialised = Vec<u8>;

pub struct CardRoundAbstraction { 
    pub board_abstraction: BoardAbstraction,
    pub connected_cards_abstraction: Option<ConnectedCardsAbstraction>,
    pub straight_abstraction: Option<StraightAbstraction>,
    pub flush_abstraction: Option<FlushAbstraction>,
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
            flush_abstraction
        }
    }

    pub fn serialise(&self) -> CardRoundAbstractionSerialised {
        // let mut serialised = vec![];
        // serialised.push(self.board_abstraction.max_consecutive_cards);
        // serialised.push(self.board_abstraction.max_suit_count);
        // serialised.push(self.board_abstraction.board_hand_type.to_int());
        // serialised.push(self.connected_cards_abstraction.as_ref().map_or(0, |a| a.to_int()));
        // serialised.push(self.straight_abstraction.as_ref().map_or(0, |a| a.to_int()));
        // serialised.push(self.flush_abstraction.as_ref().map_or(0, |a| a.to_int()));
        // serialised
        vec![]
    }
}