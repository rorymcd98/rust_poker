use itertools::Itertools;
use lazy_static::lazy_static;
use rand::seq::SliceRandom;

use super::action_history::action_history::ActionHistory;
use super::action_history::game_abstraction::{convert_deal_into_abstraction, GameAbstraction, GameAbstractionSerialised};
use super::strategy::{strategy_branch::StrategyBranch, strategy_map::StrategyMap};
use crate::evaluate::evaluate_hand::EvaluateHand;
use crate::evaluate::evaluate_hand::HandEvaluator;
use crate::models::card::cards_string;
use crate::models::card::NineCardDeal;
use crate::models::card::Rank;
use crate::models::Card;
use crate::models::Player;
use crate::models::Suit;
use crate::thread_utils::with_rng;
use crate::traversal::action_history::action::Action;
use std::cell::{Cell, RefCell};
use std::thread;
use std::thread::JoinHandle;

lazy_static! {
    static ref EVALUATOR: EvaluateHand = EvaluateHand::new();
}

enum TerminalState {
    Showdown,
    Fold,
    RoundOver,
    None,
}

struct GameStateHelper {
    game_abstraction: GameAbstraction,
    traverser_pot: Cell<u8>,
    opponent_pot: Cell<u8>,
    pub cards: [Card; 9],
    cards_dealt: Cell<u8>,
    current_player: Cell<Player>,
    small_blind_player: Player,
    big_blind_player: Player,
    bets_this_round: Cell<u8>,
    winner: Option<Player>,
    checks_this_round: Cell<u8>,
}

pub const SMALL_BLIND: u8 = 1;
pub const BIG_BLIND: u8 = 2;
// TODO - move all the rules up here, then to an entirely separate file

impl GameStateHelper {
    pub fn new(
        nine_card_deal: NineCardDeal,
        small_blind_player: Player,
    ) -> GameStateHelper {
        GameStateHelper {
            game_abstraction: convert_deal_into_abstraction(
                nine_card_deal,
                small_blind_player,
            ),
            traverser_pot: Cell::new(if small_blind_player == Player::Traverser {
                SMALL_BLIND
            } else {
                BIG_BLIND
            }),
            opponent_pot: Cell::new(if small_blind_player == Player::Opponent {
                SMALL_BLIND
            } else {
                BIG_BLIND
            }),
            cards: nine_card_deal,
            cards_dealt: Cell::new(0),
            current_player: Cell::new(small_blind_player),
            small_blind_player,
            big_blind_player: small_blind_player.get_opposite(),
            bets_this_round: Cell::new(0),
            winner: EVALUATOR.evaluate_deal(nine_card_deal),
            checks_this_round: Cell::new(0),
        }
    }

    pub fn get_current_player(&self) -> Player {
        self.current_player.get()
    }

    pub fn switch_current_player(&self) {
        self.current_player
            .set(self.current_player.get().get_opposite());
    }

    pub fn set_current_player_to_small_blind(&self) {
        self.current_player.set(self.small_blind_player);
    }

    pub fn set_current_player_to_big_blind(&self) {
        self.current_player
            .set(self.big_blind_player);
    }

    pub fn get_flop(&self) -> [Card; 3] {
        [self.cards[4], self.cards[5], self.cards[6]]
    }

    pub fn is_preflop(&self) -> bool {
        self.cards_dealt.get() == 0
    }

    pub fn is_flop(&self) -> bool {
        self.cards_dealt.get() == 3
    }

    pub fn is_turn(&self) -> bool {
        self.cards_dealt.get() == 4
    }

    pub fn is_river(&self) -> bool {
        self.cards_dealt.get() == 5
    }

    pub fn get_num_available_actions(&self) -> usize {
        if self.get_current_player_pot() == 1 {
            return 3; // we're preflop
        }
        match self.bets_this_round.get() {
            0 => 2,
            4 => 2,
            _ => 3,
        }
    }

    pub fn get_current_player_cards(&self) -> [Card; 2] {
        match self.current_player.get() {
            Player::Traverser => [self.cards[0], self.cards[1]],
            Player::Opponent => [self.cards[2], self.cards[3]],
        }
    }

    pub fn get_non_current_player_cards(&self) -> [Card; 2] {
        match self.current_player.get() {
            Player::Opponent => [self.cards[0], self.cards[1]],
            Player::Traverser => [self.cards[2], self.cards[3]],
        }
    }

    pub fn serialise_history_with_current_player(&self) -> GameAbstractionSerialised {
        self.game_abstraction.get_abstraction(
            (self.cards_dealt.get()).saturating_sub(2) as usize,
            self.get_current_player_pot(),
            self.bets_this_round.get(),
            self.current_player.get(),
        )
    }

    pub fn check_round_terminal(&self) -> TerminalState {
        if self.checks_this_round.get() == 2 {
            return TerminalState::Showdown;
        }

        let terminal_state = if self.opponent_pot.get() == self.traverser_pot.get() {
            // If the pots are equal and there have been bets then this is a showdown / round over
            if self.bets_this_round.get() > 0 {
                TerminalState::Showdown
            } else {
                // Otherwise its the first action
                TerminalState::None
            }
        } else {
            if self.get_current_player_pot() < self.get_non_current_player_pot() {
                // If the pots are unequal and we have less in the pot, then it's our turn 
                TerminalState::None
            } else {
                // Otherwise the opponent just folded
                TerminalState::Fold
            }
        };
        if !self.is_river() {
            match terminal_state {
                TerminalState::Fold => TerminalState::Fold,
                TerminalState::Showdown => TerminalState::RoundOver,
                _ => TerminalState::None,
            }
        } else {
            terminal_state
        }
    }

    // If we're at showdown, we lose our pot, or gain the opponent's pot
    fn evaluate_showdown(&self) -> f32 {
        // validate_history(&self.action_history.borrow().history);
        match self.winner {
            Some(Player::Traverser) => self.opponent_pot.get() as f32,
            Some(Player::Opponent) => -(self.traverser_pot.get() as f32),
            None => 0.0,
        }
    }

    // If we're at fold, the other player has just folded, so the traverser should get their pot
    fn evaluate_fold(&self) -> f32 {
        // validate_history(&self.action_history.borrow().history);
        match self.current_player.get() {
            Player::Traverser => self.opponent_pot.get() as f32,
            Player::Opponent => -(self.traverser_pot.get() as f32),
        }
    }

    pub fn bet(&self) {
        self.bets_this_round.set(self.bets_this_round.get() + 1);
        let raise = if self.is_preflop() { BIG_BLIND } else { BIG_BLIND * 2 };
        match self.current_player.get() {
            Player::Traverser => {
                self.traverser_pot.set(self.opponent_pot.get() + raise);
            }
            Player::Opponent => {
                self.opponent_pot.set(self.traverser_pot.get() + raise);
            }
        }
    }

    pub fn get_current_player_pot(&self) -> u8 {
        match self.current_player.get() {
            Player::Traverser => self.traverser_pot.get(),
            Player::Opponent => self.opponent_pot.get(),
        }
    }

    pub fn get_non_current_player_pot(&self) -> u8 {
        match self.current_player.get() {
            Player::Traverser => self.opponent_pot.get(),
            Player::Opponent => self.traverser_pot.get(),
        }
    }

    fn call_or_bet(&self) {
        match self.bets_this_round.get() {
            0 => self.bet(), // Handles the preflop edge case, and the start of betting rounds
            _ => self.call(),
        }
    }

    pub fn call(&self) {
        if self.get_current_player_pot() == 2 {
            self.checkfold(); // Pseudo check 
        }
        match self.current_player.get() {
            Player::Traverser => {
                self.traverser_pot.set(self.opponent_pot.get());
            }
            Player::Opponent => {
                self.opponent_pot.set(self.traverser_pot.get());
            }
        };
    }

    pub fn checkfold(&self) {
        if self.bets_this_round.get() == 0 {
            self.checks_this_round.set(self.checks_this_round.get() + 1);
        }
    }

    // Set the state back to a previous action
    pub fn undo(&self, acting_player: Player, previous_pot: u8, previous_bets: u8, previous_checks: u8) {
        match acting_player {
            Player::Traverser => {
                self.traverser_pot.set(previous_pot);
            }
            Player::Opponent => {
                self.opponent_pot.set(previous_pot);
            }
        };
        self.bets_this_round.set(previous_bets);
        self.current_player.set(acting_player);
        self.checks_this_round.set(previous_checks); // TODO - think of a more optimal way to determine checks
    }

    // implement deal and undeal
    pub fn deal(&self) {
        self.cards_dealt.set(self.cards_dealt.get() + 1);
        self.checks_this_round.set(0);
        self.bets_this_round.set(0);
        self.set_current_player_to_big_blind();
    }

    pub fn undeal(&self, previous_bets: u8, previous_player: Player, previous_checks: u8) {
        self.cards_dealt.set(self.cards_dealt.get() - 1);
        self.bets_this_round.set(previous_bets);
        self.current_player.set(previous_player);
        self.checks_this_round.set(previous_checks);
    }

    pub fn deal_flop(&self) {
        self.cards_dealt.set(3);
        self.bets_this_round.set(0);
        self.checks_this_round.set(0);
        self.set_current_player_to_big_blind();
    }

    pub fn undeal_flop(&self, previous_bets: u8, previous_player: Player, previous_checks: u8) {
        self.cards_dealt.set(0);
        self.bets_this_round.set(previous_bets);
        self.current_player.set(previous_player);
        self.checks_this_round.set(previous_checks);
    }
}

const TRAIN_ITERATIONS: usize = 100_0000;
const PLAY_ITERATIONS: usize = 1000;
const SUBPLAY_ITERATIONS: usize = 1000;
const NUM_THREADS: usize = 1;

pub struct TreeTraverser {
    strategy_map: StrategyMap,
    iterations: usize,
}

impl TreeTraverser {
    pub fn new(iterations: usize) -> TreeTraverser {
        TreeTraverser {
            strategy_map: StrategyMap::new(),
            iterations,
        }
    }

    fn spawn_thread_work(
        combo_chunk: Vec<((Card, Card), (Card, Card))>,
    ) -> JoinHandle<(
        f32,
        (Card, Card),
        f32,
        (Card, Card),
        f32,
        (Card, Card),
        f32,
        (Card, Card),
    )> {
        thread::spawn(move || {
            println!("{}", combo_chunk.len());
            let highest_so_far_bb = -10.0;
            let highest_combo_bb = (
                Card::new(Suit::Spades, Rank::Two),
                Card::new(Suit::Spades, Rank::Two),
            );

            let lowest_so_far_bb = 10.0;
            let lowest_combo_bb = (
                Card::new(Suit::Spades, Rank::Two),
                Card::new(Suit::Spades, Rank::Two),
            );

            let highest_so_far_sb = -10.0;
            let highest_combo_sb = (
                Card::new(Suit::Spades, Rank::Two),
                Card::new(Suit::Spades, Rank::Two),
            );

            let lowest_so_far_sb = 10.0;
            let lowest_combo_sb = (
                Card::new(Suit::Spades, Rank::Two),
                Card::new(Suit::Spades, Rank::Two),
            );

            let mut strategy_branch = StrategyBranch::new();

            let players = [Player::Traverser, Player::Opponent];
            for iteration in 1..=TRAIN_ITERATIONS {
                // Iter must start at 1
                // if iteration % 1000 == 0 {
                //     strategy_branch.print_stats();
                // }
                for card_combo in combo_chunk.clone() {
                    for player in &players {
                        // Alternate between who is small blind
                        debug_assert!(
                            card_combo.0 .0.serialise() <= card_combo.0 .1.serialise(),
                            "Card combo not sorted {:?}",
                            card_combo
                        );
                        debug_assert!(
                            card_combo.1 .0.serialise() <= card_combo.1 .1.serialise(),
                            "Card combo not sorted {:?}",
                            card_combo
                        );

                        let deal = match player {
                            Player::Traverser => Card::new_random_9_card_game_with(
                                card_combo.0 .0,
                                card_combo.0 .1,
                                card_combo.1 .0,
                                card_combo.1 .1,
                            ),
                            Player::Opponent => Card::new_random_9_card_game_with(
                                card_combo.1 .0,
                                card_combo.1 .1,
                                card_combo.0 .0,
                                card_combo.0 .1,
                            ),
                        };

                        let game_state =
                            GameStateHelper::new(deal, *player);
                        let mut branch_traverser =
                            BranchTraverser::new(strategy_branch, game_state, iteration);

                        let result = branch_traverser.begin_traversal(false);
                        strategy_branch = result.1;
                    }
                }
                // println!("Finished training combo {:?}", card_combo_print);
            }
            let average_utility_sb_combo1 = 0.0;
            let average_utility_bb_combo2 = 0.0;

            // first the Traverser plays with combo 1 in SB position
            // then the Traverser plays with combo 2 in BB position
            // for iteration in 1..PLAY_ITERATIONS { // Iter must start at 1
            //     for card_combo in combo_chunk.clone() {
            //         for player in &players { // Alternate between who is small blind
            //             let action_history = &mut ActionHistory::new(vec![]); // TODO - optimise this + could use Default Default

            //             match player {
            //                 Player::Traverser => { // This currently doesn't matter as we just overwrite the history
            //                     action_history.history.push(Action::Deal(card_combo.0.0.clone()));
            //                     action_history.history.push(Action::Deal(card_combo.0.1.clone()));
            //                 },
            //                 Player::Opponent => {
            //                     action_history.history.push(Action::Deal(card_combo.1.0.clone()));
            //                     action_history.history.push(Action::Deal(card_combo.1.1.clone()));
            //                 }
            //             }

            //             let deal = match player {
            //                 Player::Traverser => {
            //                     Card::new_random_9_card_game_with(card_combo.0.0, card_combo.0.1, card_combo.1.0, card_combo.1.1)
            //                 },
            //                 Player::Opponent => {
            //                     Card::new_random_9_card_game_with( card_combo.1.0, card_combo.1.1, card_combo.0.0, card_combo.0.1)
            //                 }
            //             };

            //             let game_state = GameStateHelper::new(action_history.clone(), deal, player.clone());
            //             let mut branch_traverser = BranchTraverser::new(strategy_branch, game_state, iteration);

            //             let result= branch_traverser.begin_traversal(true);
            //             strategy_branch = result.1;
            //             match player {
            //                 Player::Traverser => {
            //                     average_utility_sb_combo1 += result.0;
            //                 },
            //                 Player::Opponent => {
            //                     average_utility_bb_combo2 += result.0;
            //                 },
            //             }
            //         }
            //     }
            // }

            // average_utility_sb_combo1 /= TRAIN_ITERATIONS as f32;
            // average_utility_bb_combo2 /= TRAIN_ITERATIONS as f32;

            // if average_utility_bb_combo2 > highest_so_far_bb {
            //     highest_so_far_bb = average_utility_bb_combo2;
            //     highest_combo_bb = card_combo.1;
            // }
            // if average_utility_bb_combo2 < lowest_so_far_bb {
            //     lowest_so_far_bb = average_utility_bb_combo2;
            //     lowest_combo_bb = card_combo.1;
            // }
            // if average_utility_sb_combo1 > highest_so_far_sb {
            //     highest_so_far_sb = average_utility_sb_combo1;
            //     highest_combo_sb = card_combo.0;
            // }
            // if average_utility_sb_combo1 < lowest_so_far_sb {
            //     lowest_so_far_sb = average_utility_sb_combo1;
            //     lowest_combo_sb = card_combo.0;
            // }
            strategy_branch.print_stats();
            (
                highest_so_far_bb,
                highest_combo_bb,
                lowest_so_far_bb,
                lowest_combo_bb,
                highest_so_far_sb,
                highest_combo_sb,
                lowest_so_far_sb,
                lowest_combo_sb,
            )
        })
    }

    pub fn begin_tree_traversal(self) {
        let mut highest_so_far_bb = -10.0;
        let mut highest_combo_bb = (
            Card::new(Suit::Spades, Rank::Two),
            Card::new(Suit::Spades, Rank::Two),
        );

        let mut lowest_so_far_bb = 10.0;
        let mut lowest_combo_bb = (
            Card::new(Suit::Spades, Rank::Two),
            Card::new(Suit::Spades, Rank::Two),
        );

        let mut highest_so_far_sb = -10.0;
        let mut highest_combo_sb = (
            Card::new(Suit::Spades, Rank::Two),
            Card::new(Suit::Spades, Rank::Two),
        );

        let mut lowest_so_far_sb = 10.0;
        let mut lowest_combo_sb = (
            Card::new(Suit::Spades, Rank::Two),
            Card::new(Suit::Spades, Rank::Two),
        );

        // let card_combos = Card::all_suited_player_cards_combos(Suit::Spades).collect_vec().iter()
        // let card_combos = Card::all_suited_combos_vs_hole_cards(
        //     (Card::new(Suit::Spades, Rank::Nine), Card::new(Suit::Spades, Rank::Ten)),
        //     // (Card::new(Suit::Spades, Rank::Ace), Card::new(Suit::Clubs, Rank::Ace))
        //     Suit::Spades,
        // ).take(30)
        // .flat_map(|(a, b)| vec![(a.clone(), b.clone()), (b, a)]) // Mirror each tuple
        // .collect_vec();

        let cards = with_rng(|rng| {
            let mut x = Card::all_offsuit_combos(Suit::Spades, Suit::Clubs).collect_vec();
            x.shuffle(rng);
            x
        });
        // let ten_combos = cards.into_iter().take(2).combinations(2).map(|x| (x[0], x[1])).collect_vec();
        let ten_combos = vec![
            (
                (
                    Card::new(Suit::Spades, Rank::Ace),
                    Card::new(Suit::Clubs, Rank::Ace),
                ),
                (
                    Card::new(Suit::Spades, Rank::Two),
                    Card::new(Suit::Clubs, Rank::Seven),
                ),
            ),
            (
                (
                    Card::new(Suit::Spades, Rank::Two),
                    Card::new(Suit::Clubs, Rank::Seven),
                ),
                (
                    Card::new(Suit::Spades, Rank::Ace),
                    Card::new(Suit::Clubs, Rank::Ace),
                ),
            ),
        ]; //5s 6c vs 2s Tc

        // let chunk_size = card_combos.len() / NUM_THREADS;
        // let card_combos_chunks: Vec<_> = card_combos.chunks(chunk_size).collect();

        let mut handles = vec![];
        // for chunk in card_combos_chunks.into_iter() {
        handles.push(Self::spawn_thread_work(ten_combos));
        // }

        for handle in handles {
            let res = handle.join().unwrap();
            if res.0 > highest_so_far_bb {
                highest_so_far_bb = res.0;
                highest_combo_bb = res.1;
            }
            if res.2 < lowest_so_far_bb {
                lowest_so_far_bb = res.2;
                lowest_combo_bb = res.3;
            }

            if res.4 > highest_so_far_sb {
                highest_so_far_sb = res.4;
                highest_combo_sb = res.5;
            }

            if res.6 < lowest_so_far_sb {
                lowest_so_far_sb = res.6;
                lowest_combo_sb = res.7;
            }
        }

        println!(
            "Highest average utility: {:?} (bb), with card combo {:?}",
            highest_so_far_bb, highest_combo_bb
        );
        println!(
            "Lowest average utility: {:?} (bb), with card combo {:?}",
            lowest_so_far_bb, lowest_combo_bb
        );

        println!(
            "Highest average utility: {:?} (sb), with card combo {:?}",
            highest_so_far_sb, highest_combo_sb
        );
        println!(
            "Lowest average utility: {:?} (sb), with card combo {:?}",
            lowest_so_far_sb, lowest_combo_sb
        );
    }
}

struct BranchTraverser {
    strategy_branch: RefCell<StrategyBranch>,
    iteration: usize,
    game_state: GameStateHelper,
}

impl BranchTraverser {
    pub fn new(
        strategy_branch: StrategyBranch,
        game_state: GameStateHelper,
        iteration: usize,
    ) -> BranchTraverser {
        BranchTraverser {
            strategy_branch: RefCell::new(strategy_branch),
            iteration,
            game_state,
        }
    }

    pub fn begin_traversal(&mut self, playing: bool) -> (f32, StrategyBranch) {
        self.game_state.set_current_player_to_small_blind();
        let utility = self.traverse_action(playing);
        (utility, self.strategy_branch.take())
    }

    fn traverse_action(&self, playing: bool) -> f32 {
        match self.game_state.check_round_terminal() {
            TerminalState::Showdown => return self.game_state.evaluate_showdown(),
            TerminalState::Fold => return self.game_state.evaluate_fold(),
            TerminalState::RoundOver => {
                if self.game_state.is_preflop() {
                    return self.traverse_flop(playing);
                }
                return self.traverse_deal(playing);
            }
            TerminalState::None => (),
        };

        let info_node_key = self.game_state.serialise_history_with_current_player();
        let num_available_actions = self.game_state.get_num_available_actions();

        let pot_before_action = self.game_state.get_current_player_pot();
        let bets_before_action = self.game_state.bets_this_round.get();
        let previous_player = self.game_state.current_player.get();
        let checks_before = self.game_state.checks_this_round.get();

        if self.game_state.current_player.get().is_opponent() {
            let sampled_action= {
                let mut strategy_branch = self.strategy_branch.borrow_mut();
                let strategy = strategy_branch
                    .get_or_create_strategy(info_node_key.clone(), num_available_actions);
                let sample = strategy.sample_strategy(playing);
                sample
            };

            match sampled_action {
                0 => (), // checkfold
                1 => self.game_state.call_or_bet(),
                2 => self.game_state.bet(),
                _ => panic!("Invalid action"),
            };

            self.game_state.switch_current_player();
            let utility = self.traverse_action(playing);
            self.game_state
                .undo(previous_player, pot_before_action, bets_before_action, checks_before);
            utility
        } else {
            let mut utility = 0.0;
            let mut utilities = vec![0.0; num_available_actions];
            let current_strategy = {
                let mut strategy_branch = self.strategy_branch.borrow_mut();
                let strategy = strategy_branch
                    .get_or_create_strategy(info_node_key.clone(), num_available_actions);
                strategy.get_strategy(playing)
            };

            for action in 0..num_available_actions {
                let pot_before_action = self.game_state.get_current_player_pot();
                match action {
                    0 => self.game_state.checkfold(), 
                    1 => self.game_state.call_or_bet(), // TODO - change this call-or-bet logic
                    2 => self.game_state.bet(),
                    _ => panic!("Invalid action"),
                };
                self.game_state.switch_current_player();
                utilities[action] = self.traverse_action(playing) * current_strategy[action]; // WHY DID MOVING THIS BELOW MATTER?
                self.game_state
                    .undo(previous_player, pot_before_action, bets_before_action, checks_before);
            }

            for action in 0..num_available_actions {
                utility += utilities[action];
            }

            if !playing {
                let mut strategy_branch = self.strategy_branch.borrow_mut();
                let strategy = strategy_branch.get_strategy(info_node_key.clone());
                strategy.update_strategy(utility, utilities.clone(), self.iteration);
                if self.iteration % 1000 == 0 && self.game_state.get_current_player_pot() == 2   && self.game_state.cards[0].rank == Rank::Ace {
                    println!(
                        "Strategy for {} vs {} is {:?}, with utilities {:?}, bets this round {}",
                        cards_string(&self.game_state.get_current_player_cards()),
                        cards_string(&self.game_state.get_non_current_player_cards()),
                        strategy.regrets,
                        utilities.clone(),
                        self.game_state.bets_this_round.get()
                    );
                }
            }

            utility
        }
    }

    fn traverse_flop(&self, playing: bool) -> f32 {
        let previous_player = self.game_state.current_player.get();
        let previous_bets = self.game_state.bets_this_round.get();
        let check_before = self.game_state.checks_this_round.get();
        self.game_state.deal_flop();
        let utility = self.traverse_action(playing);
        self.game_state.undeal_flop(previous_bets, previous_player, check_before);
        utility
    }

    fn traverse_deal(&self, playing: bool) -> f32 {
        let previous_player = self.game_state.current_player.get();
        let previous_bets = self.game_state.bets_this_round.get();
        let checks_before = self.game_state.checks_this_round.get();
        self.game_state.deal();
        let utility = self.traverse_action(playing);
        self.game_state.undeal(previous_bets, previous_player, checks_before);
        utility
    }
}
