use crate::parser::*;
use petgraph::graphmap::DiGraphMap;
use unidiff::PatchSet;
use tree_sitter::{Tree, TreeCursor};
use tree_sitter::Node as TSNode;
use std::collections::HashMap;

type NodeWeight = usize;

#[derive(Debug)]
pub struct DiffGraphParams {
    pub diff_repository_dir: String,
    pub diff: PatchSet,
    pub save_default_if_missing: bool, 
    pub install_lang_if_missing: bool,
}

#[derive(Debug)]
pub struct DiffGraph {
    graph: DiGraphMap<NodeWeight, Edge>,
    diffs: Vec<Diff>,
}

#[derive(Debug)]
pub struct NodeInfo {
    pub id: usize,
    pub kind_id: u16,
    pub byte_range: std::ops::Range<usize>,
}

#[derive(Debug)]
pub struct Edge {
    pub from: NodeInfo,
    pub to: NodeInfo,
}

impl NodeInfo {
    pub fn from_ts_node(ts_node: &TSNode) -> Self {
        Self {
            id: ts_node.id(),
            kind_id: ts_node.kind_id(),
            byte_range: ts_node.byte_range(),
        }
    }
}
impl Edge {
    pub fn from_ts_nodes(from: &TSNode, to: &TSNode) -> Self {
        Self {
            from: NodeInfo::from_ts_node(from),
            to: NodeInfo::from_ts_node(to),
        }
    }
}

pub struct TreeIterator<'a, F> 
where F: FnMut(TSNode, TSNode) 
{
    walker: TreeCursor<'a>,
    traversed: bool,
    relation_cb: F,
}

impl<'a, F> TreeIterator<'a, F>
where F: FnMut(TSNode, TSNode) 
{
    pub fn new(tree: &'a Tree, relation_cb: F) -> Self {
        Self {
            walker: tree.walk(),
            traversed: false,
            relation_cb,
        }
    }

    fn reset(&mut self) {
        // Reset root
        while self.walker.goto_parent() {}
        self.traversed = false;
    }
}

impl<'a, F> Iterator for TreeIterator<'a, F> 
where F: FnMut(TSNode, TSNode) 
{
    type Item = TSNode<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.traversed {
            return None;
        }
        let node = self.walker.node();

        if self.walker.goto_first_child() || self.walker.goto_next_sibling() {
            (self.relation_cb)(node, self.walker.node());
            return Some(node);
        }
        loop {
            if !self.walker.goto_parent() {
                self.traversed = true;
                break;
            }
            if self.walker.goto_next_sibling() {
                break;
            }
        }

        // From, To
        (self.relation_cb)(node, self.walker.node());
        Some(node)
    }
}

impl DiffGraph {

    pub fn create(params: DiffGraphParams) -> Result<Self, String> {
        let diffs = match try_parse_patch(
            &params.diff, 
            None, 
            params.save_default_if_missing, 
            params.install_lang_if_missing) 
        {
            Ok(diffs) => diffs,
            Err(e) => return Err(e.to_string())
        };
        let graph = Self::create_graph_from_diffs(&diffs)?;
        println!("graph (n# {}, e#: {})", graph.node_count(), graph.edge_count());

        Ok(Self {
            graph,
            diffs,
        })
    }

    pub fn create_graph_from_diffs(diffs: &Vec<Diff>) -> Result<DiGraphMap<NodeWeight, Edge>, String> {
        let mut graph = DiGraphMap::new();
        for d in diffs {
            let mut dfs = TreeIterator::new(&d.tree, |from, to| {
                let from_node_id = graph.add_node(from.id());
                let to_node_id = graph.add_node(to.id());
                let edge = Edge::from_ts_nodes(&from, &to);

                graph.add_edge(from_node_id, to_node_id, edge);
            });

            let mut c = 0;
            while dfs.next().is_some() {
                c += 1;
            }

            println!("Processed {} nodes in dfs.", c);
        }

        Ok(graph)
    }
}
