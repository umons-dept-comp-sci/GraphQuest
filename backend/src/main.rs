use gquest_core::{data_handler::{self, invariant_handlers::{Invariant, InvariantsOrderHandler}}, db_handler::{graph_database::*, sqlite_handler::*}};




const DB_URL: &str = "resources/test.db";



#[tokio::main(flavor = "current_thread")]
async fn main() {
    
    //let wp: Workspace<SqliteGraphDatabase> = Workspace::init_workspace(DB_URL).await;
    //let wp : Workspace<SqliteGraphDatabase> = Workspace::connect_workspace(DB_URL).await;
    //wp.add_dataset(GraphQuest::db_handler::data_loaders::Method::Stdin).await;
    //wp.add_dataset<O>(data_handler::data_loaders::Method::GengAPI { nb_of_vertices: 6, graph_settings: "-c".to_string(), edges_born: (Some(4), None)}, None).await;
    //wp.close_workspace().await;
    let mut inv_handler = InvariantsOrderHandler::new();
    inv_handler.add_invariant(Invariant::new(&String::from("resources/invariant_modules/alpha.py"), 
                              &String::from("alpha"), 
                              vec![String::from("beta")],
                              None,
                              None,
                              None
                              ));   

    inv_handler.add_invariant(Invariant::new(&String::from("resources/invariant_modules/beta.py"), 
                              &String::from("beta"), 
                              vec![String::from("theta"), String::from("delta")], 
                              None,
                              None,
                              None
                              ));   

    inv_handler.add_invariant(Invariant::new(&String::from("resources/invariant_modules/delta.py"), 
                              &String::from("delta"), 
                              vec![], 
                              None,
                              None,
                              None
                              ));   

    inv_handler.add_invariant(Invariant::new(&String::from("resources/invariant_modules/theta.py"), 
                              &String::from("theta"), 
                              vec![],
                              None,
                              None ,
                              None
                              ));   
    //println!("oh damn: {:?}", InvariantsHandler::pretty_print_order(&inv_handler.get_topological_order()));

    let inv2 = InvariantsOrderHandler::read_json(&"resources/invariant_modules/dep.json".to_string());

    let inv_order = inv2.get_topological_order();
    println!("Second order: {:?}", inv_order.pretty_print_order());
    //exec_inv(&inv_order[0]);
    inv_order.handle_execution(5);
    //count_mutex::test();
    
}
