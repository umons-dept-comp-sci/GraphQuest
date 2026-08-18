use std::collections::{HashMap, HashSet};

use indexmap::IndexMap;
use log::{debug, error, info};
use thiserror::Error;
use tokio::task::JoinError;

use crate::{
    data_handler::{
        data_types::ConstantValue,
        module::{Module, ModuleError},
        rel_graph::{AutoFnCallIterator, FnArg, FnRef, RelationGraph, RelationGraphError},
    },
    database_handler::{
        AllowedGraphDb, CANONICAL_TABLE_NAME, FUNCTION_OUTPUT_COL_NAME, GraphDbRuntimeError,
        GraphDbStartupError, PK_NAME, SqlJoin, SqlSelectQuery, SqlTableSelection,
    },
    parser::parsed_expression::{
        Comparison, Condition, MathExpression, ParsedArgType, QueryStatement,
    },
    utils::{SaveOutput, config_file::ConfigFile, subject::Observer},
};

#[derive(Debug, Error)]
pub enum EngineError {
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

    /// Sends a raw sql query to the database and displays its return value.
    pub async fn send_raw_sql<O: SaveOutput>(
        db: AllowedGraphDb,
        query: impl Into<String>,
        output: &mut O,
    ) -> Result<(), EngineError> {
        match db {
            AllowedGraphDb::Sqlite(sqlite_db) => {
                sqlite_db.fetch_all_rows_raw_sql(query.into(), output).await
            }
            AllowedGraphDb::Postgres(pg_db) => {
                pg_db.fetch_all_rows_raw_sql(query.into(), output).await
            }
        }?;
        Ok(())
    }

    pub async fn exec_query<O: SaveOutput>(
        &mut self,
        mut query: QueryStatement,
        output: &mut O,
    ) -> Result<(), EngineError> {
        let mut cond_engine = ConditionEngine::new(&mut self.db, self.config.get_module_refs());

        info!("Starting query's modules executions");
        loop {
            match query {
                QueryStatement::Condition(condition) => {
                    cond_engine
                        .add_new_cond(condition, self.config.get_batch_size())
                        .await?;
                    break;
                }
                QueryStatement::IfThen(condition, query_statement) => {
                    cond_engine
                        .add_new_cond(condition, self.config.get_batch_size())
                        .await?;

                    query = *query_statement;
                    info!("Moving on to the next If-Then clause");
                }
            };
        }

        info!("Finished query's modules execution");

        cond_engine.get_result(output).await
    }

    pub async fn exec_condition<O: SaveOutput>(
        &mut self,
        cond: Condition<ParsedArgType>,
        output: &mut O,
    ) -> Result<(), EngineError> {
        info!("Starting condition's modules execution");
        let mut cond_engine = ConditionEngine::new(&mut self.db, self.config.get_module_refs());

        cond_engine
            .add_new_cond(cond, self.config.get_batch_size())
            .await?;
        info!("Finished condition's modules execution");

        cond_engine.get_result(output).await
    }
}

pub struct ConditionEngine<'a> {
    db: &'a mut AllowedGraphDb,
    // modules: Option<&'a HashMap<String, Module>>,
    graph: Option<RelationGraph<'a>>,
    /// The query that can provide the expected result.
    result: SqlSelectQuery,
    /// Contains the reference of the function's call that have already been computed before.
    already_computed: HashSet<FnRef>,
}

impl<'a> ConditionEngine<'a> {
    fn new(db: &'a mut AllowedGraphDb, modules: &'a HashMap<String, Module>) -> Self {
        Self {
            db,
            graph: Some(RelationGraph::new(modules)),
            result: SqlSelectQuery::select_all_from_table(CANONICAL_TABLE_NAME),
            already_computed: HashSet::new(),
        }
    }

    async fn add_new_cond(
        &mut self,
        cond: Condition<ParsedArgType>,
        batch_size: usize,
    ) -> Result<(), EngineError> {
        error!("Doing {cond:?}");
        // Borrow the relation graph to use it as a mutable variable alongside the engine.
        let mut graph = self.graph.take().expect("present");
        {
            // Add cond to the relation graph.
            let flattened_cond = fill_graph_cond(&mut graph, cond)?;

            // This hashmap will contain the ref to SqlJoin that can
            // be used to not have to recompute functions alongside their dependencies.
            let mut join_dep_map: IndexMap<FnRef, (SqlJoin, HashSet<FnRef>)> = IndexMap::new();

            let mut iter: AutoFnCallIterator<'_, '_> = graph.into_iter().into();
            // Iterate over all function references that are stored in the graph and that are needed in a topological ordering.
            while let Some(function_ref) = iter.next() {
                let fn_name = iter.get_inner_iter().get_function_name(&function_ref);
                // Get the module and the arguments that are linked to this function call.
                let (module, args) = iter.get_inner_iter().get_module_args(&function_ref);

                debug!("The function {fn_name} has to be executed.");
                // The list of all needed results to join for the selection query of this module.
                let mut needed_joins = Vec::new();
                // The set of already added function joined to the previous vec (used in order to not re-execute functions for no reason).
                let mut already_added = HashSet::new();

                // Find dependencies
                for expr in args {
                    for prim in expr.get_all_primitives_rec() {
                        // For every other function calls located in the args of this function call
                        // add them (and their own function calls) to the list of needed joins
                        if let FnArg::FnCall(dependency) = prim {
                            add_rec_needed_joins(
                                &dependency,
                                &mut needed_joins,
                                &mut already_added,
                                &join_dep_map,
                            );
                        }
                    }
                }
                println!("Needed joins: {needed_joins:?}\n Already added: {already_added:?}");

                // Save the result as a join query for later uses
                // by saving the selected args first in a vector of comparison.
                {
                    let mut join_conditions: Vec<Comparison<FnArg>> =
                        Vec::with_capacity(args.len());
                    for i in 0..args.len() {
                        let arg_name = module.args[i].name.clone();
                        let arg_value = args[i].clone();
                        join_conditions.push(Comparison::Equal(
                            FnArg::Constant(ConstantValue::Identifier(format!(
                                "{}{}.{arg_name}",
                                function_ref.0, function_ref.1
                            )))
                            .into(),
                            arg_value,
                            None,
                        ));
                    }
                    // then create/save the join for the function
                    join_dep_map.insert(
                        function_ref.clone(),
                        (
                            SqlJoin::new(
                                function_ref.0.clone(),
                                format!("{}{}", function_ref.0, function_ref.1),
                                join_conditions,
                            ),
                            already_added, // Already added contains all dependencies of this function call.
                        ),
                    );
                }

                if self.already_computed.contains(&function_ref) {
                    debug!("But it was already computed previously so we skip it");
                    continue;
                }

                // Save this function as already computed.
                self.already_computed.insert(function_ref.clone());

                // Execute the function
                debug!("Executing the function call {fn_name}");
                compute_module(
                    self.db,
                    &function_ref,
                    module,
                    args,
                    self.result.clone(), // Uses the previous condition result as the dataset for this one
                    needed_joins,
                    batch_size,
                    None,
                )
                .await?;
            }

            debug!("All function call for this condition were executed.");
            // Update the result query :
            self.result = self.build_result(&graph, flattened_cond, join_dep_map);
        }
        // Give ownership back of the graph to the engine.
        graph.set_all_unused();
        self.graph = Some(graph);

        Ok(())
    }

    /// Builds the result query.
    fn build_result(
        &self,
        graph: &RelationGraph,
        flattened_cond: Condition<FnArg>,
        join_dep_map: IndexMap<FnRef, (SqlJoin, HashSet<FnRef>)>,
    ) -> SqlSelectQuery {
        // Fetch all columns needed for this query always starting with the signatures from the current dataset.
        let mut all_columns = vec![MathExpression::Primitif(FnArg::Constant(
            ConstantValue::Identifier(format!("{CANONICAL_TABLE_NAME}.{PK_NAME}")),
        ))];

        //  Collect all needed joins
        let all_joins: Vec<SqlJoin> = join_dep_map
            .into_iter()
            .map(|(fn_ref, j)| {
                // as well as adding them to the resulting query outcome.
                all_columns.push(MathExpression::Primitif(FnArg::Constant(
                    ConstantValue::Identifier(format!(
                        "{}{}.{FUNCTION_OUTPUT_COL_NAME} as \"{}\"",
                        fn_ref.0,
                        fn_ref.1,
                        graph.func_to_string(&fn_ref).expect("correct val")
                    )),
                )));
                j.0
            })
            .collect();

        let dataset_table =
            SqlTableSelection::new_rename(self.result.clone(), CANONICAL_TABLE_NAME);

        let mut selection = SqlSelectQuery::select_columns_from_table(all_columns, dataset_table)
            .set_where_clause(flattened_cond);

        selection.set_distinct_values(true);
        // Thanks to the topological sort, this is already in the correct order.
        for join in all_joins {
            selection.add_join(join);
        }

        selection
    }

    async fn get_result<O: SaveOutput>(&self, output: &mut O) -> Result<(), EngineError> {
        info!("Start fetching the result");
        match &self.db {
            AllowedGraphDb::Sqlite(sqlite_db) => {
                sqlite_db.fetch_all_row_query(&self.result, output).await
            }
            AllowedGraphDb::Postgres(pg_db) => {
                pg_db.fetch_all_row_query(&self.result, output).await
            }
        }?;
        info!("Finished fetching the result");
        Ok(())
    }
}

// The following function cannot contain "self/ConditionEngine" due to
// the possibility of adding multithreading later down the line.
#[allow(clippy::too_many_arguments)]
async fn compute_module(
    db: &mut AllowedGraphDb,
    fn_ref: &FnRef,
    module: &Module,
    args: &[MathExpression<FnArg>],
    mut dataset_to_use: SqlSelectQuery,
    join_list: Vec<SqlJoin>,
    batch_size: usize,
    optional_obs: Option<&mut dyn Observer>,
) -> Result<(), GraphDbRuntimeError> {
    dataset_to_use.set_col_selection(format!("{CANONICAL_TABLE_NAME}.{PK_NAME}"));

    match db {
        AllowedGraphDb::Sqlite(sqlite_db) => {
            sqlite_db
                .compute_module(
                    fn_ref,
                    module,
                    args,
                    Some(dataset_to_use),
                    join_list,
                    batch_size,
                    optional_obs,
                )
                .await?;
        }
        AllowedGraphDb::Postgres(pg_db) => {
            pg_db
                .compute_module(
                    fn_ref,
                    module,
                    args,
                    Some(dataset_to_use),
                    join_list,
                    batch_size,
                    optional_obs,
                )
                .await?;
        }
    }
    Ok(())
}

// ___________________________ UTILS ___________________________

fn fill_graph_cond(
    rel_graph: &mut RelationGraph,
    cond: Condition<ParsedArgType>,
) -> Result<Condition<FnArg>, RelationGraphError> {
    Ok(match cond {
        Condition::Operation(comparison) => fill_graph_comp(rel_graph, comparison)?.into(),
        Condition::Not(condition) => Condition::not(fill_graph_cond(rel_graph, *condition)?),
        Condition::Or(left_cond, right_cond) => Condition::or(
            fill_graph_cond(rel_graph, *left_cond)?,
            fill_graph_cond(rel_graph, *right_cond)?,
        ),
        Condition::And(left_cond, right_cond) => Condition::and(
            fill_graph_cond(rel_graph, *left_cond)?,
            fill_graph_cond(rel_graph, *right_cond)?,
        ),
    })
}

fn fill_graph_comp(
    rel_graph: &mut RelationGraph,
    comp: Comparison<ParsedArgType>,
) -> Result<Comparison<FnArg>, RelationGraphError> {
    Ok(match comp {
        Comparison::Greater(left_expr, right_expr) => Comparison::Greater(
            fill_graph_expr(rel_graph, left_expr)?,
            fill_graph_expr(rel_graph, right_expr)?,
        ),
        Comparison::GreaterEqual(left_expr, right_expr, e) => Comparison::GreaterEqual(
            fill_graph_expr(rel_graph, left_expr)?,
            fill_graph_expr(rel_graph, right_expr)?,
            e,
        ),
        Comparison::Less(left_expr, right_expr) => Comparison::Less(
            fill_graph_expr(rel_graph, left_expr)?,
            fill_graph_expr(rel_graph, right_expr)?,
        ),
        Comparison::LessEqual(left_expr, right_expr, e) => Comparison::LessEqual(
            fill_graph_expr(rel_graph, left_expr)?,
            fill_graph_expr(rel_graph, right_expr)?,
            e,
        ),
        Comparison::Equal(left_expr, right_expr, e) => Comparison::Equal(
            fill_graph_expr(rel_graph, left_expr)?,
            fill_graph_expr(rel_graph, right_expr)?,
            e,
        ),
        Comparison::NotEqual(left_expr, right_expr, e) => Comparison::NotEqual(
            fill_graph_expr(rel_graph, left_expr)?,
            fill_graph_expr(rel_graph, right_expr)?,
            e,
        ),
    })
}

fn fill_graph_expr(
    rel_graph: &mut RelationGraph,
    expr: MathExpression<ParsedArgType>,
) -> Result<MathExpression<FnArg>, RelationGraphError> {
    Ok(match expr {
        MathExpression::Primitif(p) => fill_graph_primitif(rel_graph, p)?.into(),
        MathExpression::Negation(math_expression) => {
            MathExpression::negation(fill_graph_expr(rel_graph, *math_expression)?)
        }
        MathExpression::Floor(math_expression) => {
            MathExpression::floor(fill_graph_expr(rel_graph, *math_expression)?)
        }
        MathExpression::Ceil(math_expression) => {
            MathExpression::ceil(fill_graph_expr(rel_graph, *math_expression)?)
        }
        MathExpression::Abs(math_expression) => {
            MathExpression::abs(fill_graph_expr(rel_graph, *math_expression)?)
        }
        MathExpression::Sqrt(math_expression) => {
            MathExpression::sqrt(fill_graph_expr(rel_graph, *math_expression)?)
        }
        MathExpression::BinOperation { left, op, right } => {
            let new_left = fill_graph_expr(rel_graph, *left)?;
            let new_right = fill_graph_expr(rel_graph, *right)?;
            MathExpression::bin_operation(new_left, op, new_right)
        }
    })
}

fn fill_graph_primitif(
    rel_graph: &mut RelationGraph,
    p: ParsedArgType,
) -> Result<FnArg, RelationGraphError> {
    Ok(match p {
        ParsedArgType::PrimString(s) => FnArg::Constant(ConstantValue::String(s)),
        ParsedArgType::PrimNumeric(n) => FnArg::Constant(ConstantValue::Numeric(n)),
        ParsedArgType::Dataset => FnArg::Dataset,
        ParsedArgType::Function(parsed_function) => {
            //  Recursively adds all possible functions to the graphs that are in the arguments of this function.
            // as well as turning parsed argument into FnArg, thus flattening the functions arguments.
            // ex: P(G, chroma(n) + 12) -> P(G, chroma0 + 12) with chroma0 = chroma(n0) with n0 = n(G)
            let mut args = vec![fill_graph_expr(rel_graph, *parsed_function.first_arg)?];

            for arg in parsed_function.other_args {
                args.push(fill_graph_expr(rel_graph, arg)?);
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

/// Adds recursively add all needed join for the `to_add` argument.
/// * `res`: Vector which will store the needed join.
/// * `already_in_res`: prevents adding twice the same join.
/// * `join_dep_map`: the Hashmap that stores all the needed joins.
fn add_rec_needed_joins(
    to_add: &FnRef,
    res: &mut Vec<SqlJoin>,
    already_in_res: &mut HashSet<FnRef>,
    join_dep_map: &IndexMap<FnRef, (SqlJoin, HashSet<FnRef>)>,
) {
    if already_in_res.contains(to_add) {
        return;
    }
    let (join, deps) = join_dep_map.get(to_add).unwrap_or_else(|| {
        panic!("join_dep_map: {join_dep_map:?} \n{to_add:?}: present by the iterator topological sort logic")
    });
    // add all dependency for this join
    for dep in deps {
        add_rec_needed_joins(dep, res, already_in_res, join_dep_map);
    }
    // then add it
    res.push(join.clone());
    already_in_res.insert(to_add.clone());
}
