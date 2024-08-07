use GraphQuest::db_handler::{connect_graph_database, create_graph_database};


use GraphQuest::data_handlers::geng_api::*;


const DB_URL: &str = "resources/test.db";



#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Database test
    {
        create_graph_database(DB_URL).await;    // Create a database if it didn't already exist
    
        let gdb = connect_graph_database(DB_URL).await;     // Connects to the created db 
        
    
        gdb.create_graph_table("result_of_research").await;     // Create a new table
    
    
        gdb.add_column_to_table("result_of_research", "euler", GraphQuest::db_handler::ColumnType::Text).await;
    
        gdb.remove_column_from_table("result_of_research", "euler").await;
    }
    

    // geng test
    {
        
        geng(15,
            &[GraphArgs::Biconnected,
              GraphArgs::ClawFree,
              GraphArgs::Connected]);
    }
    
    



    

    
    println!("Hello, world!");
}
