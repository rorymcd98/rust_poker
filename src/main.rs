mod models;

use crate::models::card::Card;

fn main() {
    let card = Card::new_random_card();
    println!("{:?}", card);
}
