use gquest_core::db_handler::{lib::*, sqlite_handler::*};




const DB_URL: &str = "resources/test.db";



#[tokio::main(flavor = "current_thread")]
async fn main() {
    
    //let wp: Workspace<SqliteGraphDatabase> = Workspace::init_workspace(DB_URL).await;
    let wp : Workspace<SqliteGraphDatabase> = Workspace::connect_workspace(DB_URL).await;
    //wp.add_dataset(GraphQuest::db_handler::data_loaders::Method::Stdin).await;

    wp.close_workspace().await;

    println!("Hello, world!");
}
