use bevy_egui::egui;
use std::collections::{HashMap, HashSet};
use tracing::info;

/// Represents a node in the production chain graph (a resource)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceNode {
    pub name: String,
}

/// Represents an edge in the production chain graph (input → output relationship)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductionEdge {
    pub from: String,
    pub to: String,
    pub recipe_name: String,
}

/// Production chain graph data structure
#[derive(Debug, Clone, Default)]
pub struct ProductionGraph {
    pub nodes: Vec<ResourceNode>,
    pub edges: Vec<ProductionEdge>,
    opened_logged: bool,
}

impl ProductionGraph {
    /// Create a new empty production graph
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            opened_logged: false,
        }
    }

    /// Build a graph from hardcoded production recipes (matching 7.4.1-7.4.4)
    pub fn from_recipes() -> Self {
        let mut graph = Self::new();

        // Steel production paths (7.4.1-7.4.3)
        graph.add_edge("IronOre", "Steel", "Iron → Steel");
        graph.add_edge("Coal", "Steel", "Coal + Iron → Steel");
        graph.add_edge("IronOre", "Steel", "Coal + Iron → Steel");
        graph.add_edge("Nickel", "Steel", "Nickel + Iron → Steel");
        graph.add_edge("IronOre", "Steel", "Nickel + Iron → Steel");

        // Electronics production (7.4.2)
        graph.add_edge("CopperOre", "Electronics", "Copper → Electronics");
        graph.add_edge("Gold", "Electronics", "Gold + Graphite → Electronics");
        graph.add_edge("Graphite", "Electronics", "Gold + Graphite → Electronics");

        // Battery production (7.4.3)
        graph.add_edge("Lithium", "BatteryPack", "Lithium + Cobalt → Battery");
        graph.add_edge("Cobalt", "BatteryPack", "Lithium + Cobalt → Battery");
        graph.add_edge("SolarPower", "BatteryPack", "Solar + Lithium → Battery");
        graph.add_edge("Lithium", "BatteryPack", "Solar + Lithium → Battery");
        graph.add_edge("BatteryPack", "Energy", "Battery → Energy");

        // Energy production (7.4.3)
        graph.add_edge("Silicon", "SolarPower", "Silicon + Titanium → Solar");
        graph.add_edge("Titanium", "SolarPower", "Silicon + Titanium → Solar");
        graph.add_edge("Uranium", "NuclearFuel", "Uranium → Nuclear Fuel");
        graph.add_edge("Gas", "HydrogenCell", "Gas + Coal → Hydrogen");
        graph.add_edge("Coal", "HydrogenCell", "Gas + Coal → Hydrogen");

        // Food production (7.4.4)
        graph.add_edge("Nutrients", "Food", "Food Production");

        graph
    }

    /// Add an edge to the graph (automatically adds nodes if they don't exist)
    pub fn add_edge(&mut self, from: &str, to: &str, recipe_name: &str) {
        // Add nodes if they don't exist
        let from_node = ResourceNode {
            name: from.to_string(),
        };
        let to_node = ResourceNode {
            name: to.to_string(),
        };

        if !self.nodes.contains(&from_node) {
            self.nodes.push(from_node);
        }
        if !self.nodes.contains(&to_node) {
            self.nodes.push(to_node);
        }

        // Add edge
        self.edges.push(ProductionEdge {
            from: from.to_string(),
            to: to.to_string(),
            recipe_name: recipe_name.to_string(),
        });
    }

    /// Get all nodes that are outputs (have incoming edges)
    pub fn output_nodes(&self) -> HashSet<String> {
        self.edges.iter().map(|e| e.to.clone()).collect()
    }

    /// Get all nodes that are inputs (have outgoing edges)
    pub fn input_nodes(&self) -> HashSet<String> {
        self.edges.iter().map(|e| e.from.clone()).collect()
    }

    /// Get all recipes that produce a specific resource
    pub fn recipes_for(&self, resource: &str) -> Vec<String> {
        self.edges
            .iter()
            .filter(|e| e.to == resource)
            .map(|e| e.recipe_name.clone())
            .collect()
    }

    /// Count nodes and edges
    pub fn metrics(&self) -> (usize, usize) {
        (self.nodes.len(), self.edges.len())
    }
}

/// State for the production graph visualization window
#[derive(Debug, Clone)]
pub struct ProductionGraphState {
    pub graph: ProductionGraph,
    pub show_window: bool,
}

impl Default for ProductionGraphState {
    fn default() -> Self {
        Self {
            graph: ProductionGraph::from_recipes(),
            show_window: false,
        }
    }
}

/// Render the production chain graph as a simple hierarchical view
pub fn render_production_graph(ui: &mut egui::Ui, state: &mut ProductionGraphState) {
    if !state.graph.opened_logged {
        let (nodes, edges) = state.graph.metrics();
        info!("Production chain graph: {} nodes, {} edges", nodes, edges);
        state.graph.opened_logged = true;
    }

    ui.heading("Production Chain Graph");
    ui.separator();

    let (nodes, edges) = state.graph.metrics();
    ui.label(format!("Nodes: {}, Edges: {}", nodes, edges));
    ui.separator();

    // Group by output resource
    let outputs = state.graph.output_nodes();
    let mut sorted_outputs: Vec<_> = outputs.into_iter().collect();
    sorted_outputs.sort();

    ui.label("Production Chains:");
    egui::ScrollArea::vertical()
        .max_height(400.0)
        .show(ui, |ui| {
            for output in sorted_outputs {
                ui.group(|ui| {
                    ui.strong(&output);
                    let recipes = state.graph.recipes_for(&output);
                    for recipe in recipes {
                        ui.label(format!("  ← {}", recipe));
                    }
                });
            }
        });
}

/// Render the production graph in a window
pub fn render_production_graph_window(ui: &mut egui::Ui, state: &mut ProductionGraphState) -> bool {
    let mut open = state.show_window;

    egui::Window::new("Production Chain Visualization")
        .open(&mut open)
        .default_width(400.0)
        .show(ui.ctx(), |ui| {
            render_production_graph(ui, state);
        });

    state.show_window = open;
    open
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_graph() {
        let graph = ProductionGraph::new();
        assert_eq!(graph.nodes.len(), 0);
        assert_eq!(graph.edges.len(), 0);
    }

    #[test]
    fn test_add_edge_creates_nodes() {
        let mut graph = ProductionGraph::new();
        graph.add_edge("Iron", "Steel", "Smelt");

        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
        assert!(graph.nodes.iter().any(|n| n.name == "Iron"));
        assert!(graph.nodes.iter().any(|n| n.name == "Steel"));
    }

    #[test]
    fn test_from_recipes_creates_graph() {
        let graph = ProductionGraph::from_recipes();

        // Should have multiple nodes and edges from the hardcoded recipes
        assert!(graph.nodes.len() > 0, "Graph should have nodes");
        assert!(graph.edges.len() > 0, "Graph should have edges");

        // Verify some key resources exist
        assert!(graph.nodes.iter().any(|n| n.name == "Steel"));
        assert!(graph.nodes.iter().any(|n| n.name == "Electronics"));
        assert!(graph.nodes.iter().any(|n| n.name == "BatteryPack"));
    }

    #[test]
    fn test_output_nodes() {
        let mut graph = ProductionGraph::new();
        graph.add_edge("A", "B", "Recipe1");
        graph.add_edge("C", "B", "Recipe2");
        graph.add_edge("B", "D", "Recipe3");

        let outputs = graph.output_nodes();
        assert!(outputs.contains("B"));
        assert!(outputs.contains("D"));
        assert!(!outputs.contains("A"));
        assert!(!outputs.contains("C"));
    }

    #[test]
    fn test_input_nodes() {
        let mut graph = ProductionGraph::new();
        graph.add_edge("A", "B", "Recipe1");
        graph.add_edge("C", "B", "Recipe2");
        graph.add_edge("B", "D", "Recipe3");

        let inputs = graph.input_nodes();
        assert!(inputs.contains("A"));
        assert!(inputs.contains("B"));
        assert!(inputs.contains("C"));
        assert!(!inputs.contains("D"));
    }

    #[test]
    fn test_recipes_for_resource() {
        let mut graph = ProductionGraph::new();
        graph.add_edge("Iron", "Steel", "Basic Smelting");
        graph.add_edge("Coal", "Steel", "Arc Furnace");
        graph.add_edge("Nickel", "Steel", "Enhanced Smelting");

        let steel_recipes = graph.recipes_for("Steel");
        assert_eq!(steel_recipes.len(), 3);
        assert!(steel_recipes.contains(&"Basic Smelting".to_string()));
        assert!(steel_recipes.contains(&"Arc Furnace".to_string()));
        assert!(steel_recipes.contains(&"Enhanced Smelting".to_string()));
    }

    #[test]
    fn test_metrics() {
        let mut graph = ProductionGraph::new();
        graph.add_edge("A", "B", "R1");
        graph.add_edge("C", "D", "R2");

        let (nodes, edges) = graph.metrics();
        assert_eq!(nodes, 4);
        assert_eq!(edges, 2);
    }

    #[test]
    fn test_render_graph_visual_test() {
        // Visual integration test: verify graph renders without panicking
        let ctx = egui::Context::default();
        let mut state = ProductionGraphState::default();

        ctx.begin_frame(egui::RawInput::default());
        egui::CentralPanel::default().show(&ctx, |ui| {
            render_production_graph(ui, &mut state);
        });
        let _ = ctx.end_frame();

        // Should have logged graph metrics
        assert!(state.graph.opened_logged);
    }

    #[test]
    fn test_graph_updates_when_recipes_change() {
        let mut graph = ProductionGraph::new();
        let initial_metrics = graph.metrics();

        // Add a new recipe
        graph.add_edge("NewInput", "NewOutput", "NewRecipe");
        let updated_metrics = graph.metrics();

        // Verify graph changed
        assert!(updated_metrics.0 > initial_metrics.0 || updated_metrics.1 > initial_metrics.1);
    }

    #[test]
    fn test_production_graph_state_default() {
        let state = ProductionGraphState::default();
        assert!(!state.show_window);
        assert!(
            state.graph.nodes.len() > 0,
            "Default state should have recipe graph"
        );
    }
}
