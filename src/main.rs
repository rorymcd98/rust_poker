use traversal::main_train::begin_tree_train_traversal;

mod evaluate;
mod models;
mod thread_utils;
mod traversal;
mod config;
mod misc_tests;

fn main() {
    begin_tree_train_traversal();
}
