extern crate plotters;
extern crate petgraph;
extern crate serde_json;
//use petgraph::graph::Graph;
use serde_json::Value;
//use std::str::FromStr;
use petgraph::dot::Dot;
use std::fs;
//use plotters::prelude::*;
use petgraph::algo::find_negative_cycle;
//use petgraph::prelude::*;
//use petgraph_evcxr::draw_graph;

use std::collections::HashMap;
//use petgraph::graph::DiGraph; 
use petgraph::graph::{DiGraph, NodeIndex};

fn data() -> (HashMap<String, Value>, Vec<Value>) {
    let data = fs::read_to_string("./dict.json")
        .expect("Unable to read file");

    let ticker_mapping: HashMap<String, Value> = serde_json::from_str(&data)
        .expect("JSON does not have correct format.");

    let price_data = fs::read_to_string("./mock_prices.js")
        .expect("Unable to read file");

    let json: Vec<Value> = serde_json::from_str(&price_data)
        .expect("JSON does not have correct format.");
    return (ticker_mapping, json);
}

fn graph_builder(ticker_mapping: HashMap<String, Value>, json: Vec<Value>) -> (DiGraph::<String, f64>, HashMap<String, NodeIndex>){
    let mut graph = DiGraph::<String, f64>::new();
    let mut nodes = HashMap::new();

    for entry in json {
        if let Some(symbol) = entry.get("symbol").and_then(|v| v.as_str()) {
            if let Some(mapping) = ticker_mapping.get(symbol) {
                if let Some(price_str) = entry.get("price").and_then(|v| v.as_str()) {
                    if let Ok(price) = price_str.parse::<f64>() {
                        let base = mapping.get("base").and_then(|v| v.as_str()).unwrap_or("");
                        let other = mapping.get("other").and_then(|v| v.as_str()).unwrap_or("");

                        let node_a = *nodes.entry(base.to_string()).or_insert_with(|| graph.add_node(base.to_string()));
                        let node_b = *nodes.entry(other.to_string()).or_insert_with(|| graph.add_node(other.to_string()));

                        if price > 0.0 {
                            graph.add_edge(node_a, node_b, -price.log2());
                            graph.add_edge(node_b, node_a, price.log2());
                        }
                    }
                }
            }
        }
    }
    println!("{}", Dot::new(&graph));
    return (graph, nodes);
}

fn negative_finder(graph: &DiGraph<String, f64>, _nodes: &HashMap<String, NodeIndex>) -> Option<Vec<NodeIndex>> {
    let path = find_negative_cycle(graph, NodeIndex::new(0));
    let mut profit = 1.0;
    

    println!("Path: {:?}", path);
    if let Some(p) = path {
        //println!("Negative cycle found:");

        for window in p.windows(2) {
            if let [start_node, end_node] = window {
                if let Some(edge) = graph.find_edge(*start_node, *end_node) {
                    let weight = graph[edge];
                    let price = 2f64.powf(-weight);
                    profit = profit * price;
                    println!("Edge from {} to {}: Price {}", graph[*start_node], graph[*end_node], price);
                }
            }
        }

        if let Some(&last_node) = p.last() {
            if let Some(&first_node) = p.first() {
                if let Some(edge) = graph.find_edge(last_node, first_node) {
                    let weight = graph[edge];
                    let price = 2f64.powf(-weight);
                    profit = profit * price;
                    println!("Edge from {} to {}: Price {}", graph[last_node], graph[first_node], price);
                    println!("Profit: {}", profit);
                    println!();
                }
            }
        }

        Some(p)
    } else {
        None
    }
}

fn remove_nodes(graph: &mut DiGraph<String, f64>, nodes_to_remove: &[NodeIndex]) {
    

    //let node_label = &graph[nodes_to_remove[2]];
    //if node_label != "BTC" && node_label != "ETH" && node_label != "USDT" && node_label != "BNB" && node_label != "EUR" {
    //    graph.remove_node(nodes_to_remove[2]);
    //}
    graph.remove_node(nodes_to_remove[2]);
    println!("Nodes in the negative cycle have been deleted.");
}

fn main() {
    let (ticker_mapping, json) = data();

    let (mut graph, nodes) = graph_builder(ticker_mapping, json);
    
    //checka mexri na adiasei 
    while let Some(negative_cycle) = negative_finder(&graph, &nodes) {
        remove_nodes(&mut graph, &negative_cycle);
    }
}
