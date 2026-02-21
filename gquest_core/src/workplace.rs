use std::sync::Arc;

use indexmap::IndexSet;
use log::info;
use thiserror::Error;
use tokio::{
    sync::Mutex,
    task::{JoinError, JoinSet},
};

use crate::{
    data_handler::invariant_execs::{Module, ModuleError, ModuleIterator, ModuleSorter},
    database_handler::{
        AllowedGraphDb, ClassSelection, ExtremalConjecture, GraphDbRuntimeError,
        GraphDbStartupError, PK_NAME, SqlCondition, SqlSelectQuery, SqlTableSelection,
        VERTICES_TABLE_NAME, graph_queries,
    },
    utils::{SaveOutput, config_file::ConfigFile, subject::Observer},
};

#[derive(Debug, Error)]
pub enum WorkplaceError {
    #[error("Encountered an error from the database during initialisation : \"{0}\"")]
    GraphDbStartupError(#[from] GraphDbStartupError),
    #[error("Encountered an error from the database during an execution : \"{0}\"")]
    GraphDbRuntimeError(#[from] GraphDbRuntimeError),
    #[error("Encountered an error from a module : \"{0}\"")]
    ModuleError(#[from] ModuleError),
    #[error("One of the invariant thread did not end correctly: \"{0}\"")]
    JoinError(#[from] JoinError),
}

/// A struct used to facilitate more complicated operations involving both a configuration file ([`ConfigFile`]) and an open graph database ([`GraphDatabase`]).
pub struct Workplace {
    pub db: AllowedGraphDb,
    config: ConfigFile,
}

impl Workplace {
    /// Creates a new [`Workplace`].
    pub fn new(db: impl Into<AllowedGraphDb>, config: ConfigFile) -> Self {
        let db = db.into();

        Self { db, config }
    }

    /// Executes all stored invariants using the given settings from the [`ConfigFile`].
    /// Does not impose any condition on the graph used for the computations.
    pub async fn execute_all_modules(&mut self) -> Result<(), WorkplaceError> {
        self.execute_invariant_modules(
            self.config.get_execs_ref().clone().try_into()?,
            None,
            vec![],
        )
        .await
    }

    /// Executes the modules stored inside the sorter using the given settings from the [`ConfigFile`].
    pub async fn execute_invariant_modules(
        &mut self,
        sorter: ModuleSorter,
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
            self.execute_all_modules_multithread(sorted, add_condition, inv_to_skip)
                .await
        } else {
            info!("Execute all modules using 1 thread");
            while let Some(next_inv) = sorted.next_module() {
                // Check if this invariant should be skipped
                if !next_inv
                    .invariant_names
                    .iter()
                    .all(|name| inv_to_skip.contains(name))
                {
                    info!("Doing invariants \"{:?}\"", next_inv.invariant_names);
                    compute_module(
                        &mut self.db,
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

    async fn execute_all_modules_multithread(
        &mut self,
        sorted: ModuleIterator,
        add_condition: Option<SqlSelectQuery>,
        inv_to_skip: Vec<String>,
    ) -> Result<(), WorkplaceError> {
        let iter = Arc::new(Mutex::new(sorted));

        // Used to directly unlock the mutex after acquiring the next invariant
        let get_next_inv = &mut async |iter_lock: &Arc<Mutex<ModuleIterator>>| {
            iter_lock.lock().await.next_module()
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
                        compute_module(
                            &mut db_clone,
                            &next_inv,
                            add_condition_clone,
                            batch_size,
                            None,
                        )
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

    pub async fn query_condition<O: SaveOutput>(
        &mut self,
        condition: SqlCondition,
        output: &mut O,
    ) -> Result<(), WorkplaceError> {
        let invariants = condition.get_all_identifiers();
        self.compute_invariants(invariants, vec![]).await?;

        let query = graph_queries::select_all_graph_cond(condition);

        self.fetch_all_row_query(&query, output).await?;

        Ok(())
    }

    pub async fn find_extremals_graphs<O: SaveOutput>(
        &mut self,
        extremal: ClassSelection,
        additional_condition: Option<SqlCondition>,
        output: &mut O,
    ) -> Result<(), WorkplaceError> {
        let extremal_inv = graph_queries::get_extremal_invariants(&extremal, &additional_condition);

        self.compute_invariants(extremal_inv, vec![]).await?;

        // Get them
        let query = graph_queries::select_all_extremal_graphs(&extremal, &additional_condition);

        self.fetch_all_row_query(&query, output).await?;

        Ok(())
    }

    /// Tries to find result for this conjecture using this workplace and an extremal selection.
    pub async fn query_extremal_conjecture<O: SaveOutput>(
        &mut self,
        conjecture: ExtremalConjecture,
        output: &mut O,
    ) -> Result<(), WorkplaceError> {
        // Fully compute the necessary invariants (and their dependencies)
        let inv_to_compute = conjecture.get_invariants_to_compute();

        self.compute_invariants(inv_to_compute.clone(), vec![])
            .await?;

        info!("Finished computing extremal graphs, moving on to counterexample search");

        // Only compute the necessary invariants in order to disprove the conjecture
        let mut conjecture_invariants = conjecture.get_invariants_from_conjecture();
        conjecture_invariants.shift_remove(VERTICES_TABLE_NAME);

        if !conjecture_invariants.is_empty() {
            // This automatically adds the dependencies of the conjecture invariants
            let invariant_necessary = ModuleSorter::new_from(
                self.config.get_execs_ref(),
                &conjecture_invariants.difference(&inv_to_compute).collect(), // Remove already computed invariants
            )?;

            // Get additional extremal condition :
            let extremal_condition = conjecture.get_extremal_input_selection();
            info!(
                "Computing conjecture invariants : {conjecture_invariants:?} (skip if already done)"
            );
            self.execute_invariant_modules(
                invariant_necessary,
                Some(extremal_condition),
                Vec::from_iter(inv_to_compute), // No need to compute these already fully computed invariants
            )
            .await?;
            info!("Finished computing conjecture invariants");
        }

        info!("Try to fetch counterexample graphs");
        // Return query result
        let query: SqlSelectQuery = conjecture.into();

        self.fetch_all_row_query(&query, output).await?;

        Ok(())
    }

    async fn compute_invariants(
        &mut self,
        mut inv_to_compute: IndexSet<String>,
        inv_to_skip: Vec<String>,
    ) -> Result<(), WorkplaceError> {
        // This table is used but is not part of any module
        inv_to_compute.shift_remove(VERTICES_TABLE_NAME); // FIXME: Probably remove the vertices table all together :/
        info!("Computing the following invariants: {:?}", inv_to_compute);

        if !inv_to_compute.is_empty() {
            let invariant_necessary =
                ModuleSorter::new_from(self.config.get_execs_ref(), &inv_to_compute)?;
            // Compute them
            self.execute_invariant_modules(invariant_necessary, None, inv_to_skip)
                .await?;
        }

        info!("Finished computing invariants");

        Ok(())
    }

    /// Tries to find a counter example to a conjecture using this workplace.
    pub async fn query_conjecture<O: SaveOutput>(
        &mut self,
        criterion: SqlCondition,
        conjecture: SqlCondition,
        output: &mut O,
    ) -> Result<(), WorkplaceError> {
        // Fully compute the left invariants
        let inv_to_compute = criterion.get_all_identifiers();

        self.compute_invariants(inv_to_compute.clone(), vec![])
            .await?;

        info!("Finished finding invariants ");

        // Only compute the necessary invariants in order to disprove the conjecture
        let mut conjecture_invariants = conjecture.get_all_identifiers();
        conjecture_invariants.shift_remove(VERTICES_TABLE_NAME);

        if !conjecture_invariants.is_empty() {
            // This automatically adds the dependencies of the conjecture invariants
            let invariant_necessary = ModuleSorter::new_from(
                self.config.get_execs_ref(),
                &conjecture_invariants.difference(&inv_to_compute).collect(), // Remove already computed invariants
            )?;

            // Get only graph respecting the criterion :
            let extremal_condition = SqlSelectQuery::select_column_from_table(
                PK_NAME,
                SqlTableSelection::new(graph_queries::select_all_graph_cond(criterion.clone())),
            );
            info!(
                "Computing conjecture invariants : {conjecture_invariants:?} (skip if already done)"
            );
            self.execute_invariant_modules(
                invariant_necessary,
                Some(extremal_condition),
                Vec::from_iter(inv_to_compute), // No need to compute these already fully computed invariants
            )
            .await?;
            info!("Finished computing conjecture invariants");
        }

        info!("Try to fetch counterexample graphs");

        let query = graph_queries::select_all_graph_cond(SqlCondition::and(criterion, conjecture));

        self.fetch_all_row_query(&query, output).await?;

        Ok(())
    }

    async fn fetch_all_row_query<O: SaveOutput>(
        &self,
        query: &SqlSelectQuery,
        output: &mut O,
    ) -> Result<(), WorkplaceError> {
        match &self.db {
            AllowedGraphDb::Sqlite(sqlite_db) => sqlite_db.fetch_all_row_query(query, output).await,
            AllowedGraphDb::Postgres(pg_db) => pg_db.fetch_all_row_query(query, output).await,
        }?;
        Ok(())
    }
}

async fn compute_module(
    allowed_db: &mut AllowedGraphDb,
    module: &Module,
    add_query: Option<SqlSelectQuery>,
    batch_size: usize,
    optional_obs: Option<&mut dyn Observer>,
) -> Result<(), GraphDbRuntimeError> {
    match allowed_db {
        AllowedGraphDb::Sqlite(sqlite_db) => {
            sqlite_db
                .compute_module(module, add_query, batch_size, optional_obs)
                .await?;
        }
        AllowedGraphDb::Postgres(pg_db) => {
            pg_db
                .compute_module(module, add_query, batch_size, optional_obs)
                .await?;
        }
    }
    Ok(())
}
