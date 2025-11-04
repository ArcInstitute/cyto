use hashbrown::HashMap;
use petgraph::{
    unionfind::UnionFind,
    visit::{EdgeRef, IntoEdgeReferences, IntoNodeIdentifiers, NodeCompactIndexable},
};

/// Compute the connected components of an undirected graph or weakly connected
/// components of a directed graph.
///
/// # Arguments
/// * `g`: a directed or undirected graph.
///
/// # Returns
/// Return a vector where each element is a connected component containing the
/// node identifiers. The order of nodes within each component is arbitrary.
///
/// For a directed graph, this returns weakly connected components.
/// For an undirected graph, this returns connected components.
///
/// # Complexity
/// * Time complexity: **O(|V| + |E|)**.
/// * Auxiliary space: **O(|V|)**.
///
/// where **|V|** is the number of nodes and **|E|** is the number of edges.
///
/// # Examples
///
/// ```rust
/// use petgraph::Graph;
/// use petgraph::prelude::*;
/// use cyto_ibu_umi_correct::connected_components_vec;
///
/// let mut graph: Graph<(), (), Undirected> = Graph::new_undirected();
/// let a = graph.add_node(());
/// let b = graph.add_node(());
/// let c = graph.add_node(());
/// let d = graph.add_node(());
///
/// graph.extend_with_edges(&[(a, b), (b, c), (d, d)]);
/// // a --- b --- c    d (self-loop)
///
/// let components = connected_components_vec(&graph);
/// assert_eq!(components.len(), 2); // Two components
/// for component in components {
///     assert!(component.len() == 3 || component.len() == 1);
/// }
/// ```
pub fn connected_components_vec<G>(g: G) -> Vec<Vec<G::NodeId>>
where
    G: NodeCompactIndexable + IntoEdgeReferences + IntoNodeIdentifiers,
{
    let mut node_sets = UnionFind::new(g.node_bound());

    // Union nodes connected by edges
    for edge in g.edge_references() {
        let (a, b) = (edge.source(), edge.target());
        node_sets.union(g.to_index(a), g.to_index(b));
    }

    // Group nodes by their component label
    let labels = node_sets.into_labeling();
    let mut components: HashMap<usize, Vec<G::NodeId>> = HashMap::new();

    for node in g.node_identifiers() {
        let idx = g.to_index(node);
        components
            .entry(labels[idx])
            .or_insert_with(Vec::new)
            .push(node);
    }

    // Convert HashMap values to Vec
    components.into_values().collect()
}

#[cfg(test)]
mod testing {
    use super::*;
    use petgraph::Directed;
    use petgraph::Undirected;
    use petgraph::graph::{Graph, NodeIndex};
    use std::collections::HashSet;

    #[test]
    fn test_empty_graph() {
        let graph: Graph<(), (), Undirected> = Graph::new_undirected();
        let components = connected_components_vec(&graph);
        assert_eq!(components.len(), 0);
    }

    #[test]
    fn test_single_node() {
        let mut graph: Graph<(), (), Undirected> = Graph::new_undirected();
        graph.add_node(());
        let components = connected_components_vec(&graph);
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].len(), 1);
    }

    #[test]
    fn test_two_isolated_nodes() {
        let mut graph: Graph<(), (), Undirected> = Graph::new_undirected();
        graph.add_node(());
        graph.add_node(());
        let components = connected_components_vec(&graph);
        assert_eq!(components.len(), 2);
        for component in components {
            assert_eq!(component.len(), 1);
        }
    }

    #[test]
    fn test_simple_connected_component() {
        let mut graph: Graph<(), (), Undirected> = Graph::new_undirected();
        let a = graph.add_node(());
        let b = graph.add_node(());
        let c = graph.add_node(());

        graph.add_edge(a, b, ());
        graph.add_edge(b, c, ());

        let components = connected_components_vec(&graph);
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].len(), 3);
    }

    #[test]
    fn test_two_separate_components() {
        let mut graph: Graph<(), (), Undirected> = Graph::new_undirected();
        let a = graph.add_node(());
        let b = graph.add_node(());
        let c = graph.add_node(());
        let d = graph.add_node(());

        graph.add_edge(a, b, ());
        graph.add_edge(c, d, ());

        let components = connected_components_vec(&graph);
        assert_eq!(components.len(), 2);

        // Check that each component has the right size
        let sizes: HashSet<usize> = components.iter().map(|c| c.len()).collect();
        assert_eq!(sizes, HashSet::from([2]));
    }

    #[test]
    fn test_self_loop() {
        let mut graph: Graph<(), (), Undirected> = Graph::new_undirected();
        let a = graph.add_node(());
        let b = graph.add_node(());
        let c = graph.add_node(());

        graph.add_edge(a, b, ());
        graph.add_edge(b, c, ());
        graph.add_edge(c, c, ()); // self-loop

        let components = connected_components_vec(&graph);
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].len(), 3);
    }

    #[test]
    fn test_isolated_node_with_self_loop() {
        let mut graph: Graph<(), (), Undirected> = Graph::new_undirected();
        let a = graph.add_node(());
        let b = graph.add_node(());
        let c = graph.add_node(());
        let d = graph.add_node(());

        graph.add_edge(a, b, ());
        graph.add_edge(b, c, ());
        graph.add_edge(d, d, ()); // isolated self-loop

        let components = connected_components_vec(&graph);
        assert_eq!(components.len(), 2);

        let sizes: Vec<usize> = components.iter().map(|c| c.len()).collect();
        assert!(sizes.contains(&3));
        assert!(sizes.contains(&1));
    }

    #[test]
    fn test_multiple_components_various_sizes() {
        let mut graph: Graph<(), (), Undirected> = Graph::new_undirected();

        // Component 1: 4 nodes
        let a = graph.add_node(());
        let b = graph.add_node(());
        let c = graph.add_node(());
        let d = graph.add_node(());
        graph.add_edge(a, b, ());
        graph.add_edge(b, c, ());
        graph.add_edge(c, d, ());

        // Component 2: 2 nodes
        let e = graph.add_node(());
        let f = graph.add_node(());
        graph.add_edge(e, f, ());

        // Component 3: 1 isolated node
        graph.add_node(());

        let components = connected_components_vec(&graph);
        assert_eq!(components.len(), 3);

        let mut sizes: Vec<usize> = components.iter().map(|c| c.len()).collect();
        sizes.sort_unstable();
        assert_eq!(sizes, vec![1, 2, 4]);
    }

    #[test]
    fn test_cycle() {
        let mut graph: Graph<(), (), Undirected> = Graph::new_undirected();
        let a = graph.add_node(());
        let b = graph.add_node(());
        let c = graph.add_node(());
        let d = graph.add_node(());

        // Create a cycle: a-b-c-d-a
        graph.add_edge(a, b, ());
        graph.add_edge(b, c, ());
        graph.add_edge(c, d, ());
        graph.add_edge(d, a, ());

        let components = connected_components_vec(&graph);
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].len(), 4);
    }

    #[test]
    fn test_complete_graph() {
        let mut graph: Graph<(), (), Undirected> = Graph::new_undirected();
        let nodes: Vec<NodeIndex> = (0..5).map(|_| graph.add_node(())).collect();

        // Connect every node to every other node
        for i in 0..nodes.len() {
            for j in i + 1..nodes.len() {
                graph.add_edge(nodes[i], nodes[j], ());
            }
        }

        let components = connected_components_vec(&graph);
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].len(), 5);
    }

    #[test]
    fn test_directed_graph_weakly_connected() {
        let mut graph: Graph<(), (), Directed> = Graph::new();
        let a = graph.add_node(());
        let b = graph.add_node(());
        let c = graph.add_node(());

        // a -> b -> c (weakly connected, not strongly)
        graph.add_edge(a, b, ());
        graph.add_edge(b, c, ());

        let components = connected_components_vec(&graph);
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].len(), 3);
    }

    #[test]
    fn test_directed_graph_two_components() {
        let mut graph: Graph<(), (), Directed> = Graph::new();
        let a = graph.add_node(());
        let b = graph.add_node(());
        let c = graph.add_node(());
        let d = graph.add_node(());

        // Component 1: a -> b
        graph.add_edge(a, b, ());

        // Component 2: c -> d
        graph.add_edge(c, d, ());

        let components = connected_components_vec(&graph);
        assert_eq!(components.len(), 2);

        for component in components {
            assert_eq!(component.len(), 2);
        }
    }

    #[test]
    fn test_star_graph() {
        let mut graph: Graph<(), (), Undirected> = Graph::new_undirected();
        let center = graph.add_node(());
        let leaves: Vec<NodeIndex> = (0..5).map(|_| graph.add_node(())).collect();

        // Connect all leaves to center
        for leaf in leaves {
            graph.add_edge(center, leaf, ());
        }

        let components = connected_components_vec(&graph);
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].len(), 6); // 1 center + 5 leaves
    }

    #[test]
    fn test_line_graph() {
        let mut graph: Graph<(), (), Undirected> = Graph::new_undirected();
        let nodes: Vec<NodeIndex> = (0..10).map(|_| graph.add_node(())).collect();

        // Create a line: 0-1-2-3-4-5-6-7-8-9
        for i in 0..nodes.len() - 1 {
            graph.add_edge(nodes[i], nodes[i + 1], ());
        }

        let components = connected_components_vec(&graph);
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].len(), 10);
    }

    #[test]
    fn test_multiple_edges_same_nodes() {
        let mut graph: Graph<(), (), Undirected> = Graph::new_undirected();
        let a = graph.add_node(());
        let b = graph.add_node(());

        // Add multiple edges between same nodes
        graph.add_edge(a, b, ());
        graph.add_edge(a, b, ());
        graph.add_edge(a, b, ());

        let components = connected_components_vec(&graph);
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].len(), 2);
    }

    #[test]
    fn test_bridge_connects_components() {
        let mut graph: Graph<(), (), Undirected> = Graph::new_undirected();

        // First clique
        let a = graph.add_node(());
        let b = graph.add_node(());
        let c = graph.add_node(());
        graph.add_edge(a, b, ());
        graph.add_edge(b, c, ());
        graph.add_edge(c, a, ());

        // Second clique
        let d = graph.add_node(());
        let e = graph.add_node(());
        let f = graph.add_node(());
        graph.add_edge(d, e, ());
        graph.add_edge(e, f, ());
        graph.add_edge(f, d, ());

        // Before bridge: 2 components
        let components = connected_components_vec(&graph);
        assert_eq!(components.len(), 2);

        // Add bridge
        graph.add_edge(c, d, ());

        // After bridge: 1 component
        let components = connected_components_vec(&graph);
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].len(), 6);
    }

    fn gnp_graph(n: usize, p: f64) -> Graph<usize, usize, Undirected> {
        let mut graph = Graph::new_undirected();
        for i in 0..n {
            graph.add_node(i);
        }
        for i in 0..n {
            for j in i + 1..n {
                if rand::random::<f64>() < p {
                    graph.add_edge(NodeIndex::new(i), NodeIndex::new(j), i + j);
                }
            }
        }
        graph
    }

    fn gnm_graph(n: usize, m: usize) -> Graph<usize, usize, Undirected> {
        let mut graph = Graph::new_undirected();
        for i in 0..n {
            graph.add_node(i);
        }
        for _ in 0..m {
            let i = rand::random_range(0..n);
            let j = rand::random_range(0..n);
            if i != j {
                graph.add_edge(NodeIndex::new(i), NodeIndex::new(j), i + j);
            }
        }
        graph
    }

    #[test]
    fn test_random_gnm_graphs_and_scc() {
        let mut min_components = usize::MAX;
        let mut max_components = usize::MIN;
        for _ in 0..100 {
            let n = 100;
            let m = 33;
            let graph = gnm_graph(n, m);
            let components = connected_components_vec(&graph);

            min_components = min_components.min(components.len());
            max_components = max_components.max(components.len());

            assert_eq!(
                components.len(),
                petgraph::algo::connected_components(&graph)
            );
            assert_eq!(components.len(), petgraph::algo::kosaraju_scc(&graph).len());
            assert_eq!(components.len(), petgraph::algo::tarjan_scc(&graph).len());
        }
        // println!("Min components: {}", min_components);
        // println!("Max components: {}", max_components);
        // assert!(false);
    }

    #[test]
    fn test_random_gnp_graphs_and_scc() {
        let mut min_components = usize::MAX;
        let mut max_components = usize::MIN;
        for _ in 0..100 {
            let n = 100;
            let p = 0.01;
            let graph = gnp_graph(n, p);
            let components = connected_components_vec(&graph);

            min_components = min_components.min(components.len());
            max_components = max_components.max(components.len());

            assert_eq!(
                components.len(),
                petgraph::algo::connected_components(&graph)
            );
            assert_eq!(components.len(), petgraph::algo::kosaraju_scc(&graph).len());
            assert_eq!(components.len(), petgraph::algo::tarjan_scc(&graph).len());
        }
        // println!("Min components: {}", min_components);
        // println!("Max components: {}", max_components);
        // assert!(false);
    }
}
