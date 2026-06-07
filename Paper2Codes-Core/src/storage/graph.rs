use crate::storage::GraphAnalysis;
use crate::types::DependencyGraph;

/// Graph analysis operations
pub struct GraphAnalyzer;

impl GraphAnalyzer {
    /// Analyze dependency graph structure
    pub fn analyze_graph(graph: &DependencyGraph) -> GraphAnalysis {
        let total_modules = Self::count_unique_modules(graph);
        let total_dependencies = graph.edges.len();
        let max_depth = Self::calculate_max_depth(graph);
        let average_dependencies = if total_modules > 0 {
            total_dependencies as f64 / total_modules as f64
        } else {
            0.0
        };
        let isolated_modules = Self::find_isolated_modules(graph);

        GraphAnalysis {
            total_modules,
            total_dependencies,
            max_depth,
            average_dependencies,
            isolated_modules,
        }
    }

    /// Topologically order modules so that every module appears *after* all the
    /// modules it depends on (Phase 6 graph-native generation ordering).
    ///
    /// `nodes` is the full module set (so isolated modules are included even
    /// when they have no edges). Edge `(from, to)` means *from depends on to*.
    /// The order is deterministic given the input order. Cycles cannot stall the
    /// algorithm: any nodes left after a Kahn pass (i.e. involved in a cycle) are
    /// appended in input order.
    pub fn topological_order(nodes: &[String], graph: &DependencyGraph) -> Vec<String> {
        use std::collections::{HashMap, HashSet, VecDeque};

        // Full node set, preserving input order, then any extra edge endpoints.
        let mut all: Vec<String> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        for n in nodes {
            if seen.insert(n.clone()) {
                all.push(n.clone());
            }
        }
        for (from, to) in &graph.edges {
            for endpoint in [from, to] {
                if seen.insert(endpoint.clone()) {
                    all.push(endpoint.clone());
                }
            }
        }

        // In-degree = number of dependencies; dependents[to] = nodes depending on `to`.
        let mut indegree: HashMap<String, usize> = all.iter().map(|n| (n.clone(), 0)).collect();
        let mut dependents: HashMap<String, Vec<String>> = HashMap::new();
        for (from, to) in &graph.edges {
            if let Some(d) = indegree.get_mut(from) {
                *d += 1;
            }
            dependents.entry(to.clone()).or_default().push(from.clone());
        }

        // Kahn's algorithm, seeding the queue in input order for determinism.
        let mut queue: VecDeque<String> = all
            .iter()
            .filter(|n| indegree.get(*n).copied().unwrap_or(0) == 0)
            .cloned()
            .collect();
        let mut order: Vec<String> = Vec::with_capacity(all.len());
        let mut placed: HashSet<String> = HashSet::new();

        while let Some(node) = queue.pop_front() {
            if !placed.insert(node.clone()) {
                continue;
            }
            order.push(node.clone());
            if let Some(deps) = dependents.get(&node) {
                for dependent in deps {
                    if let Some(d) = indegree.get_mut(dependent) {
                        *d = d.saturating_sub(1);
                        if *d == 0 {
                            queue.push_back(dependent.clone());
                        }
                    }
                }
            }
        }

        // Cycle fallback: append any unplaced nodes in input order.
        if order.len() < all.len() {
            for n in &all {
                if placed.insert(n.clone()) {
                    order.push(n.clone());
                }
            }
        }
        order
    }

    /// Breadth-first transitive dependency closure of `start`, up to `max_hops`
    /// (Phase 6 GraphRAG-style structural expansion). Excludes `start`; returns
    /// dependencies in BFS order.
    pub fn dependency_closure(
        graph: &DependencyGraph,
        start: &str,
        max_hops: usize,
    ) -> Vec<String> {
        use std::collections::HashSet;

        let mut result = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        seen.insert(start.to_string());
        let mut frontier = vec![start.to_string()];

        for _ in 0..max_hops {
            let mut next = Vec::new();
            for node in &frontier {
                for dep in graph.get_dependencies(node) {
                    if seen.insert(dep.clone()) {
                        result.push(dep.clone());
                        next.push(dep);
                    }
                }
            }
            if next.is_empty() {
                break;
            }
            frontier = next;
        }
        result
    }

    /// Find circular dependencies in a graph
    pub fn find_circular_dependencies(graph: &DependencyGraph) -> Vec<Vec<String>> {
        let mut cycles = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut rec_stack = std::collections::HashSet::new();
        let mut path = Vec::new();

        // Build adjacency list
        let mut adj: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for (from, to) in &graph.edges {
            adj.entry(from.clone())
                .or_insert_with(Vec::new)
                .push(to.clone());
        }

        // DFS to find cycles
        for node in adj.keys() {
            if !visited.contains(node) {
                Self::dfs_cycle(
                    node,
                    &adj,
                    &mut visited,
                    &mut rec_stack,
                    &mut path,
                    &mut cycles,
                );
            }
        }

        cycles
    }

    fn dfs_cycle(
        node: &String,
        adj: &std::collections::HashMap<String, Vec<String>>,
        visited: &mut std::collections::HashSet<String>,
        rec_stack: &mut std::collections::HashSet<String>,
        path: &mut Vec<String>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        visited.insert(node.clone());
        rec_stack.insert(node.clone());
        path.push(node.clone());

        if let Some(neighbors) = adj.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    Self::dfs_cycle(neighbor, adj, visited, rec_stack, path, cycles);
                } else if rec_stack.contains(neighbor) {
                    // Found a cycle
                    if let Some(start_idx) = path.iter().position(|x| x == neighbor) {
                        cycles.push(path[start_idx..].to_vec());
                    }
                }
            }
        }

        rec_stack.remove(node);
        path.pop();
    }

    fn count_unique_modules(graph: &DependencyGraph) -> usize {
        let mut modules = std::collections::HashSet::new();
        for (from, to) in &graph.edges {
            modules.insert(from.clone());
            modules.insert(to.clone());
        }
        modules.len()
    }

    fn calculate_max_depth(graph: &DependencyGraph) -> usize {
        // Build adjacency list
        let mut adj: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for (from, to) in &graph.edges {
            adj.entry(from.clone())
                .or_insert_with(Vec::new)
                .push(to.clone());
        }

        // Find root nodes (nodes with no incoming edges)
        let mut has_incoming = std::collections::HashSet::new();
        for (_, to_list) in &adj {
            for to in to_list {
                has_incoming.insert(to.clone());
            }
        }

        let mut max_depth = 0;
        for node in adj.keys() {
            if !has_incoming.contains(node) {
                let depth = Self::dfs_depth(node, &adj, &mut std::collections::HashSet::new());
                max_depth = max_depth.max(depth);
            }
        }

        max_depth
    }

    fn dfs_depth(
        node: &String,
        adj: &std::collections::HashMap<String, Vec<String>>,
        visited: &mut std::collections::HashSet<String>,
    ) -> usize {
        if visited.contains(node) {
            return 0;
        }
        visited.insert(node.clone());

        let mut max_child_depth = 0;
        if let Some(neighbors) = adj.get(node) {
            for neighbor in neighbors {
                let child_depth = Self::dfs_depth(neighbor, adj, visited);
                max_child_depth = max_child_depth.max(child_depth);
            }
        }

        1 + max_child_depth
    }

    fn find_isolated_modules(graph: &DependencyGraph) -> Vec<String> {
        let mut modules = std::collections::HashSet::new();
        for (from, to) in &graph.edges {
            modules.insert(from.clone());
            modules.insert(to.clone());
        }

        // This is a simplified version - in practice, isolated means no edges at all
        // For a more complete implementation, we'd need to check both incoming and outgoing
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    /// Assert that `before` precedes `after` in `order`.
    fn precedes(order: &[String], before: &str, after: &str) {
        let bi = order.iter().position(|x| x == before).unwrap();
        let ai = order.iter().position(|x| x == after).unwrap();
        assert!(bi < ai, "expected {} before {} in {:?}", before, after, order);
    }

    #[test]
    fn topo_order_places_dependencies_first() {
        // c depends on b, b depends on a  => order a, b, c
        let mut g = DependencyGraph::new();
        g.add_dependency("c".into(), "b".into());
        g.add_dependency("b".into(), "a".into());
        let order = GraphAnalyzer::topological_order(&ids(&["a", "b", "c"]), &g);
        assert_eq!(order.len(), 3);
        precedes(&order, "a", "b");
        precedes(&order, "b", "c");
    }

    #[test]
    fn topo_order_handles_diamond_and_isolated() {
        // d depends on b and c; b,c depend on a; e isolated.
        let mut g = DependencyGraph::new();
        g.add_dependency("b".into(), "a".into());
        g.add_dependency("c".into(), "a".into());
        g.add_dependency("d".into(), "b".into());
        g.add_dependency("d".into(), "c".into());
        let order = GraphAnalyzer::topological_order(&ids(&["a", "b", "c", "d", "e"]), &g);
        assert_eq!(order.len(), 5);
        precedes(&order, "a", "b");
        precedes(&order, "a", "c");
        precedes(&order, "b", "d");
        precedes(&order, "c", "d");
        assert!(order.contains(&"e".to_string()));
    }

    #[test]
    fn topo_order_cycle_does_not_stall_and_includes_all() {
        let mut g = DependencyGraph::new();
        g.add_dependency("a".into(), "b".into());
        g.add_dependency("b".into(), "a".into());
        let order = GraphAnalyzer::topological_order(&ids(&["a", "b"]), &g);
        assert_eq!(order.len(), 2);
        assert!(order.contains(&"a".to_string()) && order.contains(&"b".to_string()));
    }

    #[test]
    fn dependency_closure_is_bfs_and_hop_limited() {
        // a -> b -> c -> d
        let mut g = DependencyGraph::new();
        g.add_dependency("a".into(), "b".into());
        g.add_dependency("b".into(), "c".into());
        g.add_dependency("c".into(), "d".into());

        assert_eq!(GraphAnalyzer::dependency_closure(&g, "a", 1), ids(&["b"]));
        assert_eq!(GraphAnalyzer::dependency_closure(&g, "a", 2), ids(&["b", "c"]));
        assert_eq!(
            GraphAnalyzer::dependency_closure(&g, "a", 10),
            ids(&["b", "c", "d"])
        );
        assert!(GraphAnalyzer::dependency_closure(&g, "d", 5).is_empty());
    }
}
