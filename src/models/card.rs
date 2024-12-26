use rand::Rng;
use std::hash::{Hash, Hasher};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Suit {
    Spades,
    Hearts,
    Diamonds,
    Clubs,   
}

impl Suit {
    pub fn from_int(suit: u8) -> Suit {
        match suit {
            0 => Suit::Spades,
            1 => Suit::Hearts,
            2 => Suit::Diamonds,
            3 => Suit::Clubs,
            _ => panic!("Invalid suit"),
        }
    }
    
    pub fn to_int(&self) -> u8 {
        match self {
            Suit::Spades => 0,
            Suit::Hearts => 1,
            Suit::Diamonds => 2,
            Suit::Clubs => 3,
        }
    }

    pub fn random() -> Suit {
        let mut rng = rand::thread_rng();
        Suit::from_int(rng.gen_range(0..4))
    }
}

impl Default for Suit {
    fn default() -> Self {
        Suit::Spades
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Rank {
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
    Ace,
}

impl Rank {
    pub fn from_int(rank: u8) -> Rank {
        match rank {
            0 => Rank::Two,
            1 => Rank::Three,
            2 => Rank::Four,
            3 => Rank::Five,
            4 => Rank::Six,
            5 => Rank::Seven,
            6 => Rank::Eight,
            7 => Rank::Nine,
            8 => Rank::Ten,
            9 => Rank::Jack,
            10 => Rank::Queen,
            11 => Rank::King,
            12 => Rank::Ace,
            _ => panic!("Invalid rank"),
        }
    }

    pub fn to_int(&self) -> u8 {
        match self {
            Rank::Two => 0,
            Rank::Three => 1,
            Rank::Four => 2,
            Rank::Five => 3,
            Rank::Six => 4,
            Rank::Seven => 5,
            Rank::Eight => 6,
            Rank::Nine => 7,
            Rank::Ten => 8,
            Rank::Jack => 9,
            Rank::Queen => 10,
            Rank::King => 11,
            Rank::Ace => 12,
        }
    }

    pub fn to_prime(&self) -> u8 {
        match self {
            Rank::Two => 2,
            Rank::Three => 3,
            Rank::Four => 5,
            Rank::Five => 7,
            Rank::Six => 11,
            Rank::Seven => 13,
            Rank::Eight => 17,
            Rank::Nine => 19,
            Rank::Ten => 23,
            Rank::Jack => 29,
            Rank::Queen => 31,
            Rank::King => 37,
            Rank::Ace => 41,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
}

impl Hash for Card {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u8(self.serialise());
    }
}

impl Card {
    pub fn new(suit: Suit, rank: Rank) -> Card {
        Card {
            suit,
            rank,
        }
    }

    pub fn from_ints(suit: u8, rank: u8) -> Card {
        Card {
            suit: Suit::from_int(suit),
            rank: Rank::from_int(rank),
        }
    }

    fn new_random_card() -> Card {
        let mut rng = rand::thread_rng();
        let card = rng.gen_range(0..52);

        let suit = card / 13;
        let rank = card % 13;
        Card::new(Suit::from_int(suit), Rank::from_int(rank))
    }

    pub fn new_random_cards(num_cards: usize) -> Vec::<Card> {
        let mut existing_cards = Vec::new();
        while existing_cards.len() < num_cards {
            let new_card = Card::new_random_card();
            if !existing_cards.contains(&new_card) {
                existing_cards.push(new_card);
            }
        }
        existing_cards
    }

    pub fn get_one_more_card(existing_cards: &Vec<Card>) -> Card {
        loop {
            let new_card = Card::new_random_card();
            if !existing_cards.contains(&new_card) {
                return new_card;
            }
        }
    }

    pub fn get_n_more_cards(existing_cards: &Vec<Card>, n: usize) -> Vec<Card> {
        if n + existing_cards.len() > 52 {
            panic!("Cannot get more than 52 cards");
        }

        let mut new_cards = Vec::new();
        while new_cards.len() < n {
            let new_card = Card::new_random_card();
            if !existing_cards.contains(&new_card) && !new_cards.contains(&new_card) {
                new_cards.push(new_card);
            }
        }
        new_cards
    }

    pub fn serialise(&self) -> u8 {
        let suit = match self.suit {
            Suit::Spades => 0,
            Suit::Hearts => 1,
            Suit::Diamonds => 2,
            Suit::Clubs => 3,
        };

        let rank = match self.rank {
            Rank::Two => 0,
            Rank::Three => 1,
            Rank::Four => 2,
            Rank::Five => 3,
            Rank::Six => 4,
            Rank::Seven => 5,
            Rank::Eight => 6,
            Rank::Nine => 7,
            Rank::Ten => 8,
            Rank::Jack => 9,
            Rank::Queen => 10,
            Rank::King => 11,
            Rank::Ace => 12,
        };

        (suit << 4) | rank
    }

    pub fn deserialise(serialised_card: u8) -> Card {
        let suit = Suit::from_int((serialised_card & 0b00110000) >> 4);
        let rank = Rank::from_int(serialised_card & 0b00001111);
        Card::new(suit, rank)
    }
 
    pub fn all_suited_combos(suit: Suit) -> impl Iterator<Item = (Card, Card)> {
        (0..12).flat_map(move |first_rank| {
            ((first_rank+1)..13).map({
                let suit = suit.clone();
                move |second_rank| {
                    (
                        Card::new(suit.clone(), Rank::from_int(first_rank)),
                        Card::new(suit.clone(), Rank::from_int(second_rank)),
                    )
                }
            })
        })
    }

    pub fn all_offsuit_combos(first_suit: Suit, second_suit: Suit) -> impl Iterator<Item = (Card, Card)> {
        (0..12).flat_map(move |first_rank| {
            ((first_rank+1)..13).map({
                let first_suit = first_suit.clone();
                let second_suit = second_suit.clone();
                move |second_rank| {
                    (
                        Card::new(first_suit.clone(), Rank::from_int(first_rank)),
                        Card::new(second_suit.clone(), Rank::from_int(second_rank)),
                    )
                }
            })
        })
    }
}

impl Default for Card {
    fn default() -> Self {
        Card::new(Suit::Spades, Rank::Two)
    }   
}

/// Conveniently, our serialisation function allows for sensible sorting of cards
impl PartialOrd for Card {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.serialise().cmp(&other.serialise()))
    }
}

impl Ord for Card {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.serialise().cmp(&other.serialise())
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use super::*;

    #[test]
    fn test_suit_from_int() {
        assert_eq!(Suit::from_int(0), Suit::Spades);
        assert_eq!(Suit::from_int(1), Suit::Hearts);
        assert_eq!(Suit::from_int(2), Suit::Diamonds);
        assert_eq!(Suit::from_int(3), Suit::Clubs);
    }

    #[test]
    #[should_panic(expected = "Invalid suit")]
    fn test_suit_from_int_invalid() {
        Suit::from_int(4);
    }

    #[test]
    fn test_rank_from_int() {
        assert_eq!(Rank::from_int(0), Rank::Two);
        assert_eq!(Rank::from_int(1), Rank::Three);
        assert_eq!(Rank::from_int(2), Rank::Four);
        assert_eq!(Rank::from_int(3), Rank::Five);
        assert_eq!(Rank::from_int(4), Rank::Six);
        assert_eq!(Rank::from_int(5), Rank::Seven);
        assert_eq!(Rank::from_int(6), Rank::Eight);
        assert_eq!(Rank::from_int(7), Rank::Nine);
        assert_eq!(Rank::from_int(8), Rank::Ten);
        assert_eq!(Rank::from_int(9), Rank::Jack);
        assert_eq!(Rank::from_int(10), Rank::Queen);
        assert_eq!(Rank::from_int(11), Rank::King);
        assert_eq!(Rank::from_int(12), Rank::Ace);
    }

    #[test]
    #[should_panic(expected = "Invalid rank")]
    fn test_rank_from_int_invalid() {
        Rank::from_int(13);
    }

    #[test]
    fn test_card_new() {
        let card = Card::new(Suit::Spades, Rank::Ace);
        assert_eq!(card.suit, Suit::Spades);
        assert_eq!(card.rank, Rank::Ace);
    }

    #[test]
    fn test_card_from_ints() {
        let card = Card::from_ints(0, 12);
        assert_eq!(card.suit, Suit::Spades);
        assert_eq!(card.rank, Rank::Ace);
    }

    #[rstest]
    #[case(Card::new(Suit::Spades, Rank::Two))]
    #[case(Card::new(Suit::Hearts, Rank::Three))]
    #[case(Card::new(Suit::Diamonds, Rank::Four))]
    #[case(Card::new(Suit::Clubs, Rank::King))]
    fn test_card_serialise_deserialise(#[case] card: Card) {
        let serialised = card.serialise();
        let deserialised = Card::deserialise(serialised);
        assert_eq!(card, deserialised);
    }

    #[test]
    fn test_new_random_cards() {
        let cards = Card::new_random_cards(5);
        assert_eq!(cards.len(), 5);
        // Check that all cards are unique
        for i in 0..5 {
            for j in i+1..5 {
                assert_ne!(cards[i], cards[j]);
            }
        }
    }

    #[test]
    fn test_get_n_more_cards() {
        let existing_cards = Card::new_random_cards(5);
        let new_cards = Card::get_n_more_cards(&existing_cards, 4);
        assert_eq!(new_cards.len(), 4);
        for card in new_cards {
            assert!(!existing_cards.contains(&card));
        }
    }
}
