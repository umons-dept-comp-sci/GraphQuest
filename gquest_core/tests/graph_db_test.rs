use gquest_core::{
    data_handler::{data_loader::GengProcess, invariant_execs::InvariantsExecutable},
    database_handler::{
        CANONICAL_TABLE_NAME, GraphDbRuntimeError, GraphDbStartupError, SqlSelectQuery,
        SqliteGraphDB, VERTICES_TABLE_NAME,
    },
    utils::subject::Observer,
};
use sqlx::{Sqlite, migrate::MigrateDatabase};
use std::{io::Read, process::ChildStdout};

const MEMORY_DB_URL: &str = "sqlite::memory:";
const PHYSICAL_DB_URL: &str = "sqlite:test.db";

const EXEC_VERTICES: &str = "tests/modules/vertices.py";
const GENG_VERTICE_COUNT: u32 = 5;
const BAD_DB_URL: &str = "sqlite::bad_url";

// Simple observer that tracks how many times it was called and the given progression
#[derive(Debug)]
struct CustomObs {
    pub total_ticks: u64,
    pub progression: u64,
}

impl CustomObs {
    fn reset(&mut self) {
        self.total_ticks = 0;
        self.progression = 0;
    }
}

impl Observer for CustomObs {
    fn notify_tick(&mut self) {
        self.total_ticks += 1;
    }
    fn notify_data_pushed(&mut self, delta: u64) {
        self.progression += delta;
    }
}

#[tokio::test]
async fn connect_db_test_success() {
    let test = SqliteGraphDB::connect_graph_database(MEMORY_DB_URL, None)
        .await
        .unwrap();
    test.close_connection().await;
}

#[tokio::test]
async fn connect_db_test_bad_url() {
    remove_all_created_df().await;
    SqliteGraphDB::connect_graph_database(BAD_DB_URL, None)
        .await
        .expect_err("Expected an error because the db doesn't exist");
}

#[tokio::test]
async fn create_db_test() {
    // remove_all_created_df().await;
    SqliteGraphDB::create_graph_database(PHYSICAL_DB_URL, None)
        .await
        .expect("This was supposed to not cause an error");

    // Already exists
    assert!(matches!(
        SqliteGraphDB::create_graph_database(PHYSICAL_DB_URL, None).await,
        Err(GraphDbStartupError::DatabaseAlreadyCreated { .. })
    ));

    // Drop the created db
    Sqlite::drop_database(PHYSICAL_DB_URL).await.unwrap();
}

#[tokio::test]
async fn print_all_db_table_test() {
    // remove_all_created_df().await;
    let _test = SqliteGraphDB::connect_create_graph_database(MEMORY_DB_URL, None)
        .await
        .unwrap();

    // test.print_all_tables().await.expect("No errors");
}

#[tokio::test]
async fn add_to_dataset_test() {
    let mut test = SqliteGraphDB::connect_create_graph_database(MEMORY_DB_URL, None)
        .await
        .unwrap();
    let (expected, geng_reader) = get_geng_values();

    test.add_to_dataset(geng_reader, 1000, None)
        .await
        .expect("no issues");

    let dataset_content = test.read_all_dataset().await.expect("correct results");

    assert_eq!(dataset_content.len(), expected.len());
    for signature in dataset_content {
        assert!(expected.contains(&signature))
    }
}

#[tokio::test]
async fn add_to_dataset_obs_test() {
    let mut obs = CustomObs {
        progression: 0,
        total_ticks: 0,
    };

    let mut test = SqliteGraphDB::connect_create_graph_database(MEMORY_DB_URL, None)
        .await
        .unwrap();
    let (expected, geng_reader) = get_geng_values();

    test.add_to_dataset(geng_reader, 1000, Some(&mut obs))
        .await
        .expect("no issues");

    assert_eq!(obs.progression, expected.len() as u64);
    assert_eq!(obs.total_ticks, expected.len() as u64);
}

#[tokio::test]
async fn is_table_added_test() {
    let mut test = SqliteGraphDB::connect_create_graph_database(MEMORY_DB_URL, None)
        .await
        .unwrap();

    let (_, geng_reader) = get_geng_values();

    test.add_to_dataset(geng_reader, 1000, None)
        .await
        .expect("no issues");

    assert!(
        test.is_table_added(CANONICAL_TABLE_NAME)
            .await
            .expect("no issues")
    );

    assert!(!test.is_table_added("Fake table").await.expect("no issues"));
}

#[tokio::test]
async fn get_size_of_table_test() {
    let mut test = SqliteGraphDB::connect_create_graph_database(MEMORY_DB_URL, None)
        .await
        .unwrap();
    let (expected, geng_reader) = get_geng_values();

    test.add_to_dataset(geng_reader, 1000, None)
        .await
        .expect("no issues");

    assert_eq!(
        expected.len(),
        test.get_size_of_table(CANONICAL_TABLE_NAME)
            .await
            .expect("no issues")
    );
}

#[tokio::test]
async fn compute_executable_test() {
    let mut db_test = SqliteGraphDB::connect_create_graph_database(MEMORY_DB_URL, None)
        .await
        .expect("no issues with db init");

    // Get invariant :
    let identity =
        InvariantsExecutable::new_no_dep(EXEC_VERTICES, vec!["ident"]).expect("correct inv");

    // This should fail since no dataset were initialised at first
    assert!(matches!(
        db_test.compute_executable(&identity, None, 100, None).await,
        Err(GraphDbRuntimeError::DatasetNotInitialisedError(_))
    ));

    // Create dataset
    let (expected, geng_reader) = get_geng_values();

    db_test
        .add_to_dataset(geng_reader, 1000, None)
        .await
        .expect("no issues");

    // Then execute without any troubles
    db_test
        .compute_executable(&identity, None, 100, None)
        .await
        .expect("No errors");

    // Check db content
    assert!(db_test.is_table_added("ident").await.expect("no errors"));
    assert_eq!(
        expected.len(),
        db_test.get_size_of_table("ident").await.expect("no errors")
    );
    // Fetch all values and compare them to expected

    let values = db_test.read_all_table("ident").await.expect("No issues");
    for (_, value) in values {
        assert_eq!(value, GENG_VERTICE_COUNT as f64)
    }
}

#[tokio::test]
async fn compute_executable_obs_test() {
    let mut db_test = SqliteGraphDB::connect_create_graph_database(MEMORY_DB_URL, None)
        .await
        .expect("no issues with db init");

    // Get invariant :
    let identity =
        InvariantsExecutable::new_no_dep(EXEC_VERTICES, vec!["ident"]).expect("correct inv");

    // Create dataset
    let (expected, geng_reader) = get_geng_values();

    // Observer
    let mut obs = CustomObs {
        progression: 0,
        total_ticks: 0,
    };

    db_test
        .add_to_dataset(geng_reader, 1000, None)
        .await
        .expect("no issues");

    // Then execute without any troubles
    db_test
        .compute_executable(&identity, None, 100, Some(&mut obs))
        .await
        .expect("No errors");

    assert_eq!(obs.total_ticks, expected.len() as u64);
    assert_eq!(obs.progression, expected.len() as u64);
}

#[tokio::test]
async fn compute_executable_with_selection_test() {
    let expected_len = 5;

    let mut db_test = SqliteGraphDB::connect_create_graph_database(MEMORY_DB_URL, None)
        .await
        .expect("no issues with db init");

    // Get invariant :
    let identity =
        InvariantsExecutable::new_no_dep(EXEC_VERTICES, vec!["ident"]).expect("correct inv");

    // Create dataset
    let geng_reader = get_geng_values().1;

    db_test
        .add_to_dataset(geng_reader, 1000, None)
        .await
        .expect("no issues");

    // This query restricts the total amount of canonical
    let query = SqlSelectQuery::select_all_from_table(CANONICAL_TABLE_NAME)
        .set_limit_clause(None, expected_len);

    // Then execute without any troubles
    db_test
        .compute_executable(&identity, Some(query), 100, None)
        .await
        .expect("No errors");

    // Check db content
    assert!(db_test.is_table_added("ident").await.expect("no errors"));
    assert_eq!(
        expected_len,
        db_test.get_size_of_table("ident").await.expect("no errors")
    );

    /* Try a query with no PK column (no signature column) */

    // This query has NO canonical signatures
    let query = SqlSelectQuery::select_column_from_table(VERTICES_TABLE_NAME, VERTICES_TABLE_NAME)
        .set_limit_clause(None, expected_len);

    assert!(
        (db_test
            .compute_executable(&identity, Some(query), 100, None)
            .await)
            .is_err()
    );
}

#[tokio::test]
async fn compute_executable_no_duplicate() {
    let first_batch_size = 5;
    // Observer
    let mut obs = CustomObs {
        progression: 0,
        total_ticks: 0,
    };
    // This query restricts the total amount of data sent
    let query = SqlSelectQuery::select_all_from_table(CANONICAL_TABLE_NAME)
        .set_limit_clause(None, first_batch_size);

    let mut db_test = SqliteGraphDB::connect_create_graph_database(MEMORY_DB_URL, None)
        .await
        .expect("no issues with db init");

    // Get invariant :
    let identity =
        InvariantsExecutable::new_no_dep(EXEC_VERTICES, vec!["ident"]).expect("correct inv");

    // Create dataset
    let (expected, geng_reader) = get_geng_values();

    db_test
        .add_to_dataset(geng_reader, 1000, None)
        .await
        .expect("no issues");

    // Then execute without any troubles
    db_test
        .compute_executable(&identity, Some(query), 100, Some(&mut obs))
        .await
        .expect("No errors");

    assert_eq!(first_batch_size, obs.progression as usize);
    // Reset obs
    obs.reset();

    // Compute the rest, which SHOULD NOT include the five first computed values
    db_test
        .compute_executable(&identity, None, 100, Some(&mut obs))
        .await
        .expect("No errors");

    assert_eq!(expected.len() - first_batch_size, obs.progression as usize);
    
}

async fn remove_all_created_df() {
    // Install sqlite, postgre and mysql drivers
    // sqlx::any::install_default_drivers();

    // Drop the created db
    let _ = Sqlite::drop_database(PHYSICAL_DB_URL).await;
    let _ = Sqlite::drop_database(BAD_DB_URL).await;
    let _ = Sqlite::drop_database(MEMORY_DB_URL).await;
}

fn get_geng_values() -> (Vec<String>, std::io::BufReader<ChildStdout>) {
    let geng = GengProcess::call_geng(GENG_VERTICE_COUNT, &"".to_string(), (None, None))
        .expect("correct call");
    let mut res: String = String::default();
    geng.get_reader().read_to_string(&mut res).expect("correct");
    let expected_res: Vec<String> = res
        .split_ascii_whitespace()
        .map(|f| f.to_string())
        .collect();

    let geng = GengProcess::call_geng(GENG_VERTICE_COUNT, &"".to_string(), (None, None))
        .expect("correct call");
    let reader: std::io::BufReader<ChildStdout> = geng.get_reader();

    (expected_res, reader)
}
