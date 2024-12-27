mod models;
mod traversal;
mod thread_utils;
mod evaluate;

use crate::traversal::main_traverse::TreeTraverser;

fn main() {
    let traverser = TreeTraverser::new(1);
    traverser.begin_tree_traversal();
}
