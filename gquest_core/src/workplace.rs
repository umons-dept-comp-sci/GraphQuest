use std::sync::Arc;

use log::info;
use sqlx::{FromRow, migrate::MigrateDatabase};
use thiserror::Error;
use tokio::{
    sync::Mutex,
    task::{JoinError, JoinSet},
};

use crate::{
    data_handler::invariant_execs::{ExecutableIterator, ExecutableSorter, InvariantError},
    database_handler::{
        ClassSelection, ExtremalCounterQuery, GraphDatabase, GraphDb, GraphDbRuntimeError,
        GraphDbStartupError, SqlCondition, SqlSelectQuery, VERTICES_TABLE_NAME, graph_queries,
    },
    utils::{SaveOutput, config_file::ConfigFile},
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

/// A struct used to facilitate more complicated operations involving both a configuration file ([`ConfigFile`]) and an open graph database ([`GraphDatabase`]).
pub struct Workplace<'a, DB: GraphDb> {
    pub db: &'a mut GraphDatabase<DB>,
    config: ConfigFile,
}

impl<'a, DB: GraphDb> Workplace<'a, DB>
where
    DB: MigrateDatabase,
    // Allow column indexing using usize
    usize: Send + Unpin + sqlx::ColumnIndex<DB::Row>,
    // Allow decoding/encoding
    f64: sqlx::Encode<'static, DB>,
    String: sqlx::Encode<'static, DB>,
    for<'q> String: sqlx::Decode<'q, DB>,
    for<'q> i64: sqlx::Decode<'q, DB>,
    for<'q> f64: sqlx::Decode<'q, DB>,
    for<'q> f32: sqlx::Decode<'q, DB>,
    for<'q> i32: sqlx::Decode<'q, DB>,
    // Type of values
    String: sqlx::Type<DB>,
    i64: sqlx::Type<DB>,
    f64: sqlx::Type<DB>,
    f32: sqlx::Type<DB>,
    // Return values
    (i64,): Send + Unpin + for<'q> FromRow<'q, DB::Row>,
    (i32,): Send + Unpin + for<'q> FromRow<'q, DB::Row>,
    (String,): Send + Unpin + for<'q> FromRow<'q, DB::Row>,
    (String, f64): Send + Unpin + for<'q> FromRow<'q, DB::Row>,
{
    /// Creates a new [`Workplace`].
    pub fn new(db: &'a mut GraphDatabase<DB>, config: ConfigFile) -> Self {
        Self { db, config }
    }

    /// Executes all stored invariants using the given settings from the [`ConfigFile`].
    /// Does not impose any condition on the graph used for the computations.
    pub async fn execute_all_executables(&mut self) -> Result<(), WorkplaceError> {
        self.execute_invariant_execs(
            self.config.get_execs_ref().clone().try_into()?,
            None,
            vec![],
        )
        .await
    }

    /// Executes the executables stored inside the sorter using the given settings from the [`ConfigFile`].
    pub async fn execute_invariant_execs(
        &mut self,
        sorter: ExecutableSorter,
        add_condition: Option<SqlSelectQuery>,
        inv_to_skip: Vec<String>,
    ) -> Result<(), WorkplaceError> {
        let mut sorted = sorter.to_iter()?;
        // If allowed, use multiple threads to compute the invariants
        if self.config.get_nb_threads() > 1 {
            info!(
                "Execute all modules using at most {} threads",
                self.config.get_nb_threads()
            );
            self.execute_all_executables_multithread(sorted, add_condition, inv_to_skip)
                .await
        } else {
            info!("Execute all modules using 1 thread");
            while let Some(next_inv) = sorted.next_invariant() {
                // Check if this invariant should be skipped
                if !next_inv
                    .invariant_names
                    .iter()
                    .all(|name| inv_to_skip.contains(name))
                {
                    info!("Doing invariants \"{:?}\"", next_inv.invariant_names);
                    self.db
                        .compute_executable(
                            &next_inv,
                            add_condition.clone(),
                            self.config.get_batch_size(),
                            None,
                        )
                        .await?;
                    info!(
                        "Finished doing invariants \"{:?}\"",
                        next_inv.invariant_names
                    );
                }

                sorted.update_dependencies(&next_inv);
            }
            Ok(())
        }
    }

    async fn execute_all_executables_multithread(
        &mut self,
        sorted: ExecutableIterator,
        add_condition: Option<SqlSelectQuery>,
        inv_to_skip: Vec<String>,
    ) -> Result<(), WorkplaceError> {
        let iter = Arc::new(Mutex::new(sorted));

        // Used to directly unlock the mutex after acquiring the next invariant
        let get_next_inv = &mut async |iter_lock: &Arc<Mutex<ExecutableIterator>>| {
            iter_lock.lock().await.next_invariant()
        };

        let mut handles: JoinSet<Result<(), GraphDbRuntimeError>> = JoinSet::new();
        while !iter.lock().await.is_finished() {
            // Spawns as many threads as possible while respecting the maximum number of threads to use
            while let Some(next_inv) = get_next_inv(&iter).await
                && handles.len() < self.config.get_nb_threads()
            {
                // Check if this invariant should be skipped
                if !next_inv
                    .invariant_names
                    .iter()
                    .all(|name| inv_to_skip.contains(name))
                {
                    info!(
                        "Starting a thread to do invariants \"{:?}\"",
                        next_inv.invariant_names
                    );
                    let mut db_clone = self.db.clone();
                    let batch_size = self.config.get_batch_size();
                    let add_condition_clone = add_condition.clone();
                    // let update_dep_clone = *update_dep;
                    let iter_clone = iter.clone();
                    // A thread will compute the given invariant then end
                    handles.spawn(async move {
                        db_clone
                            .compute_executable(&next_inv, add_condition_clone, batch_size, None)
                            .await?;
                        // Update the graph so that the main thread can compute new invariants
                        iter_clone.lock().await.update_dependencies(&next_inv);
                        info!(
                            "Thread finished doing invariants \"{:?}\"",
                            next_inv.invariant_names
                        );
                        Ok(())
                    });
                } else {
                    // Skip this invariant and move to another one directly
                    iter.lock().await.update_dependencies(&next_inv);
                }
            }
            // Wait for any of them to finish then execute new ones
            handles.join_next().await.expect("joinSet is not empty")??; // checks for JoinError then for the GraphRuntimeError
        }
        info!("Wait for all threads to finish");
        // wait for all threads to finish and checks for any errors
        for handle in handles.join_all().await {
            handle?
        }
        info!("Finished waiting for threads");
        Ok(())
    }

    pub async fn find_graphs_condition<O: SaveOutput>(
        &mut self,
        condition: SqlCondition,
        output: &mut O,
    ) -> Result<(), WorkplaceError> {
        let mut invariants = condition.get_all_identifiers();
        // This table is used but is not part of any executable
        invariants.remove(VERTICES_TABLE_NAME);

        if !invariants.is_empty() {
            let invariant_necessary =
                ExecutableSorter::new_from(self.config.get_execs_ref(), &invariants)?;
            // Compute them
            self.execute_invariant_execs(invariant_necessary, None, vec![])
                .await?;
        }

        let query = graph_queries::select_all_graph_cond(condition);

        self.db.fetch_all_row_query(&query, output).await?;

        Ok(())
    }

    pub async fn find_extremals_graphs<O: SaveOutput>(
        &mut self,
        extremal: ClassSelection,
        additional_condition: Option<SqlCondition>,
        output: &mut O,
    ) -> Result<(), WorkplaceError> {
        let mut extremal_inv =
            graph_queries::get_extremal_invariants(&extremal, &additional_condition);

        // This table is used but is not part of any executable
        extremal_inv.remove(VERTICES_TABLE_NAME); // FIXME: Probably remove the vertices table all together :/
        info!("Find extremal graphs of {:?}", extremal);

        if !extremal_inv.is_empty() {
            let invariant_necessary =
                ExecutableSorter::new_from(self.config.get_execs_ref(), &extremal_inv)?;
            // Compute them
            self.execute_invariant_execs(invariant_necessary, None, vec![])
                .await?;
        }

        // Get them
        let query = graph_queries::select_all_extremal_graphs(&extremal, &additional_condition);

        self.db.fetch_all_row_query(&query, output).await?;

        Ok(())
    }

    /// Tries to find a counter example to a conjecture using this workplace.
    pub async fn find_counterexamples<O: SaveOutput>(
        &mut self,
        conjecture: ExtremalCounterQuery,
        output: &mut O,
    ) -> Result<(), WorkplaceError> {
        // Fully compute the necessary invariants (and their dependencies)
        let mut inv_to_compute = conjecture.get_invariants_to_compute();
        // This table is used but is not part of any executable
        inv_to_compute.remove(VERTICES_TABLE_NAME); // FIXME: Probably remove the vertices table all together :/
        info!("Find extremal graphs of {:?}", conjecture.selection);
        info!(
            "Fully compute the following invariants: {:?}",
            inv_to_compute
        );
        if !inv_to_compute.is_empty() {
            let invariant_necessary =
                ExecutableSorter::new_from(self.config.get_execs_ref(), &inv_to_compute)?;
            // Compute them
            self.execute_invariant_execs(invariant_necessary, None, vec![])
                .await?;
        }
        info!("Finished computing extremal graphs, moving on to counterexample search");

        // Only compute the necessary invariants in order to disprove the conjecture
        let mut conjecture_invariants = conjecture.get_invariants_from_conjecture();
        conjecture_invariants.remove(VERTICES_TABLE_NAME);

        if !conjecture_invariants.is_empty() {
            // This automatically adds the dependencies of the conjecture invariants
            let invariant_necessary = ExecutableSorter::new_from(
                self.config.get_execs_ref(),
                &conjecture_invariants.difference(&inv_to_compute).collect(), // Remove already computed invariants
            )?;

            // Get additional extremal condition :
            let extremal_condition = conjecture.get_extremal_input_selection();
            info!(
                "Computing conjecture invariants : {conjecture_invariants:?} (skip if already done)"
            );
            self.execute_invariant_execs(
                invariant_necessary,
                Some(extremal_condition),
                Vec::from_iter(inv_to_compute), // No need to compute these already fully computed invariants
            )
            .await?;
            info!("Finished computing conjecture invariants");
        }

        info!("Try to fetch counterexample graphs");
        // Return counter example query result
        let query: SqlSelectQuery = conjecture.into();

        self.db.fetch_all_row_query(&query, output).await?;

        Ok(())
    }
}
