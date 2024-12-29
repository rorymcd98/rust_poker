mod evaluate;
mod models;
mod thread_utils;
mod traversal;

use crate::traversal::main_traverse::TreeTraverser;

fn main() {
    let traverser = TreeTraverser::new(1);
    traverser.begin_tree_traversal();
}
