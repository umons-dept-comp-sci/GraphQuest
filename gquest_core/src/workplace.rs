use std::sync::Arc;

use sqlx::{Database, FromRow, migrate::MigrateDatabase};
use thiserror::Error;
use tokio::{sync::Mutex, task::JoinSet};

use crate::{
    data_handler::invariant_execs::{ExecutableIterator, ExecutableSorter, InvariantError},
    database_handler::{DbQuerySystem, GraphDatabase, GraphDbRuntimeError, GraphDbStartupError},
    utils::{config_file::ConfigFile, table_handler::QueryTable},
};

#[derive(Debug, Error)]
pub enum WorkplaceError {
    #[error("Encountered an error from the database during initialisation : \"{0}\"")]
    GraphDbStartupError(#[from] GraphDbStartupError),
    #[error("Encountered an error from the database during an execution : \"{0}\"")]
    GraphDbRuntimeError(#[from] GraphDbRuntimeError),
    #[error("Encountered an error from an invariant executable : \"{0}\"")]
    InvariantError(#[from] InvariantError),
}

pub struct Workplace<DB: Database + DbQuerySystem<DB>> {
    pub db: GraphDatabase<DB>,
    config: ConfigFile,
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
    pub fn new(db: GraphDatabase<DB>, config: ConfigFile) -> Self {
        Self { db, config }
    }
    pub async fn init_workplace(
        uri: impl ToString,
        config: ConfigFile,
    ) -> Result<Self, WorkplaceError> {
        let db = GraphDatabase::connect_create_graph_database(uri, None).await?;

        Ok(Self::new(db, config))
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

    /// Properly closes the worspace
    pub async fn close_workspace(self) {
        self.db.close_connection().await;
    }

    /// Executes all stored invariants
    pub async fn execute_all_executables(&mut self) -> Result<(), WorkplaceError> {
        // Sort all executables
        let sorter: ExecutableSorter = self.config.get_execs_ref().clone().try_into()?;

        let mut sorted = sorter.to_iter().expect("ok");

        if self.config.get_nb_threads() > 1 {
            self.execute_all_executables_multithread(sorted).await
        } else {
            while let Some(next_inv) = sorted.next_invariant() {
                self.db
                    .compute_executable(&next_inv, self.config.get_batch_size(), None)
                    .await?;

                sorted.update_dependencies(&next_inv);
            }
            Ok(())
        }
    }

    async fn execute_all_executables_multithread(
        &mut self,
        sorted: ExecutableIterator,
    ) -> Result<(), WorkplaceError> {
        let iter = Arc::new(Mutex::new(sorted));

        let mut handles = JoinSet::new();
        while !iter.lock().await.is_finished() {
            while let Some(next_inv) = iter.lock().await.next_invariant()
                && handles.len() < self.config.get_nb_threads()
            {
                let mut db_clone = self.db.clone();
                let batch_size = self.config.get_batch_size();
                let iter_clone = iter.clone();
                // A thread will try to compute as much executable as possible without ending until
                handles.spawn(async move {
                    // This loop is written like this to prevent any deadlock by always releasing the lock after aquiring the next exec

                    db_clone
                        .compute_executable(&next_inv, batch_size, None)
                        .await
                        .expect("no errors");
                    // Signal that we finished with this dependency
                    iter_clone.lock().await.update_dependencies(&next_inv);
                });
            }
            // Wait for any of them to finish
            handles.join_next().await.expect("no problem").expect("sds"); // TODO: Collect error :)
        }
        let _res = handles.join_all().await;

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
