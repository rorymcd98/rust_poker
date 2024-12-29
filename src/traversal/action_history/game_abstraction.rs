use crate::models::card::Card;
use crate::{
    models::{card::NineCardDeal, Player},
    traversal::action_history::card_round_abstraction::CardRoundAbstraction,
};

use super::{
    card_abstraction::HoleCardsAbstraction, card_round_abstraction::CardRoundAbstractionSerialised,
};

pub struct GameAbstraction {
    pub is_sb: bool,
    pub hole_cards: HoleCardsAbstraction, // the last element will be suited
    pub round_abstractions: [CardRoundAbstractionSerialised; 4],
    pub round_bets: [u8; 4], // the last element will be the largest opponent pot size
}

pub fn convert_deal_into_abstraction(
    deal: NineCardDeal,
    player: Player,
    is_sb: bool,
) -> GameAbstraction {
    let first_deal_index = match player {
        Player::Traverser => 0,
        Player::Opponent => 2,
    };
    let hole_card1 = deal[first_deal_index];
    let hole_card2 = deal[first_deal_index + 1];
    debug_assert!(hole_card1.to_int() < hole_card2.to_int());
    let hole_cards: [Card; 2] = [hole_card1, hole_card2];

    let round_abstractions = [
        CardRoundAbstraction::new(&hole_cards, &[]).serialise(),
        CardRoundAbstraction::new(&hole_cards, &deal[4..7]).serialise(),
        CardRoundAbstraction::new(&hole_cards, &deal[4..8]).serialise(),
        CardRoundAbstraction::new(&hole_cards, &deal[4..9]).serialise(),
    ];

    let hole_cards = HoleCardsAbstraction {
        lower_card: hole_card1.rank,
        higher_card: hole_card2.rank,
        suited: hole_card1.suit == hole_card2.suit,
    };

    GameAbstraction {
        is_sb,
        round_abstractions,
        hole_cards,
        round_bets: [0, 0, 0, 0],
    }
}
