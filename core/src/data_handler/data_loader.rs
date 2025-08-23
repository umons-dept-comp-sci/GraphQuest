/// List of all methods available to read the data
pub enum Method {
    /// Read input from GengAPI
    GengAPI {
        /// The number of vertices of the graphs to add
        nb_of_vertices: u32,
        /// The graph parameters to give, they must follow the `geng` rules
        graph_settings: String,
        /// The edges boundaries of the graph to generate
        edges_bound: (Option<u32>, Option<u32>),
    },
    /// Read input from stdin
    Stdin,
    /// Read input from file using a given path
    File(String),
}
