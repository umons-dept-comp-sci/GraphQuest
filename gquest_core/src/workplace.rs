use std::{collections::HashMap, sync::Arc};

use indexmap::{IndexMap, IndexSet};
use log::info;
use sqlx::Sqlite;
use tabled::grid::config;
use thiserror::Error;
use tokio::{
    sync::Mutex,
    task::{JoinError, JoinSet},
};

use crate::{
    data_handler::{
        data_types::ConstantValue,
        invariant_execs,
        module::{Module, ModuleError},
        rel_graph::{AutoFnCallIterator, FnArg, FnRef, RelationGraph, RelationGraphError},
    },
    database_handler::{
        AllowedGraphDb, GraphDbRuntimeError, GraphDbStartupError, PK_NAME, SqlJoin, SqlSelectQuery,
        SqlTableSelection, VERTICES_TABLE_NAME,
    },
    parser::parsed_expression::{Comparison, Condition, MathExpression, ParsedArgType},
    utils::{SaveOutput, config_file2::ConfigFile, subject::Observer},
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
    #[error("Ran into an error while building the relation graph: \"{0}\"")]
    RelationGraphError(#[from] RelationGraphError),
}

/// A struct used to facilitate more complicated operations involving both a configuration file ([`ConfigFile`]) and an open graph database ([`GraphDatabase`]).
pub struct GquestEngine {
    pub db: AllowedGraphDb,
    config: ConfigFile,
}

impl GquestEngine {
    /// Creates a new [`GquestEngine`].
    pub fn new(db: impl Into<AllowedGraphDb>, config: ConfigFile) -> Self {
        let db = db.into();

        Self { db, config }
    }

    pub async fn close(self) {
        self.db.close_connection().await
    }

    pub async fn exec_condition_no_multithread(
        &mut self,
        cond: Condition<ParsedArgType>,
    ) -> Result<(), WorkplaceError> {
        let mut rel_graph = RelationGraph::new(self.config.get_module_refs());
        let flattened_cond = Self::fill_graph_cond(&mut rel_graph, cond)?;
        // Create
        let mut join_map: IndexMap<FnRef, SqlJoin> = IndexMap::new();

        let mut iter: AutoFnCallIterator<'_> = rel_graph.into_iter().into();

        while let Some(re) = iter.next() {
            println!("sdqsd");
            let (module, args) = iter.get_module_args(&re);
            // Fetch all needed function to join for this module.
            let mut needed_joins = Vec::new();
            for expr in args {
                for prim in expr.get_all_primitives_rec() {
                    if let FnArg::FnCall(dependency) = prim {
                        needed_joins.push(
                            join_map
                                .get(&dependency)
                                .expect("present by the iterator topological sort logic")
                                .clone(),
                        );
                    }
                }
            }

            // Execute the function
            compute_module(
                &mut self.db,
                &re,
                module,
                args,
                needed_joins,
                self.config.get_batch_size(),
                None,
            )
            .await?;

            // Save the result as a join query for later uses.
            let mut join_conditions: Vec<Comparison<FnArg>> = Vec::new();
            for i in 0..args.len() {
                let arg_name = module.args[i].name.clone();
                let arg_value = args[i].clone();
                join_conditions.push(Comparison::Equal(
                    FnArg::Constant(ConstantValue::Identifier(format!(
                        "{}{}.{arg_name}",
                        re.0, re.1
                    )))
                    .into(),
                    arg_value,
                    None,
                ));
            }

            join_map.insert(
                re.clone(),
                SqlJoin::new(re.0.clone(), format!("{}{}", re.0, re.1), join_conditions),
            );
        }

        Ok(())
    }

    fn fill_graph_cond(
        rel_graph: &mut RelationGraph,
        cond: Condition<ParsedArgType>,
    ) -> Result<Condition<FnArg>, WorkplaceError> {
        Ok(match cond {
            Condition::Operation(comparison) => {
                Self::fill_graph_comp(rel_graph, comparison)?.into()
            }
            Condition::Not(condition) => {
                Condition::not(Self::fill_graph_cond(rel_graph, *condition)?)
            }
            Condition::Or(left_cond, right_cond) => Condition::or(
                Self::fill_graph_cond(rel_graph, *left_cond)?,
                Self::fill_graph_cond(rel_graph, *right_cond)?,
            ),
            Condition::And(left_cond, right_cond) => Condition::and(
                Self::fill_graph_cond(rel_graph, *left_cond)?,
                Self::fill_graph_cond(rel_graph, *right_cond)?,
            ),
        })
    }

    fn fill_graph_comp(
        rel_graph: &mut RelationGraph,
        comp: Comparison<ParsedArgType>,
    ) -> Result<Comparison<FnArg>, WorkplaceError> {
        Ok(match comp {
            Comparison::Greater(left_expr, right_expr) => Comparison::Greater(
                Self::fill_graph_expr(rel_graph, left_expr)?,
                Self::fill_graph_expr(rel_graph, right_expr)?,
            ),
            Comparison::GreaterEqual(left_expr, right_expr, e) => Comparison::GreaterEqual(
                Self::fill_graph_expr(rel_graph, left_expr)?,
                Self::fill_graph_expr(rel_graph, right_expr)?,
                e,
            ),
            Comparison::Less(left_expr, right_expr) => Comparison::Less(
                Self::fill_graph_expr(rel_graph, left_expr)?,
                Self::fill_graph_expr(rel_graph, right_expr)?,
            ),
            Comparison::LessEqual(left_expr, right_expr, e) => Comparison::LessEqual(
                Self::fill_graph_expr(rel_graph, left_expr)?,
                Self::fill_graph_expr(rel_graph, right_expr)?,
                e,
            ),
            Comparison::Equal(left_expr, right_expr, e) => Comparison::Equal(
                Self::fill_graph_expr(rel_graph, left_expr)?,
                Self::fill_graph_expr(rel_graph, right_expr)?,
                e,
            ),
            Comparison::NotEqual(left_expr, right_expr, e) => Comparison::NotEqual(
                Self::fill_graph_expr(rel_graph, left_expr)?,
                Self::fill_graph_expr(rel_graph, right_expr)?,
                e,
            ),
        })
    }

    fn fill_graph_expr(
        rel_graph: &mut RelationGraph,
        expr: MathExpression<ParsedArgType>,
    ) -> Result<MathExpression<FnArg>, WorkplaceError> {
        Ok(match expr {
            MathExpression::Primitif(p) => Self::fill_graph_primitif(rel_graph, p)?.into(),
            MathExpression::Negation(math_expression) => {
                MathExpression::negation(Self::fill_graph_expr(rel_graph, *math_expression)?)
            }
            MathExpression::Floor(math_expression) => {
                MathExpression::floor(Self::fill_graph_expr(rel_graph, *math_expression)?)
            }
            MathExpression::Ceil(math_expression) => {
                MathExpression::ceil(Self::fill_graph_expr(rel_graph, *math_expression)?)
            }
            MathExpression::Abs(math_expression) => {
                MathExpression::abs(Self::fill_graph_expr(rel_graph, *math_expression)?)
            }
            MathExpression::Sqrt(math_expression) => {
                MathExpression::sqrt(Self::fill_graph_expr(rel_graph, *math_expression)?)
            }
            MathExpression::BinOperation { left, op, right } => {
                let new_left = Self::fill_graph_expr(rel_graph, *left)?;
                let new_right = Self::fill_graph_expr(rel_graph, *right)?;
                MathExpression::bin_operation(new_left, op, new_right)
            }
        })
    }

    fn fill_graph_primitif(
        rel_graph: &mut RelationGraph,
        p: ParsedArgType,
    ) -> Result<FnArg, WorkplaceError> {
        Ok(match p {
            ParsedArgType::PrimString(s) => FnArg::Constant(ConstantValue::String(s)),
            ParsedArgType::PrimNumeric(n) => FnArg::Constant(ConstantValue::Numeric(n)),
            ParsedArgType::Graph => FnArg::Graph,
            ParsedArgType::Function(parsed_function) => {
                //  Recursively adds all possible functions to the graphs that are in the arguments of this function.
                // as well as turning parsed argument into FnArg, thus flattening the functions arguments.
                // ex: P(G, chroma(n) + 12) -> P(G, chroma0 + 12) with chroma0 = chroma(n0) with n0 = n(G)
                let mut args = vec![GquestEngine::fill_graph_expr(
                    rel_graph,
                    *parsed_function.first_arg,
                )?];

                for arg in parsed_function.other_args {
                    args.push(GquestEngine::fill_graph_expr(rel_graph, arg)?);
                }

                // Add this function to the graph
                let fn_ref = rel_graph.try_add_fn_call(parsed_function.name.clone(), &args)?;

                // Add all dependencies by finding all primitives in each arguments and checking if they contain a function or not.
                for arg in &args {
                    let prim_args = arg.get_all_primitives_rec();
                    for prim in prim_args {
                        if let FnArg::FnCall(dep_fn_ref) = prim {
                            rel_graph.try_add_dep(&fn_ref, &dep_fn_ref)?;
                        }
                    }
                }
                FnArg::FnCall(fn_ref)
            }
        })
    }
}

async fn compute_module(
    allowed_db: &mut AllowedGraphDb,
    fn_ref: &FnRef,
    module: &Module,
    args: &Vec<MathExpression<FnArg>>,
    join_list: Vec<SqlJoin>,
    batch_size: usize,
    mut optional_obs: Option<&mut dyn Observer>,
) -> Result<(), GraphDbRuntimeError> {
    match allowed_db {
        AllowedGraphDb::Sqlite(sqlite_db) => {
            sqlite_db
                .compute_module(fn_ref, module, args, join_list, batch_size, optional_obs)
                .await?;
        }
        AllowedGraphDb::Postgres(pg_db) => {
            pg_db
                .compute_module(fn_ref, module, args, join_list, batch_size, optional_obs)
                .await?;
        }
    }
    Ok(())
}

// impl Workplace {
//     /// Creates a new [`Workplace`].
//     pub fn new(db: impl Into<AllowedGraphDb>, config: ConfigFile) -> Self {
//         let db = db.into();

//         Self { db, config }
//     }

//     /// Executes all stored invariants using the given settings from the [`ConfigFile`].
//     /// Does not impose any condition on the graph used for the computations.
//     pub async fn execute_all_modules(&mut self) -> Result<(), WorkplaceError> {
//         self.execute_invariant_modules(
//             self.config.get_execs_ref().clone().try_into()?,
//             None,
//             vec![],
//         )
//         .await
//     }

//     /// Executes the modules stored inside the sorter using the given settings from the [`ConfigFile`].
//     pub async fn execute_invariant_modules(
//         &mut self,
//         sorter: ModuleSorter,
//         add_condition: Option<SqlSelectQuery>,
//         inv_to_skip: Vec<String>,
//     ) -> Result<(), WorkplaceError> {
//         let mut sorted = sorter.to_iter()?;
//         // If allowed, use multiple threads to compute the invariants
//         if self.config.get_nb_threads() > 1 {
//             info!(
//                 "Execute all modules using at most {} threads",
//                 self.config.get_nb_threads()
//             );
//             self.execute_all_modules_multithread(sorted, add_condition, inv_to_skip)
//                 .await
//         } else {
//             info!("Execute all modules using 1 thread");
//             while let Some(next_inv) = sorted.next_module() {
//                 // Check if this invariant should be skipped
//                 if !next_inv
//                     .invariant_names
//                     .iter()
//                     .all(|name| inv_to_skip.contains(name))
//                 {
//                     info!("Doing invariants \"{:?}\"", next_inv.invariant_names);
//                     compute_module(
//                         &mut self.db,
//                         &next_inv,
//                         add_condition.clone(),
//                         self.config.get_batch_size(),
//                         None,
//                     )
//                     .await?;

//                     info!(
//                         "Finished doing invariants \"{:?}\"",
//                         next_inv.invariant_names
//                     );
//                 }

//                 sorted.update_dependencies(&next_inv);
//             }
//             Ok(())
//         }
//     }

//     async fn execute_all_modules_multithread(
//         &mut self,
//         sorted: ModuleIterator,
//         add_condition: Option<SqlSelectQuery>,
//         inv_to_skip: Vec<String>,
//     ) -> Result<(), WorkplaceError> {
//         let iter = Arc::new(Mutex::new(sorted));

//         // Used to directly unlock the mutex after acquiring the next invariant
//         let get_next_inv = &mut async |iter_lock: &Arc<Mutex<ModuleIterator>>| {
//             iter_lock.lock().await.next_module()
//         };

//         let mut handles: JoinSet<Result<(), GraphDbRuntimeError>> = JoinSet::new();
//         while !iter.lock().await.is_finished() {
//             // Spawns as many threads as possible while respecting the maximum number of threads to use
//             while let Some(next_inv) = get_next_inv(&iter).await
//                 && handles.len() < self.config.get_nb_threads()
//             {
//                 // Check if this invariant should be skipped
//                 if !next_inv
//                     .invariant_names
//                     .iter()
//                     .all(|name| inv_to_skip.contains(name))
//                 {
//                     info!(
//                         "Starting a thread to do invariants \"{:?}\"",
//                         next_inv.invariant_names
//                     );
//                     let mut db_clone = self.db.clone();
//                     let batch_size = self.config.get_batch_size();
//                     let add_condition_clone = add_condition.clone();
//                     // let update_dep_clone = *update_dep;
//                     let iter_clone = iter.clone();
//                     // A thread will compute the given invariant then end
//                     handles.spawn(async move {
//                         compute_module(
//                             &mut db_clone,
//                             &next_inv,
//                             add_condition_clone,
//                             batch_size,
//                             None,
//                         )
//                         .await?;
//                         // Update the graph so that the main thread can compute new invariants
//                         iter_clone.lock().await.update_dependencies(&next_inv);
//                         info!(
//                             "Thread finished doing invariants \"{:?}\"",
//                             next_inv.invariant_names
//                         );
//                         Ok(())
//                     });
//                 } else {
//                     // Skip this invariant and move to another one directly
//                     iter.lock().await.update_dependencies(&next_inv);
//                 }
//             }
//             // Wait for any of them to finish then execute new ones
//             handles.join_next().await.expect("joinSet is not empty")??; // checks for JoinError then for the GraphRuntimeError
//         }
//         info!("Wait for all threads to finish");
//         // wait for all threads to finish and checks for any errors
//         for handle in handles.join_all().await {
//             handle?
//         }
//         info!("Finished waiting for threads");
//         Ok(())
//     }

//     /// Sends a raw sql query to the database and displays its return value.
//     pub async fn send_raw_sql<O: SaveOutput>(
//         db: AllowedGraphDb,
//         query: impl Into<String>,
//         output: &mut O,
//     ) -> Result<(), WorkplaceError> {
//         match db {
//             AllowedGraphDb::Sqlite(sqlite_db) => {
//                 sqlite_db.fetch_all_rows_raw_sql(query.into(), output).await
//             }
//             AllowedGraphDb::Postgres(pg_db) => {
//                 pg_db.fetch_all_rows_raw_sql(query.into(), output).await
//             }
//         }?;
//         Ok(())
//     }

//     pub async fn query_condition<O: SaveOutput>(
//         &mut self,
//         condition: SqlCondition,
//         output: &mut O,
//     ) -> Result<(), WorkplaceError> {
//         let invariants = condition.get_all_identifiers();
//         self.compute_invariants(invariants, vec![]).await?;

//         let query = graph_queries::select_all_graph_cond(condition);

//         self.fetch_all_row_query(&query, output).await?;

//         Ok(())
//     }

//     pub async fn find_extremals_graphs<O: SaveOutput>(
//         &mut self,
//         extremal: ClassSelection,
//         additional_condition: Option<SqlCondition>,
//         output: &mut O,
//     ) -> Result<(), WorkplaceError> {
//         // Fetch module names :
//         let extremal_inv = graph_queries::get_extremal_invariants(&extremal, &additional_condition);
//         // Compute all invariants :
//         self.compute_invariants(extremal_inv, vec![]).await?;

//         // Get them
//         let query = graph_queries::select_all_extremal_graphs(&extremal, &additional_condition);

//         self.fetch_all_row_query(&query, output).await?;

//         Ok(())
//     }

//     /// Tries to find result for this conjecture using this workplace and an extremal selection.
//     pub async fn query_extremal_conjecture<O: SaveOutput>(
//         &mut self,
//         conjecture: ExtremalConjecture,
//         output: &mut O,
//     ) -> Result<(), WorkplaceError> {
//         // Fully compute the necessary invariants (and their dependencies)
//         let inv_to_compute = conjecture.get_invariants_to_compute();

//         self.compute_invariants(inv_to_compute.clone(), vec![])
//             .await?;

//         info!("Finished computing extremal graphs, moving on to counterexample search");

//         // Only compute the necessary invariants in order to disprove the conjecture
//         let mut conjecture_invariants = conjecture.get_invariants_from_conjecture();
//         conjecture_invariants.shift_remove(VERTICES_TABLE_NAME);

//         if !conjecture_invariants.is_empty() {
//             // This automatically adds the dependencies of the conjecture invariants
//             let invariant_necessary = ModuleSorter::new_from(
//                 self.config.get_execs_ref(),
//                 &conjecture_invariants.difference(&inv_to_compute).collect(), // Remove already computed invariants
//             )?;

//             // Get additional extremal condition :
//             let extremal_condition = conjecture.get_extremal_input_selection();
//             info!(
//                 "Computing conjecture invariants : {conjecture_invariants:?} (skip if already done)"
//             );
//             self.execute_invariant_modules(
//                 invariant_necessary,
//                 Some(extremal_condition),
//                 Vec::from_iter(inv_to_compute), // No need to compute these already fully computed invariants
//             )
//             .await?;
//             info!("Finished computing conjecture invariants");
//         }

//         info!("Try to fetch counterexample graphs");
//         // Return query result
//         let query: SqlSelectQuery = conjecture.into();

//         self.fetch_all_row_query(&query, output).await?;

//         Ok(())
//     }

//     async fn compute_invariants(
//         &mut self,
//         mut inv_to_compute: IndexSet<String>,
//         inv_to_skip: Vec<String>,
//     ) -> Result<(), WorkplaceError> {
//         // This table is used but is not part of any module
//         inv_to_compute.shift_remove(VERTICES_TABLE_NAME); // FIXME: Probably remove the vertices table all together :/
//         info!("Computing the following invariants: {:?}", inv_to_compute);

//         if !inv_to_compute.is_empty() {
//             let invariant_necessary =
//                 ModuleSorter::new_from(self.config.get_execs_ref(), &inv_to_compute)?;
//             // Compute them
//             self.execute_invariant_modules(invariant_necessary, None, inv_to_skip)
//                 .await?;
//         }

//         info!("Finished computing invariants");

//         Ok(())
//     }

//     /// Tries to find a counter example to a conjecture using this workplace.
//     pub async fn query_conjecture<O: SaveOutput>(
//         &mut self,
//         criterion: SqlCondition,
//         conjecture: SqlCondition,
//         output: &mut O,
//     ) -> Result<(), WorkplaceError> {
//         // Fully compute the left invariants
//         let inv_to_compute = criterion.get_all_identifiers();

//         self.compute_invariants(inv_to_compute.clone(), vec![])
//             .await?;

//         info!("Finished finding invariants ");

//         // Only compute the necessary invariants in order to disprove the conjecture
//         let mut conjecture_invariants = conjecture.get_all_identifiers();
//         conjecture_invariants.shift_remove(VERTICES_TABLE_NAME);

//         if !conjecture_invariants.is_empty() {
//             // This automatically adds the dependencies of the conjecture invariants
//             let invariant_necessary = ModuleSorter::new_from(
//                 self.config.get_execs_ref(),
//                 &conjecture_invariants.difference(&inv_to_compute).collect(), // Remove already computed invariants
//             )?;

//             // Get only graph respecting the criterion :
//             let extremal_condition = SqlSelectQuery::select_column_from_table(
//                 PK_NAME,
//                 SqlTableSelection::new(graph_queries::select_all_graph_cond(criterion.clone())),
//             );
//             info!(
//                 "Computing conjecture invariants : {conjecture_invariants:?} (skip if already done)"
//             );
//             self.execute_invariant_modules(
//                 invariant_necessary,
//                 Some(extremal_condition),
//                 Vec::from_iter(inv_to_compute), // No need to compute these already fully computed invariants
//             )
//             .await?;
//             info!("Finished computing conjecture invariants");
//         }

//         info!("Try to fetch counterexample graphs");

//         let query = graph_queries::select_all_graph_cond(SqlCondition::and(criterion, conjecture));

//         self.fetch_all_row_query(&query, output).await?;

//         Ok(())
//     }

//     async fn fetch_all_row_query<O: SaveOutput>(
//         &self,
//         query: &SqlSelectQuery,
//         output: &mut O,
//     ) -> Result<(), WorkplaceError> {
//         match &self.db {
//             AllowedGraphDb::Sqlite(sqlite_db) => sqlite_db.fetch_all_row_query(query, output).await,
//             AllowedGraphDb::Postgres(pg_db) => pg_db.fetch_all_row_query(query, output).await,
//         }?;
//         Ok(())
//     }
// }
