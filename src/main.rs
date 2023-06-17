mod cli;
mod graph;

use graph::*;

fn main() {
    match cli::get_params() {
        Ok(params) => {
            match DiffGraph::create(params) {
                Ok(graph) => println!("{:?}", graph),
                Err(e) => println!("{}", e),
            }
        },
        Err(e) => println!("{}", e),
    }
}
