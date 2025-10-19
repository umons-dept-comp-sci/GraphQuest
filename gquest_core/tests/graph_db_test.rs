use gquest_core::database_handler::{GraphDatabase, GraphDatabaseError, SqliteGraphDatabase};
use sqlx::migrate::MigrateDatabase;

const MEMORY_DB_URL: &str = "sqlite::memory:";
const PHYSICAL_DB_URL: &str = "sqlite:test.db";
// const FUNNY: &str = "sqlite:/home/axel/Téléchargements/chinook.db";

const BAD_DB_URL: &str = "sqlite::bad_url";

#[tokio::test]
async fn connect_db_test_success() {
    let test = GraphDatabase::<SqliteGraphDatabase>::connect_graph_database(MEMORY_DB_URL, None)
        .await
        .unwrap();

    sqlx::Any::drop_database(MEMORY_DB_URL).await.unwrap();
    test.close_connection().await;
}

#[tokio::test]
async fn connect_db_test_bad_url() {
    GraphDatabase::<SqliteGraphDatabase>::connect_graph_database(BAD_DB_URL, None)
        .await
        .expect_err("Expected an error because the db doesn't exist");
}

#[tokio::test]
async fn create_db_test() {
    GraphDatabase::<SqliteGraphDatabase>::create_graph_database(PHYSICAL_DB_URL, None)
        .await
        .expect("This was supposed to not cause an error");
    // Already exists
    assert!(matches!(
        GraphDatabase::<SqliteGraphDatabase>::create_graph_database(PHYSICAL_DB_URL, None).await,
        Err(GraphDatabaseError::DatabaseAlreadyCreated { .. })
    ));

    // Drop the created db
    sqlx::Any::drop_database(PHYSICAL_DB_URL).await.unwrap();
}

#[tokio::test]
async fn print_all_db_table_test() {
    let test =
        GraphDatabase::<SqliteGraphDatabase>::connect_create_graph_database(MEMORY_DB_URL, None)
            .await
            .unwrap();

    test.print_all_tables().await.expect("No errors");
}

/// This *test* is used to remove any database that could have failed
#[tokio::test]
async fn remove_all_created_df() {
    // Install sqlite, postgre and mysql drivers
    sqlx::any::install_default_drivers();

    // Drop the created db
    let _ = sqlx::Any::drop_database(PHYSICAL_DB_URL).await;
    let _ = sqlx::Any::drop_database(BAD_DB_URL).await;
    let _ = sqlx::Any::drop_database(MEMORY_DB_URL).await;
}
