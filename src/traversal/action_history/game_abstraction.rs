use std::fmt::format;

use crate::{
    models::{card::{NineCardDeal, Rank}, player, Player},
    traversal::action_history::card_round_abstraction::CardRoundAbstraction,
};

use super::{
    card_abstraction::HoleCardsAbstraction, card_round_abstraction::CardRoundAbstractionSerialised,
};

pub type GameAbstractionSerialised = Vec<u8>;

pub struct GameAbstraction {
    // TODO - Make sb_player, traverser_hole_cards, and opponent_hole_cards keys of the StrategyMap rather than members here
    // This will allow us to access strategies locklessly
    sb_player: Player,
    traverser_hole_cards: HoleCardsAbstraction,
    opponent_hole_cards: HoleCardsAbstraction,

    traverser_round_abstractions: [CardRoundAbstractionSerialised; 4],
    opponent_round_abstractions: [CardRoundAbstractionSerialised; 4],

    traverser_round_abstractions_un: [CardRoundAbstraction; 4],
    opponent_round_abstractions_un: [CardRoundAbstraction; 4],
    nine_card_deal: NineCardDeal,
}

impl GameAbstraction {
    pub fn get_abstraction(&self,
        round: usize,
        game_pot: u8,
        bets_this_round: u8,
        current_player: Player,
    ) -> GameAbstractionSerialised {
        // [] = 8 bits
        // [hole1] [hole2] [round bets (4bits) | xx | suited cards | is sb] [ ... round abstraction ...] [game pot] [round bets]
        
        let is_sb = current_player == self.sb_player;
        let hole_cards = match current_player {
            Player::Traverser => &self.traverser_hole_cards,
            Player::Opponent => &self.opponent_hole_cards,
        };
        let round_abstraction = match current_player {
            Player::Traverser => &self.traverser_round_abstractions[round],
            Player::Opponent => &self.opponent_round_abstractions[round],
        };

        let round_abstraction_un = match current_player {
            Player::Traverser => &self.traverser_round_abstractions_un[round],
            Player::Opponent => &self.opponent_round_abstractions_un[round],
        };

        // let board = format!("{}", self.nine_card_deal[4..].iter().map(|c| c.to_string()).collect::<Vec<String>>().join(" "));
        // println!("Deal: {}{}{}, {}", hole_cards.lower_card, hole_cards.higher_card, if hole_cards.suited { "s" } else { "o" }, board);
        // println!("Round {}, pot {}, bets {}, player {:?}", round, game_pot, bets_this_round, current_player);
        // println!("Round abstraction: \n{}", round_abstraction_un);
        // println!("");

        let mut serialised = vec![];
        serialised.push(hole_cards.lower_card.to_int());
        serialised.push(hole_cards.higher_card.to_int());
        serialised.push((bets_this_round << 4) | (hole_cards.suited as u8) << 1 | is_sb as u8);
        serialised.extend(round_abstraction.clone());
        serialised.push(game_pot);
        serialised.push(bets_this_round);
        serialised
    }
}

pub fn convert_deal_into_abstraction(
    deal: NineCardDeal,
    sb_player: Player,
) -> GameAbstraction {
    let traverser_hole_cards = HoleCardsAbstraction {
        lower_card: deal[0].rank,
        higher_card: deal[1].rank,
        suited: deal[0].suit == deal[1].suit,
    };

    let opponent_hole_cards = HoleCardsAbstraction {
        lower_card: deal[2].rank,
        higher_card: deal[3].rank,
        suited: deal[2].suit == deal[3].suit,
    };

    let traverser_round_abstractions = [
        CardRoundAbstraction::new(&[deal[0], deal[1]], &[]).serialise(),
        CardRoundAbstraction::new(&[deal[0], deal[1]], &deal[4..7]).serialise(),
        CardRoundAbstraction::new(&[deal[0], deal[1]], &deal[4..8]).serialise(),
        CardRoundAbstraction::new(&[deal[0], deal[1]], &deal[4..9]).serialise(),
    ];

    let opponent_round_abstractions = [
        CardRoundAbstraction::new(&[deal[2], deal[3]], &[]).serialise(),
        CardRoundAbstraction::new(&[deal[2], deal[3]], &deal[4..7]).serialise(),
        CardRoundAbstraction::new(&[deal[2], deal[3]], &deal[4..8]).serialise(),
        CardRoundAbstraction::new(&[deal[2], deal[3]], &deal[4..9]).serialise(),
    ];

    let traverser_round_abstractions_un = [
        CardRoundAbstraction::new(&[deal[0], deal[1]], &[]),
        CardRoundAbstraction::new(&[deal[0], deal[1]], &deal[4..7]),
        CardRoundAbstraction::new(&[deal[0], deal[1]], &deal[4..8]),
        CardRoundAbstraction::new(&[deal[0], deal[1]], &deal[4..9]),
    ];

    let opponent_round_abstractions_un = [
        CardRoundAbstraction::new(&[deal[2], deal[3]], &[]),
        CardRoundAbstraction::new(&[deal[2], deal[3]], &deal[4..7]),
        CardRoundAbstraction::new(&[deal[2], deal[3]], &deal[4..8]),
        CardRoundAbstraction::new(&[deal[2], deal[3]], &deal[4..9]),
    ];

    GameAbstraction {
        sb_player,
        traverser_hole_cards,
        opponent_hole_cards,
        traverser_round_abstractions,
        opponent_round_abstractions,
        // Using this for logging
        traverser_round_abstractions_un,
        opponent_round_abstractions_un,
        nine_card_deal: deal,
    }
}
