use std::fmt::format;

use crate::{
    models::{
        card::{NineCardDeal, Rank},
        Player,
    },
    traversal::action_history::card_round_abstraction::CardRoundAbstraction, Card,
};

use super::card_round_abstraction::CardRoundAbstractionSerialised;

pub type GameAbstractionSerialised = Vec<u8>;

pub fn to_string_game_abstraction(hole1: Rank, hole2: Rank, suited: bool, is_sb: bool, abstraction: &GameAbstractionSerialised) -> String {
    let round = abstraction[0];
    let game_pot = abstraction[1];
    let bets_this_round = abstraction[2];
    let round_abstraction = CardRoundAbstraction::deserialise(&abstraction[3..]);

    format!(
        "{}{}{}{} {} p-bet: {} r-bets: {} {}",
        hole1,
        hole2,
        if suited { "s" } else { "o" },
        if is_sb { "SB" } else { "BB" },
        match round {
            0 => "P",
            1 => "F",
            2 => "T",
            3 => "R",
            _ => panic!("Invalid round {}", round),
        },
        game_pot,
        bets_this_round,
        round_abstraction
        )
}

#[derive(Default, Clone)]
pub struct GameAbstraction {
    traverser_round_abstractions: [CardRoundAbstractionSerialised; 4],
    opponent_round_abstractions: [CardRoundAbstractionSerialised; 4],
}

impl GameAbstraction {
    pub fn get_abstraction(
        &self,
        round: usize,
        game_pot: u8,
        bets_this_round: u8,
        current_player: &Player,
    ) -> GameAbstractionSerialised {
        // TODO - Compress this down
        // [_] = 8 bits
        // [round] [game pot] [round bets] [ ... round abstraction ...]

        let round_abstraction = match current_player {
            Player::Traverser => &self.traverser_round_abstractions[round],
            Player::Opponent => &self.opponent_round_abstractions[round],
        };

        let mut serialised = vec![];
        serialised.push(round as u8);
        serialised.push(game_pot);
        serialised.push(bets_this_round);
        serialised.extend(round_abstraction.clone());
        serialised
    }

    /// Replace the round abstraction for the current player without creating a whole new vec
    pub fn replace_round_abstraction(&self, game_abstraction_serialised: &mut GameAbstractionSerialised, round: usize, current_player: &Player) -> bool {
        let round_abstraction = match current_player {
            Player::Traverser => &self.traverser_round_abstractions[round],
            Player::Opponent => &self.opponent_round_abstractions[round],
        };
        let identical = game_abstraction_serialised[3..] == round_abstraction[..];
        if !identical {
            game_abstraction_serialised.splice(3.., round_abstraction.iter().cloned());
        }
        identical
    }
}

// TODO - Allow for 7 card abstractions
pub fn convert_deal_into_abstraction(deal: NineCardDeal) -> GameAbstraction {
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

    GameAbstraction {
        traverser_round_abstractions,
        opponent_round_abstractions,
    }
}