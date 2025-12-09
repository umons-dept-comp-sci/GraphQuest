use std::sync::Arc;

use sqlx::{Database, FromRow, migrate::MigrateDatabase};
use thiserror::Error;
use tokio::{
    sync::Mutex,
    task::{JoinError, JoinSet},
};

use crate::{
    data_handler::invariant_execs::{ExecutableIterator, ExecutableSorter, InvariantError},
    database_handler::{
        DbQuerySystem, ExtremalCounterExampleQuery, GraphDatabase, GraphDbRuntimeError,
        GraphDbStartupError, SqlSelectQuery, VERTICES_TABLE_NAME,
    },
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
    #[error("One of the invariant thread did not end correctly: \"{0}\"")]
    JoinError(#[from] JoinError),
}

pub struct Workplace<DB: Database + DbQuerySystem<DB>> {
    pub db: GraphDatabase<DB>,
    config: ConfigFile,
}

impl<DB: Database + DbQuerySystem<DB>> Workplace<DB>
where
    DB: Send,
    DB: MigrateDatabase,
    // Allow column indexing using usize
    usize: Send + Unpin + sqlx::ColumnIndex<DB::Row>,
    // Allow decoding/encoding
    String: sqlx::Encode<'static, DB>,
    for<'a> String: sqlx::Decode<'a, DB>,
    for<'a> i64: sqlx::Decode<'a, DB>,
    for<'a> f64: sqlx::Decode<'a, DB>,
    // Type of values
    String: sqlx::Type<DB>,
    i64: sqlx::Type<DB>,
    f64: sqlx::Type<DB>,
    // Return values
    (i64,): Send + Unpin + for<'a> FromRow<'a, DB::Row>,
    (String,): Send + Unpin + for<'a> FromRow<'a, DB::Row>,
    (String, f64): Send + Unpin + for<'a> FromRow<'a, DB::Row>,
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

    /// Executes all stored invariants using the given settings from the [`ConfigFile`].
    /// Does not impose any condition on the graph used for the computations.
    pub async fn execute_all_executables(&mut self) -> Result<(), WorkplaceError> {
        self.execute_invariant_execs(self.config.get_execs_ref().clone().try_into()?, None)
            .await
    }

    /// Executes the given executables stored inside the sorter using the given settings from the [`ConfigFile`].
    pub async fn execute_invariant_execs(
        &mut self,
        sorter: ExecutableSorter,
        add_condition: Option<SqlSelectQuery>,
    ) -> Result<(), WorkplaceError> {
        let mut sorted = sorter.to_iter()?;
        // If allowed, use multiple threads to compute the invariants
        if self.config.get_nb_threads() > 1 {
            self.execute_all_executables_multithread(sorted, add_condition)
                .await
        } else {
            while let Some(next_inv) = sorted.next_invariant() {
                self.db
                    .compute_executable(
                        &next_inv,
                        add_condition.clone(),
                        self.config.get_batch_size(),
                        None,
                    )
                    .await?;

                sorted.update_dependencies(&next_inv);
            }
            Ok(())
        }
    }

    async fn execute_all_executables_multithread(
        &mut self,
        sorted: ExecutableIterator,
        add_condition: Option<SqlSelectQuery>,
    ) -> Result<(), WorkplaceError> {
        let iter = Arc::new(Mutex::new(sorted));

        let mut handles: JoinSet<Result<(), GraphDbRuntimeError>> = JoinSet::new();
        while !iter.lock().await.is_finished() {
            // Spawns as many threads as possible while respecting the maximum number of threads to use
            while let Some(next_inv) = iter.lock().await.next_invariant()
                && handles.len() < self.config.get_nb_threads()
            {
                let mut db_clone = self.db.clone();
                let batch_size = self.config.get_batch_size();
                let add_condition_clone = add_condition.clone();
                let iter_clone = iter.clone();
                // A thread will compute the given invariant then end
                handles.spawn(async move {
                    db_clone
                        .compute_executable(&next_inv, add_condition_clone, batch_size, None)
                        .await?;
                    // Update the graph so that the main thread can compute new invariants
                    iter_clone.lock().await.update_dependencies(&next_inv);
                    Ok(())
                });
            }
            // Wait for any of them to finish then execute new ones
            handles.join_next().await.expect("joinSet is not empty")??; // checks for JoinError then for the GraphRuntimeError
        }
        // wait for all threads to finish and checks for any errors
        for handle in handles.join_all().await {
            handle?
        }

        Ok(())
    }

    /// Tries to find a counter example to a conjecture using this workplace.
    pub async fn find_counterexamples(
        &mut self,
        conjecture: ExtremalCounterExampleQuery,
    ) -> Result<Option<QueryTable>, WorkplaceError> {
        // Fully compute the necessary invariants (and their dependencies)
        let mut inv_to_compute = conjecture.get_invariants_to_compute();
        // This table is used but is not part of any executable
        inv_to_compute.remove(VERTICES_TABLE_NAME); // FIXME: Probably remove the vertices table all together :/
        if !inv_to_compute.is_empty() {
            let invariant_necessary =
                ExecutableSorter::new_from(self.config.get_execs_ref(), &inv_to_compute)?;
            // Compute them
            self.execute_invariant_execs(invariant_necessary, None)
                .await?;
        }

        // Only compute the necessary invariants in order to disprove the conjecture
        let mut conjecture_invariants = conjecture.get_invariants_from_conjecture();
        conjecture_invariants.remove(VERTICES_TABLE_NAME);
        if !conjecture_invariants.is_empty() {
            let invariant_necessary =
                ExecutableSorter::new_from(self.config.get_execs_ref(), &conjecture_invariants)?;
            // Get additional extremal condition :

            let extremal_condition = conjecture.get_invariant_input_selection();
            self.execute_invariant_execs(invariant_necessary, Some(extremal_condition))
                .await?;
        }

        // Return counter example query result
        let query: SqlSelectQuery = conjecture.into();

        Ok(self
            .db
            .fetch_all_row_query(&query, crate::utils::table_handler::QueryTableOptions::Full)
            .await?)
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

/*

THis doesn't work : too slow :
SELECT Dataset.*
    FROM (Dataset),
        (SELECT * FROM (eci INNER JOIN vertices USING (canon) INNER JOIN d_nm USING (canon) INNER JOIN m USING (canon))) as all_inv,
        (SELECT vertices, m, MAX(eci) as eci FROM (eci INNER JOIN vertices USING (canon) INNER JOIN m USING (canon)) GROUP BY vertices, m) as extremal
    WHERE ((all_inv.eci = extremal.eci AND all_inv.m = extremal.m AND all_inv.eci = extremal.eci AND all_inv.vertices = extremal.vertices)
            AND ((d_nm >= 3)))
        AND Dataset.canon = all_inv.canon
        AND (NOT (EXISTS(SELECT canon FROM (m) WHERE m.canon = Dataset.canon)));

SELECT all_inv.* FROM (SELECT * FROM (eci INNER JOIN vertices USING (canon) INNER JOIN d_nm USING (canon) INNER JOIN m USING (canon))) as all_inv, (SELECT vertices, m, MAX(eci) as eci FROM (eci INNER JOIN vertices USING (canon) INNER JOIN m USING (canon)) GROUP BY vertices, m) as extremal WHERE ((all_inv.eci = extremal.eci AND all_inv.m = extremal.m AND all_inv.eci = extremal.eci AND all_inv.vertices = extremal.vertices) AND ((d_nm >= 3))) AND (NOT (EXISTS(SELECT canon FROM (m) WHERE m.canon = all_inv.canon)));
*/
