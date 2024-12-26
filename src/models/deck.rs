use crate::models::Card;
use lazy_static::lazy_static;

pub struct Deck {
    pub cards: Vec<Card>,
}

impl Deck {
    pub fn new() -> Deck {
        let mut cards = Vec::new();
        for suit in 0..4 {
            for rank in 0..13 {
                cards.push(Card::from_ints(suit, rank));
            }
        }
        Deck {
            cards,
        }
    }
}

lazy_static! {
    pub static ref DECK: Deck = {
        Deck::new()
    };
}