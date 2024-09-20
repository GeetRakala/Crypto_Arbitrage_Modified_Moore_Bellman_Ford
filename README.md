# Arbitrage Cycle Detection with Rust

This Rust project builds a directed graph to model cryptocurrency tickers and their corresponding exchange prices. It uses the **Petgraph** library to represent relationships between different assets and searches for arbitrage opportunities by detecting negative cycles in the graph.

## Key Features:
- **Data Parsing**: Reads cryptocurrency tickers and price data from JSON files (`dict.json` and `mock_prices.js`).
- **Graph Construction**: Builds a directed graph using base currencies and other pairs with the logarithmic values of their price differences.
- **Cycle Detection**: Identifies arbitrage opportunities by detecting negative cycles in the graph using `find_negative_cycle()` from the Petgraph library.
- **Cycle Removal**: Once a negative cycle (an arbitrage opportunity) is found, it removes the involved nodes from the graph and continues searching for further cycles.
- **Graph Visualization**: Uses the DOT format to visualize the graph structure for debugging purposes.

## How it works:
1. **Data Ingestion**: The program reads ticker mapping and mock price data from two JSON files.
2. **Graph Building**: A directed graph is created, where nodes represent different assets, and edges represent price discrepancies between them.
3. **Arbitrage Detection**: The program detects and prints negative cycles (indicating potential arbitrage opportunities) and calculates the profit.
4. **Node Removal**: After identifying a negative cycle, the involved nodes are removed from the graph to avoid repeated cycles.

## Dependencies:
- **Petgraph**: For building and manipulating the directed graph.
- **Serde and Serde-JSON**: For reading and deserializing JSON data.
- **Plotters**: For potential graph visualization (currently unused in the provided code).

## Running the Program:
Make sure to have the required JSON files (`dict.json` and `mock_prices.js`) in the correct directory structure. Then, run the project using Cargo:

```bash
cargo run
