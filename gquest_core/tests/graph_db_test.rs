use gquest_core::{
    data_handler::data_loader::GengProcess,
    database_handler::{
        GraphDatabase, GraphDbStartupError, SqliteQuerySystem, CANONICAL_TABLE_NAME,
    },
};
use sqlx::migrate::MigrateDatabase;
use std::{
    io::{BufRead, Lines, Read},
    process::ChildStdout,
};

const MEMORY_DB_URL: &str = "sqlite::memory:";
const PHYSICAL_DB_URL: &str = "sqlite:test.db";

const BAD_DB_URL: &str = "sqlite::bad_url";

#[tokio::test]
async fn connect_db_test_success() {
    remove_all_created_df().await;
    let test = GraphDatabase::<SqliteQuerySystem>::connect_graph_database(MEMORY_DB_URL, None)
        .await
        .unwrap();

    sqlx::Any::drop_database(MEMORY_DB_URL).await.unwrap();
    test.close_connection().await;
}

#[tokio::test]
async fn connect_db_test_bad_url() {
    remove_all_created_df().await;
    GraphDatabase::<SqliteQuerySystem>::connect_graph_database(BAD_DB_URL, None)
        .await
        .expect_err("Expected an error because the db doesn't exist");
}

#[tokio::test]
async fn create_db_test() {
    remove_all_created_df().await;
    GraphDatabase::<SqliteQuerySystem>::create_graph_database(PHYSICAL_DB_URL, None)
        .await
        .expect("This was supposed to not cause an error");
    // Already exists
    assert!(matches!(
        GraphDatabase::<SqliteQuerySystem>::create_graph_database(PHYSICAL_DB_URL, None).await,
        Err(GraphDbStartupError::DatabaseAlreadyCreated { .. })
    ));

    // Drop the created db
    sqlx::Any::drop_database(PHYSICAL_DB_URL).await.unwrap();
}

#[tokio::test]
async fn print_all_db_table_test() {
    remove_all_created_df().await;
    let test =
        GraphDatabase::<SqliteQuerySystem>::connect_create_graph_database(MEMORY_DB_URL, None)
            .await
            .unwrap();

    test.print_all_tables().await.expect("No errors");
}

#[tokio::test]
async fn add_to_dataset_test() {
    remove_all_created_df().await;
    let mut test =
        GraphDatabase::<SqliteQuerySystem>::connect_create_graph_database(PHYSICAL_DB_URL, None)
            .await
            .unwrap();
    let (expected, geng_reader) = get_geng_values();

    test.add_to_dataset(geng_reader, 1000)
        .await
        .expect("no issues");

    let dataset_content = test.read_all_dataset().await.expect("correct results");

    assert_eq!(dataset_content.len(), expected.len());
    for (signature, _) in dataset_content {
        assert!(expected.contains(&signature))
    }
}

#[tokio::test]
async fn is_table_added_test() {
    remove_all_created_df().await;
    let test =
        GraphDatabase::<SqliteQuerySystem>::connect_create_graph_database(PHYSICAL_DB_URL, None)
            .await
            .unwrap();

    assert!(test
        .is_table_added(CANONICAL_TABLE_NAME)
        .await
        .expect("no issues"));

    assert!(!test.is_table_added("Fake table").await.expect("no issues"));
}

#[tokio::test]
async fn get_size_of_table_test() {
    remove_all_created_df().await;
    let mut test =
        GraphDatabase::<SqliteQuerySystem>::connect_create_graph_database(PHYSICAL_DB_URL, None)
            .await
            .unwrap();
    let (expected, geng_reader) = get_geng_values();

    test.add_to_dataset(geng_reader, 1000)
        .await
        .expect("no issues");

    assert_eq!(
        expected.len(),
        test.get_size_of_table(CANONICAL_TABLE_NAME)
            .await
            .expect("no issues")
    );
}

async fn remove_all_created_df() {
    // Install sqlite, postgre and mysql drivers
    sqlx::any::install_default_drivers();

    // Drop the created db
    let _ = sqlx::Any::drop_database(PHYSICAL_DB_URL).await;
    let _ = sqlx::Any::drop_database(BAD_DB_URL).await;
    let _ = sqlx::Any::drop_database(MEMORY_DB_URL).await;
}

fn get_geng_values() -> (Vec<String>, std::io::BufReader<ChildStdout>) {
    let geng = GengProcess::call_geng(5, &"".to_string(), (None, None)).expect("correct call");
    let mut res: String = String::default();
    geng.get_reader().read_to_string(&mut res).expect("correct");
    let expected_res: Vec<String> = res
        .split_ascii_whitespace()
        .map(|f| f.to_string())
        .collect();

    let geng = GengProcess::call_geng(5, &"".to_string(), (None, None)).expect("correct call");
    let reader: std::io::BufReader<ChildStdout> = geng.get_reader();

    (expected_res, reader)
}
