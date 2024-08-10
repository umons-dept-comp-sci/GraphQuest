use GraphQuest::db_handler::{lib::*, sqlite_handler::*};




const DB_URL: &str = "resources/test.db";



#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Database test
    {
        let workspace: Workspace<SqliteGraphDatabase> = init_workspace(DB_URL).await;

        

        /*
        create_graph_database(DB_URL).await;    // Create a database if it didn't already exist
    
        let gdb = connect_graph_database(DB_URL).await;     // Connects to the created db 
        
    
        gdb.create_graph_table("result_of_research").await;     // Create a new table
    
    
        gdb.add_column_to_table("result_of_research", "euler", ColumnType::Text).await;
    
        //gdb.remove_column_from_table("result_of_research", "euler").await;
        
        gdb.create_graph_table("result_of_research2").await;     // Create a new table
        
        
        //let res = load_table_with_geng(9,
        //    &[
        //      GraphArgs::Connected], &gdb, "result_of_research2").await;
        
        read_pipe_input(&gdb, "result_of_research2").await;

        gdb.drop_table("result_of_research2").await; 
        */
    }


    
    



    

    
    println!("Hello, world!");
}
