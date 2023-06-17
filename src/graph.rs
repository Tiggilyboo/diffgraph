use crate::parser::*;
use petgraph::graphmap::DiGraphMap;
use tree_sitter::Tree;
use unidiff::PatchSet;

type NodeWeight = u32;

#[derive(Debug)]
pub struct DiffGraphParams {
    pub repository_dir: String,
    pub diff: PatchSet,
}

#[derive(Debug)]
pub struct DiffGraph {
    params: DiffGraphParams,
    graph: DiGraphMap<NodeWeight, Node>,
    trees: Vec<Tree>,
}

#[derive(Debug)]
pub struct Node {
    pub id: String,
}

impl DiffGraph {
    pub fn create(params: DiffGraphParams) -> Result<Self, String> {
        let graph = DiGraphMap::new();
        let trees = match try_parse_patch(&params.diff) {
            Ok(tree) => tree,
            Err(e) => return Err(e.to_string())
        };

        dbg!(&trees);

        Ok(Self {
            params,
            graph,
            trees,
        })
    }
}
