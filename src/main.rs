use std::env;
use rust_poker::validation::validate_strategies;
use rust_poker::solve_cbr_utilties2;
use rust_poker::begin_tree_train_traversal;

pub fn main() {
        begin_tree_train_traversal();
        let args: Vec<String> = env::args().collect();
        if args.len() < 2 {
                eprintln!("Usage: {} <train|solve|validate>", args[0]);
                std::process::exit(1);
        }

        match args[1].as_str() {
                "train" => {
                // Replace with your training logic function
                println!("Training...");
                begin_tree_train_traversal();
                },
                "solve" => {
                println!("Solving...");
                solve_cbr_utilties2();
                },
                "validate" => {
                println!("Validating...");
                validate_strategies();
                },
                _ => {
                eprintln!("Invalid command: {}", args[1]);
                eprintln!("Usage: {} <train|solve|validate>", args[0]);
                std::process::exit(1);
                }
        }
}