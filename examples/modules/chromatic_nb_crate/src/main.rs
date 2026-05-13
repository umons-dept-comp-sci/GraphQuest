use std::io;

use graph6_rs::Graph;

fn get_adj_matrix(signature: &str) -> Vec<Vec<bool>> {
    let graph = Graph::from_g6(signature).expect("valid signature");
    let mut adj_matrix: Vec<Vec<bool>> = (0..graph.n)
        .map(|_| (0..graph.n).map(|_| false).collect())
        .collect();

    graph.bit_vec.iter().enumerate().for_each(|(i, val)| {
        let b: bool = *val == 1;

        adj_matrix[i / graph.n][i % graph.n] = b;
    });

    adj_matrix
}

/// Checks if it is possible to colour the given vertex with a colour
fn is_safe(adj: &[Vec<bool>], colours: &[usize], vertex: usize, colour: usize) -> bool {
    for (j, neighbor) in adj[vertex].iter().enumerate() {
        // check if it has the same color
        if *neighbor && colours[j] == colour {
            return false;
        }
    }
    true
}

/// Recursively tries to colour a graph using k colours.
fn try_colouring(adj: &[Vec<bool>], k: usize, colors: &mut Vec<usize>, curr_vertex: usize) -> bool {
    // if all vertex were coloured, then a valid colouration was found.
    if curr_vertex == adj.len() {
        return true;
    }

    // tries to apply all colours on the current vertex and see if it leads to a solution
    for c in 1..(k + 1) {
        if is_safe(adj, colors, curr_vertex, c) {
            colors[curr_vertex] = c;
            if try_colouring(adj, k, colors, curr_vertex + 1) {
                return true;
            }
            // backtracking
            colors[curr_vertex] = 0;
        }
    }
    false
}

/// Check wether the given graph is k-colourable or not
fn is_k_colourable(adj: &[Vec<bool>], k: usize) -> bool {
    // Creates the array containing the current colouration of a graph.
    let mut colors: Vec<usize> = (0..adj.len()).map(|_| 0).collect();
    try_colouring(adj, k, &mut colors, 0)
}

/// Gets the chromatic number of a graph using a dictionary search.
fn get_chromatic_nb(adj: &[Vec<bool>], s: usize, e: usize, previous: Option<usize>) -> usize {
    let size_left = (((e as isize) - (s as isize)).abs() + 1) as usize;

    let current_k = s + (size_left / 2);
    let previous = previous.unwrap_or(current_k);

    if current_k == 0 {
        return previous;
    } else if size_left == 1 {
        if is_k_colourable(adj, current_k) {
            return current_k;
        } else {
            return previous;
        }
    }

    if is_k_colourable(adj, current_k) {
        get_chromatic_nb(adj, s, current_k - 1, Some(current_k))
    } else {
        get_chromatic_nb(adj, current_k + 1, e, Some(previous))
    }
}

fn main() {
    // Captures the stdin buffer
    let mut stdin = io::stdin().lines();
    // While the stdin is open, wait and read a line
    while let Some(Ok(signature)) = stdin.next() {
        let adj_matrix: Vec<Vec<bool>> = get_adj_matrix(&signature);

        // Writes output to stdout
        println!(
            "{signature} {}",
            get_chromatic_nb(&adj_matrix, 0, adj_matrix.len(), None)
        ); // Flushes the output by default
    }
}
