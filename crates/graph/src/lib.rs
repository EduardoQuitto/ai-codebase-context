//! `aicc-graph`: structural code graph (Fase 8 foundation).
//!
//! Minimal directed multigraph: file/symbol nodes, typed edges with
//! explicit confidence. Centrality algorithms are Fase 8+ concerns.

use aicc_core::model::{Confidence, RelationKind};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub kind: NodeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeKind {
    File,
    Symbol,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub kind: RelationKind,
    pub confidence: Confidence,
}

#[derive(Debug, Default)]
pub struct Graph {
    nodes: HashMap<String, Node>,
    edges: Vec<Edge>,
}

impl Graph {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, id: impl Into<String>, kind: NodeKind) {
        let id = id.into();
        self.nodes.entry(id.clone()).or_insert(Node { id, kind });
    }

    pub fn add_edge(
        &mut self,
        from: impl Into<String>,
        to: impl Into<String>,
        kind: RelationKind,
        confidence: Confidence,
    ) {
        let from = from.into();
        let to = to.into();
        self.add_node(from.clone(), NodeKind::Symbol);
        self.add_node(to.clone(), NodeKind::Symbol);
        self.edges.push(Edge { from, to, kind, confidence });
    }

    /// Push a prebuilt edge (explicit kinds preserved).
    pub fn push_edge(&mut self, edge: Edge) {
        self.add_node(edge.from.clone(), NodeKind::Symbol);
        self.add_node(edge.to.clone(), NodeKind::Symbol);
        self.edges.push(edge);
    }

    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn outgoing(&self, id: &str) -> impl Iterator<Item = &Edge> {
        self.edges.iter().filter(move |e| e.from == id)
    }

    pub fn incoming(&self, id: &str) -> impl Iterator<Item = &Edge> {
        self.edges.iter().filter(move |e| e.to == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bidirectional_navigation() {
        let mut g = Graph::new();
        g.add_edge("a", "b", RelationKind::Imports, Confidence::High);
        assert_eq!(g.node_count(), 2);
        assert_eq!(g.edge_count(), 1);
        assert_eq!(g.outgoing("a").count(), 1);
        assert_eq!(g.incoming("b").count(), 1);
        assert_eq!(g.incoming("a").count(), 0);
    }
}
