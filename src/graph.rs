use crate::parser::*;
use petgraph::graphmap::DiGraphMap;
use unidiff::PatchSet;

type NodeWeight = u32;

#[derive(Debug)]
pub struct DiffGraphParams {
    pub diff_repository_dir: String,
    pub diff: PatchSet,
    pub save_default_if_missing: bool, 
    pub install_lang_if_missing: bool,
}

#[derive(Debug)]
pub struct DiffGraph {
    graph: DiGraphMap<NodeWeight, Node>,
    diffs: Vec<Diff>,
}

#[derive(Debug)]
pub struct Node {
    pub id: String,
}

impl DiffGraph {
    pub fn create(params: DiffGraphParams) -> Result<Self, String> {
        let graph = DiGraphMap::new();
        let diffs = match try_parse_patch(
            &params.diff, 
            None, 
            params.save_default_if_missing, 
            params.install_lang_if_missing) 
        {
            Ok(diffs) => diffs,
            Err(e) => return Err(e.to_string())
        };

        Ok(Self {
            graph,
            diffs,
        })
    }
}
