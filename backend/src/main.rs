use gquest_core::{data_handler, db_handler::{graph_database::*, sqlite_handler::*}};




const DB_URL: &str = "resources/test.db";



#[tokio::main(flavor = "current_thread")]
async fn main() {
    
    //let wp: Workspace<SqliteGraphDatabase> = Workspace::init_workspace(DB_URL).await;
    //let wp : Workspace<SqliteGraphDatabase> = Workspace::connect_workspace(DB_URL).await;
    //wp.add_dataset(GraphQuest::db_handler::data_loaders::Method::Stdin).await;
    //wp.add_dataset<O>(data_handler::data_loaders::Method::GengAPI { nb_of_vertices: 6, graph_settings: "-c".to_string(), edges_born: (Some(4), None)}, None).await;
    //wp.close_workspace().await;

    
    println!("Hello, world!");
}
