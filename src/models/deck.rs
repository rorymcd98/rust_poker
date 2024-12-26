use crate::models::Card;

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
