extern crate petgraph;
extern crate rand;
extern crate serde_json;
extern crate csv;

use petgraph::algo::find_negative_cycle;
use petgraph::dot::Dot;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use rand::Rng;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::Path;

/// Reads ticker mapping and price data from JSON files.
fn data() -> (HashMap<String, Value>, Vec<Value>) {
    let data = fs::read_to_string("./dict.json")
        .expect("Unable to read dict.json");
    let ticker_mapping: HashMap<String, Value> =
        serde_json::from_str(&data).expect("JSON does not have correct format.");

    let price_data = fs::read_to_string("./mock_prices.js")
        .expect("Unable to read mock_prices.js");
    let json: Vec<Value> =
        serde_json::from_str(&price_data).expect("JSON does not have correct format.");
    (ticker_mapping, json)
}

/// Builds a directed graph from the provided data.
fn graph_builder(
    ticker_mapping: HashMap<String, Value>,
    json: Vec<Value>,
) -> (DiGraph<String, f64>, HashMap<String, NodeIndex>) {
    let mut graph = DiGraph::<String, f64>::new();
    let mut nodes = HashMap::new();

    for entry in json {
        if let Some(symbol) = entry.get("symbol").and_then(|v| v.as_str()) {
            if let Some(mapping) = ticker_mapping.get(symbol) {
                if let Some(price_str) = entry.get("price").and_then(|v| v.as_str()) {
                    if let Ok(price) = price_str.parse::<f64>() {
                        let base = mapping.get("base").and_then(|v| v.as_str()).unwrap_or("");
                        let other = mapping.get("other").and_then(|v| v.as_str()).unwrap_or("");

                        let node_a = *nodes
                            .entry(base.to_string())
                            .or_insert_with(|| graph.add_node(base.to_string()));
                        let node_b = *nodes
                            .entry(other.to_string())
                            .or_insert_with(|| graph.add_node(other.to_string()));

                        if price > 0.0 {
                            // Use the log2 transformation as weights.
                            graph.add_edge(node_a, node_b, -price.log2());
                            graph.add_edge(node_b, node_a, price.log2());
                        }
                    }
                }
            }
        }
    }
    println!("Full graph built. DOT representation:\n{}", Dot::new(&graph));
    (graph, nodes)
}

/// Creates an induced subgraph by sampling nodes.
fn sample_subgraph(
    graph: &DiGraph<String, f64>,
    sample_ratio: f64,
) -> (DiGraph<String, f64>, HashMap<NodeIndex, NodeIndex>) {
    let mut new_graph = DiGraph::<String, f64>::new();
    let mut mapping = HashMap::new();
    let mut rng = rand::thread_rng();

    for node in graph.node_indices() {
        if rng.gen::<f64>() < sample_ratio {
            let label = graph[node].clone();
            let new_node = new_graph.add_node(label);
            mapping.insert(node, new_node);
        }
    }
    // Add edges if both endpoints are present.
    for edge in graph.edge_indices() {
        let (src, dst) = graph.edge_endpoints(edge).unwrap();
        if let (Some(&new_src), Some(&new_dst)) = (mapping.get(&src), mapping.get(&dst)) {
            new_graph.add_edge(new_src, new_dst, graph[edge]);
        }
    }
    (new_graph, mapping)
}

/// Writes the current graph in DOT format.
fn output_dot_file(graph: &DiGraph<String, f64>, dot_filename: &str) -> Result<(), Box<dyn Error>> {
    let dot_dir = "dot_files";
    fs::create_dir_all(dot_dir)?;
    let dot_path = Path::new(dot_dir).join(dot_filename);
    let dot_str = format!("{:?}", Dot::new(graph));
    let mut file = fs::File::create(dot_path)?;
    file.write_all(dot_str.as_bytes())?;
    Ok(())
}

/// Detects a negative cycle in the graph.
/// Returns the cycle as a vector of node indices if found.
fn negative_finder(graph: &DiGraph<String, f64>) -> Option<Vec<NodeIndex>> {
    let path = find_negative_cycle(graph, NodeIndex::new(0));
    println!("Checking for negative cycles...");
    if let Some(p) = path {
        println!("Negative cycle found.");
        Some(p)
    } else {
        println!("No negative cycle detected.");
        None
    }
}

/// Removes one node from the detected negative cycle.
fn remove_nodes(graph: &mut DiGraph<String, f64>, nodes_to_remove: &[NodeIndex]) {
    if nodes_to_remove.len() > 2 {
        let node_to_remove = nodes_to_remove[2];
        graph.remove_node(node_to_remove);
        println!("Removed node {} from the negative cycle.", node_to_remove.index());
    }
}

/// Computes the average out-degree of nodes in the graph.
fn average_out_degree(graph: &DiGraph<String, f64>) -> f64 {
    if graph.node_count() == 0 {
        return 0.0;
    }
    let total: usize = graph.node_indices()
        .map(|n| graph.neighbors_directed(n, petgraph::Direction::Outgoing).count())
        .sum();
    total as f64 / graph.node_count() as f64
}

fn main() -> Result<(), Box<dyn Error>> {
    // Step 1: Set sample ratio.
    let sample_ratio = 1.0;
    println!("Setting sample ratio to {} ({}% of nodes).", sample_ratio, sample_ratio * 100.0);

    // Step 2: Read data.
    println!("Reading data from files...");
    let (ticker_mapping, json) = data();

    // Step 3: Build full graph.
    println!("Building the full graph...");
    let (full_graph, _) = graph_builder(ticker_mapping, json);

    // Step 4: Sample subgraph.
    println!("Sampling subgraph using sample ratio {}...", sample_ratio);
    let (mut graph, _mapping) = sample_subgraph(&full_graph, sample_ratio);

    // Step 5: Output initial DOT file.
    println!("Generating DOT file for the initial sampled subgraph...");
    output_dot_file(&graph, "graph_updated_0.dot")?;

    // Prepare vectors to store metrics.
    let mut profit_history: Vec<f64> = Vec::new();
    let mut cycle_length_history: Vec<usize> = Vec::new();
    let mut centrality_history: Vec<f64> = Vec::new();
    let mut iterations = 0;

    // Step 6: Detect and remove negative cycles, recording metrics.
    while let Some(negative_cycle) = negative_finder(&graph) {
        // Compute cycle profit by traversing the cycle.
        let mut cycle_profit = 1.0;
        for window in negative_cycle.windows(2) {
            if let [start_node, end_node] = window {
                if let Some(edge) = graph.find_edge(*start_node, *end_node) {
                    let weight = graph[edge];
                    let price = 2f64.powf(-weight);
                    cycle_profit *= price;
                }
            }
        }
        // Close the cycle: from the last node back to the first.
        if let (Some(&last_node), Some(&first_node)) = (negative_cycle.last(), negative_cycle.first()) {
            if let Some(edge) = graph.find_edge(last_node, first_node) {
                let weight = graph[edge];
                let price = 2f64.powf(-weight);
                cycle_profit *= price;
            }
        }
        println!("Cycle profit: {}", cycle_profit);

        // Record metrics.
        profit_history.push(cycle_profit);
        cycle_length_history.push(negative_cycle.len());
        let avg_deg = average_out_degree(&graph);
        centrality_history.push(avg_deg);
        println!("Average out-degree: {}", avg_deg);

        // Remove a node from the cycle.
        remove_nodes(&mut graph, &negative_cycle);
        iterations += 1;

        // Save updated DOT file.
        let dot_filename = format!("graph_updated_{}.dot", iterations);
        output_dot_file(&graph, &dot_filename)?;
    }

    println!("Processing complete. No more negative cycles detected.");

    // Step 7: Write metrics to a CSV file.
    let mut wtr = csv::Writer::from_path("metrics.csv")?;
    wtr.write_record(&["iteration", "profit", "cycle_length", "centrality"])?;
    for (i, ((&profit, &cycle_length), &centrality)) in
        profit_history.iter().zip(&cycle_length_history).zip(&centrality_history).enumerate() {
            wtr.write_record(&[
                i.to_string(),
                profit.to_string(),
                cycle_length.to_string(),
                centrality.to_string()
            ])?;
    }
    wtr.flush()?;
    println!("Metrics saved to metrics.csv.");

    Ok(())
}
