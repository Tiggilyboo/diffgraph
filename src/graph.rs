use petgraph::graphmap::DiGraphMap;

type NodeWeight = u32;

#[derive(Debug)]
pub struct DiffGraphParams {
    pub repository_dir: String,
    pub diff: String,
}

#[derive(Debug)]
pub struct DiffGraph {
    params: DiffGraphParams,
    graph: DiGraphMap<NodeWeight, Node>,
}

#[derive(Debug)]
pub struct Node {
    pub id: String,
}

impl DiffGraph {
    pub fn create(params: DiffGraphParams) -> Result<Self, String> {
        let graph = DiGraphMap::new();

        Ok(Self {
            params,
            graph,
        })
    }
}
