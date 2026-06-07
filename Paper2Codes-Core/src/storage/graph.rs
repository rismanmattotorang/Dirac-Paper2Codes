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
