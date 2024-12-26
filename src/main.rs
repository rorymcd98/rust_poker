mod models;
mod traversal;
mod thread_utils;

use crate::traversal::main_traverse::TreeTraverser;

fn main() {
    let mut traverser = TreeTraverser::new(1);
    traverser.BeginTreeTraversal();
}
