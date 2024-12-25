mod models;
mod traversal;

use crate::traversal::main_traverse::TreeTraverser;

fn main() {
    let mut traverser = TreeTraverser::new(1);
    traverser.BeginTreeTraversal();
}
