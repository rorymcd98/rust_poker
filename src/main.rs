use rust_poker::begin_tree_train_traversal;
use rust_poker::solve_cbr_utilties;
use rust_poker::validation::validate_strategies;
use std::env;

pub fn main() {
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
        }
        "solve" => {
            println!("Solving...");
            solve_cbr_utilties();
        }
        "validate" => {
            println!("Validating...");
            validate_strategies();
        }
        _ => {
            eprintln!("Invalid command: {}", args[1]);
            eprintln!("Usage: {} <train|solve|validate>", args[0]);
            std::process::exit(1);
        }
    }
}
