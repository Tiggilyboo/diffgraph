mod cli;
mod graph;
mod parser;
mod grammars;

use graph::*;

fn main() {
    match cli::get_params() {
        Ok(params) => {
            match DiffGraph::create(params) {
                Ok(_) => (),
                Err(e) => println!("{}", e),
            }

        },
        Err(e) => println!("{}", e),
    }
}
