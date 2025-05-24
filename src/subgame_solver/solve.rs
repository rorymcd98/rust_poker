use core::panic;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::{u16, vec};

use crate::config::{BIG_BLIND, BLUEPRINT_FOLDER};
use dashmap::DashMap;
use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};
use itertools::Itertools;
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};

use crate::evaluate::evaluate_hand::HandEvaluator;
use crate::models::card::{NineCardDeal, Rank};
use crate::models::{Card, Player, Suit};
use crate::traversal::action_history::action::{Action, DEFAULT_ACTION_COUNT};
use crate::traversal::action_history::card_round_abstraction::CardRoundAbstractionSerialised;
use crate::traversal::action_history::game_abstraction::{
    get_current_abstraction, GameAbstraction, GameAbstractionSerialised,
};
use crate::traversal::game_state::game_state_helper::{GameStateHelper, EVALUATOR};
use crate::traversal::game_state::terminal_state::TerminalState;
use crate::traversal::strategy::play_strategy::PlayStrategy;
use crate::traversal::strategy::strategy_branch::{StrategyBranch, StrategyHubKey};
use crate::traversal::strategy::strategy_hub::deserialise_strategy_hub;
use crate::traversal::strategy::strategy_trait::Strategy;

#[derive(Clone)]
struct GameTreePath {
    pub abstraction: GameAbstraction,
    pub evaluation: Option<Player>,
    pub strategy_hub_key: StrategyHubKey,
    pub weight: usize,
}

type GameTreePathKey = (
    StrategyHubKey,
    u8,
    [CardRoundAbstractionSerialised; 4],
    [CardRoundAbstractionSerialised; 4],
);

impl GameTreePath {
    pub fn get_game_path_key(&self) -> GameTreePathKey {
        (
            self.strategy_hub_key.clone(),
            match self.evaluation {
                None => 0,
                Some(Player::Traverser) => 1,
                Some(Player::Opponent) => 2,
            },
            self.abstraction.traverser_round_abstractions.clone(),
            self.abstraction.opponent_round_abstractions.clone(),
        )
    }
}

struct GameStateFromActions {
    partial_deal: NineCardDeal,
    traverser_pot: u8,
    opponent_pot: u8,
    cards_dealt: u8,
    small_blind_player: Player,
    big_blind_player: Player,
    bets_this_round: u8,
    checks_this_round: u8,
    current_player: Player,
}

// Reach MaxMargin
// Generate a game state
// Look at the preceeding node
// Generate all the game state that could have lead to this node (169 states)
// For each of these histories calculate the total gift in this manner:
// 1. Calculate the CBV which can be done using vanilla CFR, multiplying the strategy by the utility for each action
// 2. Calculate the gift by finding the max of CBV(I) - CBV(I, a) for all a in A(I)

pub fn solve_cbr_utilties2() {
    let strategy_hub = deserialise_strategy_hub::<PlayStrategy>(BLUEPRINT_FOLDER).unwrap();
    let strategy_map = strategy_hub
        .into_iter()
        .collect::<DashMap<StrategyHubKey, StrategyBranch<_>>>();

    // Look for the gifts give on the Bet node
    // let action_history = &vec![
    //     Action::Bet,  Action::Bet, Action::Bet,Action::Bet, Action::Call,
    //     Action::Deal(Card::new(Suit::Clubs, Rank::Three)), Action::Deal(Card::new(Suit::Spades, Rank::Nine)), Action::Deal(Card::new(Suit::Hearts, Rank::Queen)),
    //     Action::CheckFold, Action::Bet, Action::Call,
    //     Action::Deal(Card::new(Suit::Diamonds, Rank::Five)),
    //     Action::CheckFold, Action::Bet, Action::Call,
    //     Action::Deal(Card::new(Suit::Clubs, Rank::Six)),
    //     Action::Bet, Action::Bet, Action::Bet , Action::Bet, Action::Call
    // ];

    let action_history = &vec![
        Action::Bet,
        Action::Bet,
        Action::Bet,
        Action::Bet,
        Action::Call,
        Action::Deal(Card::new(Suit::Clubs, Rank::Ace)),
        Action::Deal(Card::new(Suit::Spades, Rank::Ace)),
        Action::Deal(Card::new(Suit::Hearts, Rank::Ace)),
        Action::CheckFold,
        Action::Bet,
        Action::Call,
        Action::Deal(Card::new(Suit::Diamonds, Rank::Ace)),
        Action::CheckFold,
        Action::Bet,
        Action::Call,
        // Action::Deal(Card::new(Suit::Clubs, Rank::Queen)),
        // Action::Bet, Action::Bet, Action::Bet, Action::Bet,
    ];

    let sb_player = Player::Opponent;
    let game_state = &mut convert_actions_to_game_state(action_history, sb_player);

    let mut tree_builder = CbvSubTree {
        strategy_map: &strategy_map,
        action_history: action_history.clone(),
        game_state: game_state.clone(),
        subtrees: HashMap::new(),
        dealt_board_cards: Vec::new(),
        reaches: HoleCardReaches::new(&Vec::new(), &Vec::new()),
        hole_card_payoffs: None,
        solving_state: SolvingState::Exploring,
    };

    tree_builder.explore_root();
}

#[derive(Clone, PartialEq)]
struct HoleCardReaches {
    traverser_reaches: HashMap<(Card, Card), f64>,
    opponent_reaches: HashMap<(Card, Card), f64>,
}

impl HoleCardReaches {
    pub fn new(
        traverser_hole_cards: &Vec<(Card, Card)>,
        opponent_hole_cards: &Vec<(Card, Card)>,
    ) -> Self {
        let mut traverser_reaches = HashMap::new();
        let mut opponent_reaches = HashMap::new();

        let traverser_init = 1.0 / traverser_hole_cards.len() as f64;
        let opponent_init = 1.0 / opponent_hole_cards.len() as f64;

        for card in traverser_hole_cards {
            traverser_reaches.insert(*card, traverser_init);
        }
        for card in opponent_hole_cards {
            opponent_reaches.insert(*card, opponent_init);
        }

        Self {
            traverser_reaches,
            opponent_reaches,
        }
    }

    pub fn get_reach(&self, player: Player, card: &(Card, Card)) -> f64 {
        match player {
            Player::Traverser => *self.traverser_reaches.get(card).unwrap_or(&0.0),
            Player::Opponent => *self.opponent_reaches.get(card).unwrap_or(&0.0),
        }
    }

    pub fn update(&mut self, player: Player, hole_cards: &(Card, Card), reach_multiplier: f64) {
        match player {
            Player::Traverser => {
                self.traverser_reaches
                    .entry(*hole_cards)
                    .and_modify(|e| *e *= reach_multiplier);
            }
            Player::Opponent => {
                self.opponent_reaches
                    .entry(*hole_cards)
                    .and_modify(|e| *e *= reach_multiplier);
            }
        }
    }

    pub fn get_hole_cards(&self, player: Player) -> Vec<&(Card, Card)> {
        match player {
            Player::Traverser => self.traverser_reaches.keys().collect(),
            Player::Opponent => self.opponent_reaches.keys().collect(),
        }
    }

    pub fn clone_non_zero(&self) -> Self {
        let traverser_reaches = self
            .traverser_reaches
            .iter()
            .filter(|(_, reach)| **reach != 0.0)
            .map(|(card, reach)| (*card, *reach))
            .collect();
        let opponent_reaches = self
            .opponent_reaches
            .iter()
            .filter(|(_, reach)| **reach != 0.0)
            .map(|(card, reach)| (*card, *reach))
            .collect();
        Self {
            traverser_reaches,
            opponent_reaches,
        }
    }

    pub fn clone_without_card(&self, card: &Card, bulk_multiplier: f64) -> Self {
        let traverser_reaches = self
            .traverser_reaches
            .iter()
            .filter(|(hole_cards, _)| hole_cards.0 != *card && hole_cards.1 != *card)
            .map(|(card, reach)| (*card, *reach * bulk_multiplier))
            .collect();
        let opponent_reaches = self
            .opponent_reaches
            .iter()
            .filter(|(hole_cards, _)| hole_cards.0 != *card && hole_cards.1 != *card)
            .map(|(card, reach)| (*card, *reach * bulk_multiplier))
            .collect();
        Self {
            traverser_reaches,
            opponent_reaches,
        }
    }
}

#[derive(Clone, Default)]
struct HoleCardPayoffs {
    pub traverser_payoffs: HashMap<(Card, Card), f64>,
}

impl HoleCardPayoffs {
    pub fn add_subpayoff(&mut self, hole_card_subpayoffs: &HoleCardPayoffs) {
        for (hole_card, payoff) in hole_card_subpayoffs.traverser_payoffs.iter() {
            self.traverser_payoffs
                .entry(*hole_card)
                .and_modify(|e| *e += *payoff)
                .or_insert(*payoff);
        }
    }

    pub fn max_subpayoff(&mut self, hole_card_subpayoffs: &HoleCardPayoffs) {
        // Find the best action for each hand
        for (hole_card, payoff) in hole_card_subpayoffs.traverser_payoffs.iter() {
            self.traverser_payoffs
                .entry(*hole_card)
                .and_modify(|e| *e = e.max(*payoff))
                .or_insert(*payoff);
        }
    }

    pub fn fold_payoffs(fold_utility: f64, hole_card_reaches: &HoleCardReaches) -> HoleCardPayoffs {
        let mut payoffs = HoleCardPayoffs::default();
        for (hole_card, reach) in hole_card_reaches.traverser_reaches.iter() {
            payoffs
                .traverser_payoffs
                .insert(*hole_card, fold_utility * *reach);
        }
        payoffs
    }

    fn normalise(&mut self, total_reach: f64) {
        for (_, payoff) in self.traverser_payoffs.iter_mut() {
            *payoff /= total_reach;
        }
    }

    // TODO - Use sorting to make this more efficient
    // TODO - check if this is the correct calculation
    // Calculate total opponent weight for normalization
    pub fn showdown_payoffs(
        player_pot: u8,
        traverser_hand_rankings: HashMap<(Card, Card), u16>,
        opponent_hand_rankings: HashMap<(Card, Card), u16>,
        hole_card_reaches: &HoleCardReaches,
    ) -> HoleCardPayoffs {
        // Calculate total opponent weight for normalization
        let mut payoffs = HoleCardPayoffs::default();
        // let total_opponent_reach: f64 = hole_card_reaches.opponent_reaches.values().sum();

        // For each traverser hand
        for (traverser_hole_cards, traverser_reach) in hole_card_reaches.traverser_reaches.iter() {
            if *traverser_reach <= 0.0 {
                continue;
            }

            let mut ev = 0.0;
            let mut valid_opponent_reach = 0.0;

            // Calculate EV against opponent's distribution
            for (opponent_hole_cards, opponent_reach) in hole_card_reaches.opponent_reaches.iter() {
                // Skip conflicts and zero reaches
                if *opponent_reach <= 0.0 {
                    continue; // Skip zero reach hands
                }

                // Skip if cards conflict
                if (traverser_hole_cards.0 == opponent_hole_cards.0)
                    || (traverser_hole_cards.0 == opponent_hole_cards.1)
                    || (traverser_hole_cards.1 == opponent_hole_cards.0)
                    || (traverser_hole_cards.1 == opponent_hole_cards.1)
                {
                    continue;
                }

                let traverser_rank = traverser_hand_rankings.get(traverser_hole_cards).unwrap();
                let opponent_rank = opponent_hand_rankings.get(opponent_hole_cards).unwrap();

                let utility = match traverser_rank.cmp(opponent_rank) {
                    std::cmp::Ordering::Less => -(player_pot as f64),
                    std::cmp::Ordering::Equal => 0.0,
                    std::cmp::Ordering::Greater => player_pot as f64,
                };

                ev += utility * opponent_reach;
                valid_opponent_reach += opponent_reach;
            }

            // Normalize by valid opponent reach
            if valid_opponent_reach > 0.0 {
                ev /= valid_opponent_reach;
            }

            payoffs
                .traverser_payoffs
                .insert(*traverser_hole_cards, ev * traverser_reach);
        }
        payoffs
    }
}

impl Display for HoleCardPayoffs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (hole_card, payoff) in &self.traverser_payoffs {
            writeln!(f, "{:?} {}", hole_card, payoff)?;
        }
        Ok(())
    }
}

enum SolvingState {
    Exploring, // Building sub trees
    Solving,   // Calculating hole card payoffs in parallel
    Resolving, // Re-exploring the root up until the solved subtrees
}

type CbvSubeTreeReturn = HashMap<(Card, Card), f64>;

struct CbvSubTree<'a> {
    strategy_map: &'a DashMap<StrategyHubKey, StrategyBranch<PlayStrategy>>,
    action_history: Vec<Action>,
    game_state: GameStateHelper,
    subtrees: HashMap<Vec<Action>, CbvSubTree<'a>>,
    dealt_board_cards: Vec<Card>,
    reaches: HoleCardReaches, // can possibly remove this and just store it in sub_trees
    solving_state: SolvingState,
    hole_card_payoffs: Option<HoleCardPayoffs>,
}

impl<'a> CbvSubTree<'a> {
    pub fn explore_root(&mut self) -> HashMap<(Card, Card), f64> {
        // TODO - move Vec<Action> to the struct
        let dealt_board_cards = self
            .action_history
            .iter()
            .filter_map(|action| {
                if let Action::Deal(card) = action {
                    Some(*card)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let remaining_cards = (0..52)
            .map(Card::from_int)
            .filter(|card| !dealt_board_cards.contains(card))
            .collect::<Vec<_>>();

        let all_hole_cards = remaining_cards
            .iter()
            .combinations(2)
            .map(|cards| {
                if cards[0] <= cards[1] {
                    (*cards[0], *cards[1])
                } else {
                    (*cards[1], *cards[0])
                }
            })
            .collect::<HashSet<_>>();
        let all_hole_cards = all_hole_cards.into_iter().collect::<Vec<_>>();

        let action_history = self.action_history.clone();

        for hc in all_hole_cards.clone() {
            assert!(hc.0 <= hc.1, "Hole cards should be sorted");
        }

        // let initial_reaches = HoleCardReaches::new(&all_hole_cards.clone(), &all_hole_cards.clone());

        let initial_reaches = self.calculate_initial_reaches(
            &action_history,
            &all_hole_cards.clone(),
            &all_hole_cards.clone(),
        );
        // let initial_reaches = HoleCardReaches::new(&all_hole_cards, &all_hole_cards);
        // for hc_reach in initial_reaches.traverser_reaches.iter() {
        //     println!("{:?} {}", hc_reach.0, hc_reach.1);
        // }
        // panic!("Stop here");
        self.dealt_board_cards = dealt_board_cards;

        let initial_action_history = action_history.clone();

        self.explore_sub_trees(&initial_reaches, 7); // need to weight by reaches

        debug_assert!(
            self.action_history == initial_action_history,
            "Action history should not have changed"
        );

        // Do an operation here to get the CBV for the root node

        // for (action_history, subtree) in self.sub_trees.iter() {
        //     println!("{:?}", action_history);
        // }
        println!("Subtrees {}", self.subtrees.len());

        let pb = ProgressBar::new(self.subtrees.len() as u64);
        let total_work = self.subtrees.len();
        let progress_bar = ProgressBar::new(total_work as u64);
        progress_bar.set_style(
            ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {percent}% ({pos}/{len})")
            .expect("Failed to set progress bar style")
            .progress_chars("#>-"),
        );

        let mut sub_trees: Vec<&mut CbvSubTree<'a>> = self
            .subtrees
            .iter_mut()
            .map(|(_, tree)| tree)
            .collect::<Vec<_>>();

        sub_trees
            .par_iter_mut()
            .progress_with(pb)
            .for_each(|subtree| {
                subtree.compute_payoffs(&subtree.reaches.clone_non_zero(), 100000);
            });

        self.solving_state = SolvingState::Resolving;
        let mut res = self.explore_sub_trees(&initial_reaches, 7);

        let mut traverser_payoffs = res.traverser_payoffs;
        let mut reaches_sum = 0.0;
        for (hole_card, _) in traverser_payoffs.iter_mut() {
            let initial_reach = initial_reaches.get_reach(Player::Traverser, hole_card);
            // println!("{:?} {}, {}, {}", hole_card, initial_reach, payoff, *payoff / initial_reach);
            reaches_sum += initial_reach;
            // *payoff /= initial_reaches.get_reach(Player::Traverser, hole_card);
        }
        for (hole_card, payoff) in traverser_payoffs.iter_mut() {
            let initial_reach = initial_reaches.get_reach(Player::Traverser, hole_card);
            let initial_reach = initial_reach / reaches_sum;
            println!(
                "{:?} {}, {}, {}",
                hole_card,
                initial_reach,
                payoff,
                *payoff / initial_reach
            );
            // *payoff /= initial_reaches.get_reach(Player::Traverser, hole_card);
        }
        // println!("Reaches sum {}", reaches_sum);
        res.traverser_payoffs = traverser_payoffs;
        // println!("{}", res);

        HashMap::new()
    }

    fn calculate_initial_reaches(
        &mut self,
        action_history: &Vec<Action>,
        traverser_cards: &Vec<(Card, Card)>,
        opponent_cards: &Vec<(Card, Card)>,
    ) -> HoleCardReaches {
        // Needed to do this so that we can get initial reaches
        self.game_state.cards[0] = traverser_cards[0].0;
        self.game_state.cards[1] = traverser_cards[0].1;
        self.game_state.cards[2] = opponent_cards[0].0;
        self.game_state.cards[3] = opponent_cards[0].1;

        let mut game_state =
            GameStateHelper::new(self.game_state.cards, self.game_state.small_blind_player);
        let mut reaches = HoleCardReaches::new(traverser_cards, opponent_cards);
        self.dealt_board_cards = Vec::new();

        for action in action_history {
            let round = (game_state.cards_dealt).saturating_sub(2) as usize;
            let current_player_pot = game_state.get_current_player_pot();
            let bets_this_round = game_state.bets_this_street;
            let num_available_actions = game_state.get_num_available_actions();
            let current_player = game_state.current_player;

            let all_hole_cards = match current_player {
                Player::Traverser => traverser_cards,
                Player::Opponent => opponent_cards,
            };

            match action {
                Action::Deal(card) => {
                    game_state.deal();
                    self.dealt_board_cards.push(*card);
                }
                Action::Bet => {
                    for hole_cards in all_hole_cards {
                        let strategy = self.get_strategy(
                            hole_cards,
                            &current_player,
                            round,
                            current_player_pot,
                            bets_this_round,
                            num_available_actions,
                        );
                        let action_probability = strategy[2.min(num_available_actions - 1)];
                        reaches.update(current_player, hole_cards, action_probability);
                    }
                    game_state.bet();
                    game_state.switch_current_player();
                }
                Action::Call => {
                    for hole_cards in all_hole_cards {
                        let strategy = self.get_strategy(
                            hole_cards,
                            &current_player,
                            round,
                            current_player_pot,
                            bets_this_round,
                            num_available_actions,
                        );
                        let action_probability = strategy[1];
                        reaches.update(current_player, hole_cards, action_probability);
                    }
                    game_state.call();
                    game_state.switch_current_player();
                }
                Action::CheckFold => {
                    for hole_cards in all_hole_cards {
                        let strategy = self.get_strategy(
                            hole_cards,
                            &current_player,
                            round,
                            current_player_pot,
                            bets_this_round,
                            num_available_actions,
                        );
                        let action_probability = strategy[0];
                        reaches.update(current_player, hole_cards, action_probability);
                    }
                    game_state.checkfold();
                    game_state.switch_current_player();
                }
            }
        }
        reaches
    }

    pub fn handle_or_create_subtree(&mut self, reaches: &HoleCardReaches) -> HoleCardPayoffs {
        match self.solving_state {
            SolvingState::Solving => {
                panic!("Should not be creating a sub-tree when in the solving state");
            }
            SolvingState::Exploring => {
                println!("Inserting");
                self.subtrees.insert(
                    self.action_history.clone(),
                    CbvSubTree {
                        strategy_map: self.strategy_map,
                        action_history: self.action_history.clone(),
                        reaches: reaches.clone_non_zero(),
                        game_state: self.game_state.clone(),
                        dealt_board_cards: self.dealt_board_cards.clone(),
                        solving_state: SolvingState::Solving,
                        hole_card_payoffs: None,
                        // unused
                        subtrees: HashMap::default(),
                    },
                );
                HoleCardPayoffs::default()
            }
            SolvingState::Resolving => {
                println!("Resolving");
                self.get_computed_payoffs()
            }
        }
    }

    fn compute_payoffs(&mut self, reaches: &HoleCardReaches, depth: usize) {
        if self.hole_card_payoffs.is_some() {
            panic!("Should not be computing payoffs for a node that already has payoffs");
        }
        if reaches != &self.reaches {
            panic!("Reaches should be the same as the reaches in the node");
        }
        self.hole_card_payoffs = Some(self.explore_sub_trees(reaches, depth));
    }

    fn get_computed_payoffs(&mut self) -> HoleCardPayoffs {
        let mut subtree = self
            .subtrees
            .remove(&self.action_history.clone())
            .expect("Should have found a matching sub-tree");
        subtree
            .hole_card_payoffs
            .take()
            .expect("Should not be getting payoffs for a node that has not been computed")
    }

    fn explore_sub_trees(&mut self, reaches: &HoleCardReaches, depth: usize) -> HoleCardPayoffs {
        if !matches!(self.solving_state, SolvingState::Solving) && depth == 0 {
            return self.handle_or_create_subtree(reaches);
        }

        let num_available_actions = self.game_state.get_num_available_actions();

        let pot_before_action = self.game_state.get_current_player_pot();
        let current_player = self.game_state.current_player;

        match self.game_state.check_street_terminal() {
            TerminalState::None => {
                let round = (self.game_state.cards_dealt).saturating_sub(2) as usize;
                let bets_this_round = self.game_state.bets_this_street;
                let bets_before_action = self.game_state.bets_this_street;
                let previous_player = self.game_state.current_player;
                let checks_before = self.game_state.checks_this_street;

                //// Here we're calculating CBV as described in Safe and Nested Subgame Solving for Imperfect-Information Games
                self.perform_action(
                    reaches.clone_non_zero(),
                    num_available_actions,
                    current_player,
                    round,
                    bets_this_round,
                    pot_before_action,
                    bets_before_action,
                    previous_player,
                    checks_before,
                    depth - 1,
                )
            }
            TerminalState::StreetOver => self.traverse_deal(reaches, 0),
            TerminalState::Fold => {
                println!("Folding");
                match self.solving_state {
                    SolvingState::Solving => self.evaluate_fold(
                        reaches,
                        &current_player,
                        self.game_state.traverser_pot,
                        self.game_state.opponent_pot,
                    ),
                    _ => self.handle_or_create_subtree(reaches),
                }
            }
            TerminalState::Showdown => {
                println!("Showdown");
                match self.solving_state {
                    SolvingState::Solving => self.evaluate_showdown(reaches, pot_before_action),
                    _ => self.handle_or_create_subtree(reaches),
                }
            }
        }
    }

    pub fn evaluate_fold(
        &self,
        reaches: &HoleCardReaches,
        current_player: &Player,
        traverser_pot: u8,
        opponent_pot: u8,
    ) -> HoleCardPayoffs {
        println!("Evaluating fold {}", traverser_pot);
        let utility = match current_player {
            Player::Traverser => opponent_pot as f64,
            Player::Opponent => -(traverser_pot as f64),
        };
        HoleCardPayoffs::fold_payoffs(utility, reaches)
    }

    pub fn evaluate_showdown(
        &self,
        reaches: &HoleCardReaches,
        current_player_pot: u8,
    ) -> HoleCardPayoffs {
        let board_cards: [Card; 5] =
            self.dealt_board_cards
                .to_vec()
                .try_into()
                .unwrap_or_else(|_| {
                    panic!(
                        "Are we on the river {}, action history, {:?}",
                        self.game_state.is_river(),
                        self.action_history
                    )
                });
        let traverser_hand_rankings =
            self.hole_card_rankings_player(reaches, Player::Traverser, &board_cards);
        let opponent_hand_rankings =
            self.hole_card_rankings_player(reaches, Player::Opponent, &board_cards);

        HoleCardPayoffs::showdown_payoffs(
            current_player_pot,
            traverser_hand_rankings,
            opponent_hand_rankings,
            reaches,
        )
    }

    fn hole_card_rankings_player(
        &self,
        reaches: &HoleCardReaches,
        player: Player,
        board_cards: &[Card; 5],
    ) -> HashMap<(Card, Card), u16> {
        let player_hole_cards = reaches.get_hole_cards(player);
        self.hole_card_rankings(&player_hole_cards, board_cards)
    }

    fn hole_card_rankings(
        &self,
        hole_cards: &Vec<&(Card, Card)>,
        board_cards: &[Card; 5],
    ) -> HashMap<(Card, Card), u16> {
        let mut evaluations = HashMap::new();

        for card in hole_cards {
            let rank = EVALUATOR.evaluate_seven(&[card.0, card.1], board_cards, u16::MAX);
            evaluations.insert(**card, rank);
        }

        evaluations
    }

    fn perform_action(
        &mut self,
        reaches: HoleCardReaches,
        num_available_actions: usize,
        current_player: Player,
        round: usize,
        bets_this_round: u8,
        pot_before_action: u8,
        bets_before_action: u8,
        previous_player: Player,
        checks_before: u8,
        depth: usize,
    ) -> HoleCardPayoffs {
        let all_hole_cards = reaches.get_hole_cards(current_player);
        let mut next_reaches = vec![reaches.clone(); DEFAULT_ACTION_COUNT];

        if matches!(current_player, Player::Opponent) {
            for hole_cards in all_hole_cards.iter() {
                let strategy = self.get_strategy(
                    hole_cards,
                    &current_player,
                    round,
                    pot_before_action,
                    bets_this_round,
                    num_available_actions,
                );
                for action in 0..num_available_actions {
                    // store the reaches for each action
                    next_reaches[action].update(current_player, hole_cards, strategy[action]);
                }
            }
        }

        // println!("{}", current_player);

        let mut payoffs = HoleCardPayoffs::default();

        for action in 0..num_available_actions {
            let next_reaches = &next_reaches[action];
            let subpayoff = self.traverse_chosen_action(
                &next_reaches.clone_non_zero(),
                action,
                previous_player,
                pot_before_action,
                bets_before_action,
                checks_before,
                depth,
            );
            // payoffs.add_subpayoff(&subpayoff)
            // payoffs.max_subpayoff(&subpayoff)
            match current_player {
                Player::Traverser => payoffs.max_subpayoff(&subpayoff),
                Player::Opponent => payoffs.add_subpayoff(&subpayoff),
            }
        }
        payoffs
    }

    fn traverse_chosen_action(
        &mut self,
        reaches: &HoleCardReaches,
        action: usize,
        acting_player: Player,
        current_pot: u8,
        current_bets: u8,
        current_checks: u8,
        depth: usize,
    ) -> HoleCardPayoffs {
        match action {
            0 => {
                self.game_state.checkfold();
                self.action_history.push(Action::CheckFold);
            }
            1 => match self.game_state.call_or_bet() {
                Action::Call => self.action_history.push(Action::Call),
                Action::Bet => self.action_history.push(Action::Bet),
                _ => panic!("Invalid action"),
            },
            2 => {
                self.game_state.bet();
                self.action_history.push(Action::Bet);
            }
            _ => {
                panic!("Invalid action");
            }
        };

        self.game_state.switch_current_player();
        let subpayoffs = self.explore_sub_trees(reaches, depth);
        self.action_history.pop();
        self.game_state
            .undo(acting_player, current_pot, current_bets, current_checks);
        subpayoffs
    }

    fn traverse_deal(&mut self, reaches: &HoleCardReaches, depth: usize) -> HoleCardPayoffs {
        let previous_player = self.game_state.current_player;
        let previous_bets = self.game_state.bets_this_street;
        let checks_before = self.game_state.checks_this_street;

        let mut payoffs = HoleCardPayoffs::default();
        let potential_next_cards = self.get_potential_next_cards();
        let reach_multiplier = 1.0 / potential_next_cards.len() as f64;

        for card in potential_next_cards {
            self.action_history.push(Action::Deal(card));
            self.dealt_board_cards.push(card);
            self.game_state.deal();
            let subpayoffs =
                self.explore_sub_trees(&reaches.clone_without_card(&card, reach_multiplier), depth);
            payoffs.add_subpayoff(&subpayoffs);
            self.action_history.pop();
            self.dealt_board_cards.pop();
            self.game_state
                .undeal(previous_bets, previous_player, checks_before);
        }

        payoffs
    }

    fn get_potential_next_cards(&self) -> Vec<Card> {
        (0..52)
            .map(Card::from_int)
            .filter(|card| !self.dealt_board_cards.contains(card))
            .collect::<Vec<_>>()
    }

    fn get_strategy(
        &self,
        hole_cards: &(Card, Card),
        current_player: &Player,
        round: usize,
        current_player_pot: u8,
        bets_this_round: u8,
        num_available_actions: usize,
    ) -> [f64; DEFAULT_ACTION_COUNT] {
        let game_abstraction =
            self.get_abstraction_cache(hole_cards, round, current_player_pot, bets_this_round);
        let strategy_hub_key = Self::get_strategy_hub_key(
            hole_cards,
            &self.game_state.small_blind_player == current_player,
        );

        let strategy = self
            .strategy_map
            .get(&strategy_hub_key)
            .and_then(|strategy_branch| strategy_branch.get_strategy(&game_abstraction).cloned());

        match strategy {
            Some(strategy) => strategy.get_current_strategy(0),
            None => [1.0 / (num_available_actions as f64); DEFAULT_ACTION_COUNT],
        }
    }

    fn get_abstraction_cache(
        &self,
        hole_cards: &(Card, Card),
        round: usize,
        game_pot: u8,
        bets_this_round: u8,
    ) -> GameAbstractionSerialised {
        // TODO - Implement cache
        get_current_abstraction(
            hole_cards,
            &self.dealt_board_cards,
            round,
            game_pot,
            bets_this_round,
        )
    }

    fn get_strategy_hub_key(hole_cards: &(Card, Card), is_sb: bool) -> StrategyHubKey {
        StrategyHubKey {
            low_rank: hole_cards.0.rank,
            high_rank: hole_cards.1.rank,
            is_suited: hole_cards.0.suit == hole_cards.1.suit,
            is_sb,
        }
    }
}

fn convert_actions_to_game_state(actions: &[Action], sb_player: Player) -> GameStateHelper {
    let game_state_from_actions = actions_to_state(actions, sb_player);
    GameStateHelper {
        game_abstraction: GameAbstraction::default(),
        traverser_pot: game_state_from_actions.traverser_pot,
        opponent_pot: game_state_from_actions.opponent_pot,
        cards: game_state_from_actions.partial_deal,
        cards_dealt: game_state_from_actions.cards_dealt,
        current_player: game_state_from_actions.current_player,
        small_blind_player: game_state_from_actions.small_blind_player,
        big_blind_player: game_state_from_actions.big_blind_player,
        bets_this_street: game_state_from_actions.bets_this_round,
        winner: None,
        checks_this_street: game_state_from_actions.checks_this_round,
    }
}

fn actions_to_state(actions: &[Action], small_blind_player: Player) -> GameStateFromActions {
    let mut partial_deal = [Card::default(); 9];

    partial_deal[1] = Card::new(Suit::Spades, Rank::Ace); // Small hack because we expect the hole cards to be sorted
    partial_deal[3] = Card::new(Suit::Clubs, Rank::Ace);

    let mut deal_index = 4;
    let mut cards_dealt = 0;

    let mut traverser_pot = match small_blind_player {
        Player::Traverser => 1,
        Player::Opponent => 2,
    };
    let mut opponent_pot = match small_blind_player {
        Player::Opponent => 1,
        Player::Traverser => 2,
    };

    let mut checks_this_round = 0;
    let mut bets_this_round = 0;

    let mut current_player = small_blind_player;

    for action in actions {
        match action {
            Action::Deal(card) => {
                partial_deal[deal_index] = *card;
                current_player = small_blind_player.get_opposite();
                cards_dealt += 1;
                deal_index += 1;
                bets_this_round = 0;
                checks_this_round = 0;
            }
            Action::Bet => {
                bets_this_round += 1;
                let multiplier = if cards_dealt < 5 { 1 } else { 2 };
                match current_player {
                    Player::Traverser => {
                        traverser_pot = opponent_pot + BIG_BLIND * multiplier;
                    }
                    Player::Opponent => {
                        opponent_pot = traverser_pot + BIG_BLIND * multiplier;
                    }
                }
                current_player = current_player.get_opposite();
            }
            Action::Call => {
                match current_player {
                    Player::Traverser => {
                        traverser_pot = opponent_pot;
                    }
                    Player::Opponent => {
                        opponent_pot = traverser_pot;
                    }
                }
                current_player = current_player.get_opposite();
            }
            Action::CheckFold => {
                if bets_this_round == 0 && traverser_pot == opponent_pot {
                    checks_this_round += 1;
                }
                current_player = current_player.get_opposite();
            }
        }
    }

    GameStateFromActions {
        partial_deal,
        traverser_pot,
        opponent_pot,
        cards_dealt,
        small_blind_player,
        big_blind_player: small_blind_player.get_opposite(),
        bets_this_round,
        checks_this_round,
        current_player,
    }
}
