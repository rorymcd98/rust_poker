use crate::Card;

use super::{board_abstraction::BoardAbstraction, card_abstraction::{ConnectedCardsAbstraction, FlushAbstraction, StraightAbstraction}};

type CardRoundAbstractionSerialised = Vec<u8>;

pub struct CardRoundAbstraction { 
    pub board_abstraction: BoardAbstraction,
    pub connected_cards_abstraction: Option<ConnectedCardsAbstraction>,
    pub straight_abstraction: Option<StraightAbstraction>,
    pub flush_abstraction: Option<FlushAbstraction>,
}

impl CardRoundAbstraction {
    pub fn new(hole_cards: &[Card; 2], board_cards: &[Card]) -> CardRoundAbstraction {
        let board_abstraction = BoardAbstraction::new(board_cards);
        let connected_cards_abstraction = None;
        let straight_abstraction = None;
        let flush_abstraction = None;

        CardRoundAbstraction {
            board_abstraction,
            connected_cards_abstraction,
            straight_abstraction,
            flush_abstraction
        }
    }
}