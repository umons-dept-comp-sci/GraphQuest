use sqlx::{Database, FromRow, migrate::MigrateDatabase};
use thiserror::Error;

use crate::{
    data_handler::invariant_execs::{ExecutableSorter, InvariantError, InvariantsExecutable},
    database_handler::{DbQuerySystem, GraphDatabase, GraphDbRuntimeError},
    utils::{config_file::ConfigFile, table_handler::QueryTable},
};

#[derive(Debug, Error)]
pub enum WorkplaceError {
    #[error("Encountered an error from the database : \"{0}\"")]
    GraphDatabaseError(#[from] GraphDbRuntimeError),
    #[error("Encountered an error from an invariant executable : \"{0}\"")]
    InvariantError(#[from] InvariantError),
}

pub struct Workplace<DB: Database + DbQuerySystem<DB>> {
    _name: Option<String>,
    pub db: GraphDatabase<DB>,
    execs: Vec<InvariantsExecutable>,
    _nb_threads: usize,
}

impl<DB: Database + DbQuerySystem<DB>> Workplace<DB>
where
    DB: MigrateDatabase,
    usize: sqlx::ColumnIndex<DB::Row>,
    String: sqlx::Encode<'static, DB>,
    for<'a> String: sqlx::Decode<'a, DB>,
    String: sqlx::Type<DB>,
    i16: sqlx::Decode<'static, DB>,
    i16: sqlx::Type<DB>,
    (i16,): Send + Unpin + for<'a> FromRow<'a, DB::Row>,
    for<'a> i64: sqlx::Decode<'a, DB>,
    i64: sqlx::Type<DB>,
    (i64,): Send + Unpin + for<'a> FromRow<'a, DB::Row>,
    (String,): Send + Unpin + for<'a> FromRow<'a, DB::Row>,
    (String, i16): Send + Unpin + for<'a> FromRow<'a, DB::Row>,
{
    pub fn new(db: GraphDatabase<DB>, configs: ConfigFile) -> Self {
        Self {
            _name: None,
            db,
            execs: configs.executables,
            _nb_threads: 1,
        }
    }
    pub async fn init_workplace(_config: Option<ConfigFile>) -> Result<Self, WorkplaceError> {
        todo!()
    }

    /// Connects to the workspace using the given url
    pub async fn connect_workplace(
        _db_url: &str,
        _config: Option<ConfigFile>,
    ) -> Result<Self, WorkplaceError> {
        todo!()
    }

    pub async fn init_connect_workplace(
        _db_url: &str,
        _config: Option<ConfigFile>,
    ) -> Result<Self, WorkplaceError> {
        todo!()
    }

    /// Adds multiple executable to the workplace
    pub async fn add_executables(&mut self, mut executables: Vec<InvariantsExecutable>) {
        self.execs.append(&mut executables);
    }

    /// Properly closes the worspace
    pub async fn close_workspace(self) {
        self.db.close_connection().await;
    }

    pub async fn execute_all_executables(&mut self) -> Result<(), WorkplaceError> {
        // Sort all executables
        let sorter: ExecutableSorter = self.execs.clone().try_into()?;
        // Group them
        let man = sorter.group_execs()?;

        // Execute them by groups
        for group in man.get_groups() {
            for inv in group {
                self.db
                    .compute_executable(&inv, 1000)
                    .await
                    .expect("no errors");
            }
        }
        Ok(())
    }

    /// Gets a table that will summarize this workplace invariant computation progress.
    ///
    /// For example :
    /// ```b
    ///╭───┬────────────┬────────┬─────╮
    ///│ i │ Table Name │ Size   │ %   │
    ///├───┼────────────┼────────┼─────┤
    ///│ 0 │ Dataset    │ 288266 │ 100 │
    ///│ 1 │ size       │ 13598  │ 5   │
    ///│ 2 │ is_planar  │ 13598  │ 5   │
    ///│ 3 │ num_col    │ 3000   │ 1   │
    ///╰───┴────────────┴────────┴─────╯
    /// ```
    pub async fn summary(&self) -> QueryTable {
        todo!()
    }
}
