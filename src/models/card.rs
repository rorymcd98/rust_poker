use itertools::Itertools;
use rand::Rng;
use std::{fmt::{Display, Formatter}, hash::{Hash, Hasher}};
use crate::thread_utils::with_rng;

// 2 for traverser, 2 for opponent, 5 for board
pub type NineCardDeal = [Card; 9];

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[derive(Default)]
pub enum Suit {
    #[default]
    Spades,
    Hearts,
    Diamonds,
    Clubs,   
}

impl Display for Suit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Suit::Spades => "s",
            Suit::Hearts => "h",
            Suit::Diamonds => "d",
            Suit::Clubs => "c",
        })
    }
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

    pub fn to_bit(&self) -> u32 {
        1 << self.to_int()
    }

    pub fn random() -> Suit {
        with_rng(|rng| Suit::from_int(rng.gen_range(0..4)))
    }
}


#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[derive(Default)]
pub enum Rank {
    #[default]
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


impl Display for Rank {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Rank::Two => "2",
            Rank::Three => "3",
            Rank::Four => "4",
            Rank::Five => "5",
            Rank::Six => "6",
            Rank::Seven => "7",
            Rank::Eight => "8",
            Rank::Nine => "9",
            Rank::Ten => "T",
            Rank::Jack => "J",
            Rank::Queen => "Q",
            Rank::King => "K",
            Rank::Ace => "A",
        })
    }
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

    pub fn to_bit(&self) -> u32 {
        1 << self.to_int()
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
}

impl Display for Card {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.rank, self.suit)
    }
}

pub fn deal_string(deal: &NineCardDeal) -> String {
    format!("P1 [{}, {}] P2 [{}, {}] Board [{}, {}, {}, {}, {}]",
        deal[0], deal[1], deal[2], deal[3],
        deal[4], deal[5], deal[6], deal[7], deal[8])
}

pub fn cards_string(cards: &[Card]) -> String {
    cards.iter().map(|card| card.to_string()).join(" ")
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

    pub fn from_int(card_number: u8) -> Card {
        let suit = card_number / 13;
        let rank = card_number % 13;
        Card::new(Suit::from_int(suit), Rank::from_int(rank))
    }

    pub fn to_int(&self) -> u8 {
        self.suit.to_int() * 13 + self.rank.to_int()
    }

    pub fn from_ints(suit: u8, rank: u8) -> Card {
        Card {
            suit: Suit::from_int(suit),
            rank: Rank::from_int(rank),
        }
    }

    fn new_random_card() -> Card {
        let card_int = with_rng(|rng| rng.gen_range(0..52));
        Card::from_int(card_int)
    }

    pub fn new_random_cards(num_cards: usize) -> Vec<Card> {
        let mut taken = [false; 52];
        let mut res = Vec::with_capacity(num_cards);
        with_rng(|rng|
            while res.len() < num_cards {
                let card_int = rng.gen::<u8>() % 52;
                if taken[card_int as usize] {
                    continue;
                }
                taken[card_int as usize] = true;
                res.push(Card::from_int(card_int));
            }
        );
        res
    }

    pub fn new_random_9_card_game_with(card1: Card, card2: Card, card3: Card, card4: Card) -> NineCardDeal {
        let mut taken = [false; 52];
        let mut res = [Card::default(); 9];
        taken[card1.to_int() as usize] = true;
        taken[card2.to_int() as usize] = true;
        taken[card3.to_int() as usize] = true;
        taken[card4.to_int() as usize] = true;
        res[0] = card1;
        res[1] = card2;
        res[2] = card3;
        res[3] = card4;

        let mut count = 4;
        with_rng(|rng|
            while count < 9 {
                let card_int = rng.gen::<u8>() % 52;
                if taken[card_int as usize] {
                    continue;
                }
                taken[card_int as usize] = true;
                res[count] = Card::from_int(card_int);
                count += 1;
            }
        );
        res
    }

    pub fn new_random_9_card_game() -> NineCardDeal {
        let mut taken = [false; 52];
        let mut res = [Card::default(); 9];
        let mut count = 0;
        with_rng(|rng|
            while count < 9 {
                let card_int = rng.gen::<u8>() % 52;
                if taken[card_int as usize] {
                    continue;
                }
                taken[card_int as usize] = true;
                res[count] = Card::from_int(card_int);
                count += 1;
            }
        );
        res
    }

    fn serialise_int(card_int: u8) -> u8 {
        let suit = card_int / 13;
        let rank = card_int % 13;
        (suit << 4) | rank
    }

    pub fn get_n_more_cards(existing_cards: &Vec<Card>, n: usize) -> Vec<Card> {
        let mut taken = [false; 52];
        for card in existing_cards {
            taken[card.to_int() as usize] = true;
        }
        let mut res = Vec::with_capacity(n);
        with_rng(|rng|
            while res.len() < n {
                let card_int = (rng.gen::<u8>() % 52) as usize;
                if taken[card_int] {
                    continue;
                }
                taken[card_int] = true;
                res.push(Card::from_int(card_int as u8));
            }
        );
        res
    }

    pub fn get_one_more_card(existing_cards: &Vec<Card>) -> Card {
        let mut taken = [false; 52];
        for card in existing_cards {
            taken[card.to_int() as usize] = true;
        }
        with_rng(|rng|
            loop {
                let card_int = (rng.gen::<u8>() % 52) as usize;
                if taken[card_int] {
                    continue;
                }
                taken[card_int] = true;
                return Card::from_int(card_int as u8);
            }
        )
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
 
    // TODO - move these dealing methods to a different class...
    pub fn all_suited_combos_vs_hole_cards(hole_cards: (Card, Card), suit: Suit) -> impl Iterator<Item = ((Card, Card), (Card, Card))> {
        Self::all_suited_combos(suit)
            .map(move |(a, b)| (hole_cards, (a, b)))
    }

    pub fn all_suited_player_cards_combos(suit: Suit) -> impl Iterator<Item = ((Card, Card), (Card, Card))> {
        // Generate combinations of 4 ranks, then group them into pairs in different ways
        (0..13)
            .combinations(4) // All combinations of 4 ranks
            .flat_map(move |combo| {
                // Generate all unique pairings of the 4 ranks
                [
                    ((combo[0], combo[1]), (combo[2], combo[3])),
                    ((combo[0], combo[2]), (combo[1], combo[3])),
                    ((combo[0], combo[3]), (combo[1], combo[2])),
                ]
                .into_iter()
                .map(move |((a, b), (c, d))| {
                    (
                        (Card::new(suit, Rank::from_int(a)), Card::new(suit, Rank::from_int(b))),
                        (Card::new(suit, Rank::from_int(c)), Card::new(suit, Rank::from_int(d))),
                    )
                })
            })
    }
    
    pub fn all_suited_combos(suit: Suit) -> impl Iterator<Item = (Card, Card)> {
        (0..12).flat_map(move |first_rank| {
            ((first_rank+1)..13).map({
                let suit = suit;
                move |second_rank| {
                    (
                        Card::new(suit, Rank::from_int(first_rank)),
                        Card::new(suit, Rank::from_int(second_rank)),
                    )
                }
            })
        })
    }

    pub fn all_offsuit_combos(first_suit: Suit, second_suit: Suit) -> impl Iterator<Item = (Card, Card)> {
        (0..12).flat_map(move |first_rank| {
            ((first_rank+1)..13).map({
                let first_suit = first_suit;
                let second_suit = second_suit;
                move |second_rank| {
                    (
                        Card::new(first_suit, Rank::from_int(first_rank)),
                        Card::new(second_suit, Rank::from_int(second_rank)),
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
    use std::{collections::{HashMap, HashSet}, time::Instant};
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
    fn deck_to_int_roudtrip() {
        let mut seen = HashSet::new();
        for i in 0..52 {
            let card = Card::from_int(i);
            assert!(!seen.contains(&card));
            seen.insert(card.clone());
            assert_eq!(card.to_int(), i);
        }
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
    fn test_new_random_9_card() {
        for _ in 0..10_000 {
            let cards = Card::new_random_9_card_game();
            assert_eq!(cards.len(), 9);
            let mut seen = HashSet::new();
            for card in cards {
                assert!(!seen.contains(&card));
                seen.insert(card);
            }
        }
    }

    #[test]
    fn test_new_random_9_card_with_predicate() {
        let card1 = Card::new(Suit::Spades, Rank::Two);
        let card2 = Card::new(Suit::Hearts, Rank::Three);
        let card3 = Card::new(Suit::Diamonds, Rank::Four);
        let card4 = Card::new(Suit::Clubs, Rank::King);
        for _ in 0..10_000 {
            let cards = Card::new_random_9_card_game_with(card1, card2, card3, card4);
            assert_eq!(cards.len(), 9);
            let mut seen = HashSet::new();
            for card in cards {
                assert!(!seen.contains(&card));
                seen.insert(card);
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

    #[test]
    fn test_9_random_card_game_with_performance() {
        let existing_cards = Card::new_random_cards(4);
        let start = Instant::now();
        for _ in 0..100_000 {
            _ = Card::new_random_9_card_game_with(existing_cards[0], existing_cards[1], existing_cards[2], existing_cards[3]);
        }
        let duration = start.elapsed();
        assert!(duration.as_millis() < 500, "Performance test failed: took too long to generate cards");
    }

    #[test]
    fn test_get_one_more_card() {
        for _ in 0..10000 {
            let existing_cards = Card::new_random_cards(5);
            let new_card = Card::get_one_more_card(&existing_cards);
            assert!(!existing_cards.contains(&new_card));
        }
    }

    #[test]
    fn all_suited_whole_card_combos(){
        // are unique, there are 78 of them, they are all sorted, they are all the same suit
        let combos = Card::all_suited_combos(Suit::Spades).into_iter().collect::<Vec<_>>();
        assert_eq!(combos.len(), 12*13 /2);
        let mut seen = HashSet::new();
        for combo in combos {
            assert!(!seen.contains(&combo));
            seen.insert(combo);
            assert_eq!(combo.0.suit, Suit::Spades);
            assert_eq!(combo.1.suit, Suit::Spades);
            assert!(combo.0.rank.to_int() < combo.1.rank.to_int());
        }
    }

    #[test]
    fn all_suited_player_cards_combos(){
        // are unique, there are 2145 of them, they are all sorted, they are all the same suit
        let combos = Card::all_suited_player_cards_combos(Suit::Spades).collect::<Vec<_>>();
        assert_eq!(combos.len(), 2145);
        let mut seen = HashSet::new();
        for combo in combos {
            assert!(!seen.contains(&combo));
            seen.insert(combo);
            assert_eq!(combo.0.0.suit, Suit::Spades);
            assert_eq!(combo.0.1.suit, Suit::Spades);
            assert_eq!(combo.1.0.suit, Suit::Spades);
            assert_eq!(combo.1.1.suit, Suit::Spades);
            assert!(combo.0.0.rank.to_int() < combo.0.1.rank.to_int());
            assert!(combo.1.0.rank.to_int() < combo.1.1.rank.to_int());
        }
    }
}
