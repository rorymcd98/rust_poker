use crate::{
    models::{
        card::{Card, NineCardDeal, Rank},
        Player,
    },
    traversal::action_history::card_round_abstraction::CardRoundAbstraction,
};

use super::card_round_abstraction::CardRoundAbstractionSerialised;

pub type GameAbstractionSerialised = Vec<u8>;

#[allow(dead_code)]
pub fn to_string_game_abstraction(
    hole1: Rank,
    hole2: Rank,
    suited: bool,
    is_sb: bool,
    abstraction: &GameAbstractionSerialised,
) -> String {
    let round = abstraction[0];
    let current_player_pot = abstraction[1];
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
        current_player_pot,
        bets_this_round,
        round_abstraction
    )
}

/// The GameAbstraction allows us to compress the information state for each player into a few important pieces of information such as:
/// - A compressed view of the action that lead to this point (how much has been bet, whos turn it is)
/// - How each player's hand secretly connects with the board (do they have a straight or a flush draw, any pairs/sets/quads etc.)
///
/// Remark: This is massively important as it means we can begin to accumulate regrets for similar situations while training
#[derive(Default, Clone)]
pub struct GameAbstraction {
    pub traverser_round_abstractions: [CardRoundAbstractionSerialised; 4],
    pub opponent_round_abstractions: [CardRoundAbstractionSerialised; 4],
}

impl GameAbstraction {
    pub fn get_abstraction(
        &self,
        round: usize,
        current_player_pot: u8,
        bets_this_round: u8,
        current_player: &Player,
    ) -> GameAbstractionSerialised {
        // [_] = 8 bits
        // [round] [game pot] [round bets] [ ... round abstraction ...]
        let round_abstraction = match current_player {
            Player::Traverser => &self.traverser_round_abstractions[round],
            Player::Opponent => &self.opponent_round_abstractions[round],
        };

        let mut serialised = vec![];
        serialised.push(round as u8);
        serialised.push(current_player_pot);
        serialised.push(bets_this_round);
        serialised.extend(round_abstraction.clone());
        serialised
    }

    pub fn get_abstraction_from_round(
        round: usize,
        current_player_pot: u8,
        bets_this_round: u8,
        round_abstraction: CardRoundAbstractionSerialised,
    ) -> GameAbstractionSerialised {
        // [_] = 8 bits
        // [round] [game pot] [round bets] [ ... round abstraction ...]
        let mut serialised = vec![];
        serialised.push(round as u8);
        serialised.push(current_player_pot);
        serialised.push(bets_this_round);
        serialised.extend(round_abstraction.clone());
        serialised
    }

    /// Replace the round abstraction for the current player without creating a whole new vec
    pub fn replace_round_abstraction(
        &self,
        game_abstraction_serialised: &mut GameAbstractionSerialised,
        round: usize,
        current_player: &Player,
    ) -> bool {
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

/// Get the game abstarction for the current game state
pub fn get_current_abstraction(
    hole_cards: &(Card, Card),
    board_cards: &[Card],
    round: usize,
    current_player_pot: u8,
    bets_this_round: u8,
) -> GameAbstractionSerialised {
    let card_round_abstraction = convert_cards_into_card_abstraction(hole_cards, board_cards);
    GameAbstraction::get_abstraction_from_round(
        round,
        current_player_pot,
        bets_this_round,
        card_round_abstraction,
    )
}

fn convert_cards_into_card_abstraction(
    hole_cards: &(Card, Card),
    board_cards: &[Card],
) -> CardRoundAbstractionSerialised {
    CardRoundAbstraction::new(&[hole_cards.0, hole_cards.1], board_cards).serialise()
}

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
