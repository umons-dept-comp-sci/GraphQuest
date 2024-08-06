use GraphQuest::db_handler::{connect_graph_database, create_graph_database, GraphDatabase};



const DB_URL: &str = "resources/test.db";



#[tokio::main(flavor = "current_thread")]
async fn main() {
    create_graph_database(DB_URL).await;    // Create a database if it didn't already exist

    let gdb = connect_graph_database(DB_URL).await;     // Connects to the created db 
    

    gdb.create_graph_table("result_of_research").await;     // Create a new table



    
    

    
    println!("Hello, world!");
}
