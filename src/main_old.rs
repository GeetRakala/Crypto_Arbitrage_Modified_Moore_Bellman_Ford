extern crate petgraph;
extern crate rand;
extern crate serde_json;

use petgraph::algo::find_negative_cycle;
use petgraph::dot::Dot;
use petgraph::graph::{DiGraph, NodeIndex};
use rand::Rng;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::Path;

fn data() -> (HashMap<String, Value>, Vec<Value>) {
    let data = fs::read_to_string("./dict.json")
        .expect("Unable to read file");
    let ticker_mapping: HashMap<String, Value> =
        serde_json::from_str(&data).expect("JSON does not have correct format.");

    let price_data = fs::read_to_string("./mock_prices.js")
        .expect("Unable to read file");
    let json: Vec<Value> =
        serde_json::from_str(&price_data).expect("JSON does not have correct format.");
    (ticker_mapping, json)
}

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

/// Returns an induced subgraph consisting of nodes sampled with probability sample_ratio.
/// The returned mapping relates original node indices to their new indices in the subgraph.
fn sample_subgraph(
    graph: &DiGraph<String, f64>,
    sample_ratio: f64,
) -> (DiGraph<String, f64>, HashMap<NodeIndex, NodeIndex>) {
    let mut new_graph = DiGraph::<String, f64>::new();
    let mut mapping = HashMap::new();
    let mut rng = rand::thread_rng();

    // Sample nodes from the original graph.
    for node in graph.node_indices() {
        if rng.gen::<f64>() < sample_ratio {
            let label = graph[node].clone();
            let new_node = new_graph.add_node(label);
            mapping.insert(node, new_node);
        }
    }
    // Add edges only if both endpoints are present in the subgraph.
    for edge in graph.edge_indices() {
        let (src, dst) = graph.edge_endpoints(edge).unwrap();
        if let (Some(&new_src), Some(&new_dst)) = (mapping.get(&src), mapping.get(&dst)) {
            new_graph.add_edge(new_src, new_dst, graph[edge]);
        }
    }
    (new_graph, mapping)
}

/// Writes the graph as a DOT file.
fn output_dot_file(graph: &DiGraph<String, f64>, dot_filename: &str) -> Result<(), Box<dyn Error>> {
    let dot_dir = "dot_files";
    fs::create_dir_all(dot_dir)?; // Ensure the directory exists
    let dot_path = Path::new(dot_dir).join(dot_filename);
    let dot_str = format!("{:?}", Dot::new(graph));
    let mut file = fs::File::create(dot_path)?;
    file.write_all(dot_str.as_bytes())?;
    Ok(())
}


/// Detects a negative cycle in the graph.
/// Returns the vector of node indices representing the cycle, if found.
fn negative_finder(graph: &DiGraph<String, f64>) -> Option<Vec<NodeIndex>> {
    let path = find_negative_cycle(graph, NodeIndex::new(0));
    let mut profit = 1.0;
    println!("Checking for negative cycles...");
    if let Some(p) = path {
        println!("Negative cycle found:");
        for window in p.windows(2) {
            if let [start_node, end_node] = window {
                if let Some(edge) = graph.find_edge(*start_node, *end_node) {
                    let weight = graph[edge];
                    let price = 2f64.powf(-weight);
                    profit *= price;
                    println!(
                        "  Edge from {} to {}: Price {}",
                        graph[*start_node],
                        graph[*end_node],
                        price
                    );
                }
            }
        }
        if let (Some(&last_node), Some(&first_node)) = (p.last(), p.first()) {
            if let Some(edge) = graph.find_edge(last_node, first_node) {
                let weight = graph[edge];
                let price = 2f64.powf(-weight);
                profit *= price;
                println!(
                    "  Edge from {} to {}: Price {}",
                    graph[last_node],
                    graph[first_node],
                    price
                );
                println!("  Total profit: {}", profit);
            }
        }
        Some(p)
    } else {
        println!("No negative cycle detected.");
        None
    }
}

/// Removes one node from the detected negative cycle.
fn remove_nodes(graph: &mut DiGraph<String, f64>, nodes_to_remove: &[NodeIndex]) {
    if nodes_to_remove.len() > 2 {
        // Remove a specific node from the cycle; adjust as needed.
        graph.remove_node(nodes_to_remove[2]);
        println!("Removed node {} from the negative cycle.", nodes_to_remove[2].index());
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // Step 1: Set the sample ratio.
    let sample_ratio = 1.0;
    println!("Step 1: Setting sample ratio to {} (i.e., {}% of nodes).", sample_ratio, sample_ratio * 100.0);

    // Step 2: Read data.
    println!("Step 2: Reading data from files...");
    let (ticker_mapping, json) = data();

    // Step 3: Build the full graph.
    println!("Step 3: Building the full graph...");
    let (full_graph, _) = graph_builder(ticker_mapping, json);

    // Step 4: Create a subgraph by sampling nodes.
    println!("Step 4: Sampling subgraph using sample ratio {}...", sample_ratio);
    let (mut graph, _mapping) = sample_subgraph(&full_graph, sample_ratio);

    // Step 5: Store the initial sampled subgraph.
    println!("Step 5: Generating DOT file for the initial sampled subgraph...");
    output_dot_file(&graph, "graph_updated_0.dot")?;

    // Step 6: Detect and remove negative cycles.
    let mut iteration = 0;
    while let Some(negative_cycle) = negative_finder(&graph) {
        println!("Step 6: Negative cycle detected. Removing node from cycle...");
        remove_nodes(&mut graph, &negative_cycle);
        iteration += 1;
        let dot_filename = format!("graph_updated_{}.dot", iteration);
        let image_filename = format!("graph_updated_{}.png", iteration);
        println!("Step 7: Generating DOT file '{}' and image '{}' after removal...", dot_filename, image_filename);
        output_dot_file(&graph, &dot_filename)?;
    }

    println!("Processing complete. No more negative cycles detected.");
    Ok(())
}
