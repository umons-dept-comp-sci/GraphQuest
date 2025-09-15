use gquest_core::database_handler::{GraphDatabase, SqliteGraphDatabase};
use sqlx::migrate::MigrateDatabase;

const MEMORY_DB_URL: &str = "sqlite::memory:";
const PHYSICAL_DB_URL: &str = "sqlite:test.db";

const BAD_DB_URL: &str = "sqlite::bad_url";

#[tokio::test]
async fn connect_db_test_success() {
    let test =
        GraphDatabase::connect_graph_database(MEMORY_DB_URL, SqliteGraphDatabase::default(), None)
            .await
            .unwrap();

    sqlx::Any::drop_database(MEMORY_DB_URL).await.unwrap();
    test.close_connection().await;
}

#[tokio::test]
async fn connect_db_test_bad_url() {
    GraphDatabase::connect_graph_database(BAD_DB_URL, SqliteGraphDatabase::default(), None)
        .await
        .expect_err("Expected an error because the db doesn't exist");
}

#[tokio::test]
async fn create_db_test() {
    GraphDatabase::create_graph_database(PHYSICAL_DB_URL, SqliteGraphDatabase::default(), None)
        .await
        .expect("This was supposed to not cause an error");
    // Already exists
    GraphDatabase::create_graph_database(PHYSICAL_DB_URL, SqliteGraphDatabase::default(), None)
        .await
        .expect_err("This was supposed to cause an error");
    // Drop the created db
    sqlx::Any::drop_database(PHYSICAL_DB_URL).await.unwrap();
}

#[tokio::test]
async fn add_table_test() {
    let mut test =
    GraphDatabase::connect_graph_database(MEMORY_DB_URL, SqliteGraphDatabase::default(), None)
        .await
        .unwrap();

    test.add_signature_table().await;
}