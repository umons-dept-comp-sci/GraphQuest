use gquest_core::{data_handler::{self, invariant_handlers::{InvariantsExecutable, InvariantsOrderHandler}}, db_handler::{graph_database::*, sqlite_handler::*}};

const THREADS_AVAILABLE: usize = 4;


const DB_URL: &str = "resources/test.db";


#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    
    //let wp: Workspace<SqliteGraphDatabase> = Workspace::init_workspace(DB_URL).await;
    //let wp1 : Workspace<SqliteGraphDatabase> = Workspace::connect_workspace(DB_URL).await;
    
    let wp2 : Workspace<SqliteGraphDatabase> = Workspace::connect_workspace(DB_URL).await;
    //wp.add_dataset(GraphQuest::db_handler::data_loaders::Method::Stdin).await;
    //wp.add_dataset<O>(data_handler::data_loaders::Method::GengAPI { nb_of_vertices: 6, graph_settings: "-c".to_string(), edges_born: (Some(4), None)}, None).await;
    //wp.close_workspace().await;  
    let inv2 = InvariantsOrderHandler::read_json(&"resources/invariant_modules/dep.json".to_string());
    
    println!("oh damn: {:?}", &inv2.get_topological_order().pretty_print_order());

    //let inv_order = inv2.get_topological_order();
    //println!("Second order: {:?}", inv_order.pretty_print_order());
    
    //exec_inv(&inv_order[0]);
    //wp2.compute_invariants(inv2, THREADS_AVAILABLE).await;
    
    //count_mutex::test();
    //wp2.test().await;
    //let x = InvariantsExecutable::new(&String::from("resources/invariant_modules/theta.py"), 
    //                          vec![String::from("alpha1"), String::from("alpha2")], 
    //                          vec![],
    //                          None,
    //                          None ,
    //                          None
    //                          );
    //wp2.test(&x).await;
}
