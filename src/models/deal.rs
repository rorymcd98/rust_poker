use super::card::Card;
use std::cell::RefCell;

pub struct Deal {
    pub traverser: (Card, Card),
    pub opponent: (Card, Card),
    pub community: RefCell<Vec<Card>>,
}

impl Deal {
    pub fn new_pre_flop() -> Deal {
        let mut cards = Card::new_random_cards(4);
        let mut drain = cards.drain(..);
        Deal {
            traverser: (drain.next().unwrap(), drain.next().unwrap()),
            opponent: (drain.next().unwrap(), drain.next().unwrap()),
            community: RefCell::new(vec![]),
        }
    }
}