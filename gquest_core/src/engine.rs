use std::collections::{HashMap, HashSet};

use indexmap::IndexMap;
use log::{debug, info};
use thiserror::Error;
use tokio::task::JoinError;

use crate::{
    data_handler::{
        data_types::ConstantValue,
        module::{Module, ModuleError},
        rel_graph::{AutoFnCallIterator, FnArg, FnRef, RelationGraph, RelationGraphError},
    },
    database_handler::{
        AllowedGraphDb, CANONICAL_TABLE_NAME, Column, ColumnValue, FUNCTION_OUTPUT_COL_NAME,
        GraphDbRuntimeError, PK_NAME, SqlJoin, SqlSelectQuery, SqlTableSelection,
    },
    parser::{
        ExtremalCondition,
        extremal_expression::ExtremalSelection,
        parsed_expression::{
            Comparison,
            Condition::{self},
            MathExpression, ParsedArgType, QueryStatement,
        },
    },
    utils::{SaveOutput, config_file::ConfigFile, subject::Observer},
};

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("Encountered an error from the database during an execution : \"{0}\"")]
    GraphDbRuntimeError(#[from] GraphDbRuntimeError),
    #[error("Encountered an error from a module : \"{0}\"")]
    ModuleError(#[from] ModuleError),
    #[error("One of the invariant thread did not end correctly: \"{0}\"")]
    JoinError(#[from] JoinError),
    #[error("Ran into an error while building the relation graph:\n{0}")]
    RelationGraphError(#[from] RelationGraphError),
}

/// A struct used to facilitate more complicated operations involving both a configuration file ([`ConfigFile`]) and an open graph database ([`AllowedGraphDb`]).
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

    /// Closes the connection to the given [`AllowedGraphDb`].
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
            #[cfg(any(feature = "sqlite-unbundled", feature = "sqlite"))]
            AllowedGraphDb::Sqlite(sqlite_db) => {
                sqlite_db.fetch_all_rows_raw_sql(query.into(), output).await
            }
            #[cfg(feature = "postgres")]
            AllowedGraphDb::Postgres(pg_db) => {
                pg_db.fetch_all_rows_raw_sql(query.into(), output).await
            }
        }?;
        Ok(())
    }

    /// Executes the given query and writes the outputs.
    pub async fn exec_query<O: SaveOutput>(
        &mut self,
        mut query: QueryStatement,
        add_expr: Vec<MathExpression<ParsedArgType>>,
        output: &mut O,
    ) -> Result<(), EngineError> {
        let mut cond_engine = ConditionEngine::new(&mut self.db, self.config.get_module_refs());

        info!("Starting query's modules executions");
        loop {
            match query {
                QueryStatement::ExtremalCondition(condition) => {
                    info!("Executing: {condition}");
                    cond_engine
                        .add_new_extr_cond(condition, self.config.get_batch_size())
                        .await?;
                    break;
                }
                QueryStatement::IfThen(condition, query_statement) => {
                    info!("Executing: {condition}");
                    cond_engine
                        .add_new_extr_cond(condition, self.config.get_batch_size())
                        .await?;

                    query = *query_statement;
                    info!("Moving on to the next If-Then clause");
                }
            };
        }

        info!("Finished query's modules execution");
        if !add_expr.is_empty() {
            info!("Starting additional expressions execution");
            cond_engine
                .add_additional_expressions(add_expr, self.config.get_batch_size())
                .await?;

            info!("Finished additional expressions execution");
        }

        cond_engine.get_result(output).await
    }

    /// Finds the graphs that fits the given conditions.
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

struct ConditionEngine<'a> {
    /// The graph database used to store function results.
    db: &'a mut AllowedGraphDb,
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
            result: SqlSelectQuery::select_column_from_table(PK_NAME, None, CANONICAL_TABLE_NAME),
            already_computed: HashSet::new(),
        }
    }

    async fn add_additional_expressions(
        &mut self,
        expressions: Vec<MathExpression<ParsedArgType>>,
        batch_size: usize,
    ) -> Result<(), EngineError> {
        let mut flattened_expressions = Vec::with_capacity(expressions.len());

        // Borrow the relation graph to use it as a mutable variable alongside the engine.
        let mut graph = self.graph.take().expect("present");
        {
            // Add all expressions
            for expr in expressions {
                flattened_expressions.push(flatten_expr(&mut graph, expr)?);
            }
            // Compute them
            let join_dep_map = self.execute_graph_functions(&mut graph, batch_size).await?;

            // Update res query:
            self.result =
                self.build_expression_list_result(&graph, flattened_expressions, join_dep_map);
        }
        // Sets all function call as unused as to not have to use them again unless mentionned
        graph.set_all_unused();
        // Give ownership back of the graph to the engine.
        self.graph = Some(graph);
        Ok(())
    }

    async fn add_new_extr_cond(
        &mut self,
        extr_cond: ExtremalCondition<ParsedArgType>,
        batch_size: usize,
    ) -> Result<(), EngineError> {
        match extr_cond {
            ExtremalCondition::ExtremalSelection(extremal_selection) => {
                self.add_new_extremal(extremal_selection, batch_size).await
            }
            ExtremalCondition::Condition(condition) => {
                self.add_new_cond(condition, batch_size).await
            }
        }
    }

    async fn add_new_extremal(
        &mut self,
        extremal: ExtremalSelection<ParsedArgType>,
        batch_size: usize,
    ) -> Result<(), EngineError> {
        // Borrow the relation graph to use it as a mutable variable alongside the engine.
        let mut graph = self.graph.take().expect("present");
        {
            let flattened_extremal = flatten_selection(&mut graph, extremal)?;

            // Check if all arguments in the selection are correct
            graph.try_is_valid_extremal_selection(&flattened_extremal)?;

            // This hashmap will contain the ref to SqlJoin that can
            // be used to not have to recompute functions alongside their dependencies.
            let join_dep_map = self.execute_graph_functions(&mut graph, batch_size).await?;

            // Update the result query :
            self.result = self.build_extremal_result(&graph, flattened_extremal, join_dep_map);
        }

        // Sets all function call as unused as to not have to use them again unless mentionned
        graph.set_all_unused();
        // Give ownership back of the graph to the engine.
        self.graph = Some(graph);
        Ok(())
    }

    async fn add_new_cond(
        &mut self,
        cond: Condition<ParsedArgType>,
        batch_size: usize,
    ) -> Result<(), EngineError> {
        // Borrow the relation graph to use it as a mutable variable alongside the engine.
        let mut graph = self.graph.take().expect("present");
        {
            // Add cond to the relation graph and flatten it
            let flattened_cond = flatten_cond(&mut graph, cond)?;

            // Check if all arguments in the condition are correct
            graph.try_is_valid_cond(&flattened_cond)?;

            // This hashmap will contain the ref to SqlJoin that can
            // be used to not have to recompute functions alongside their dependencies.
            let join_dep_map = self.execute_graph_functions(&mut graph, batch_size).await?;

            debug!("All function call for this condition were executed.");
            // Update the result query :
            self.result = self.build_cond_result(&graph, flattened_cond, join_dep_map);
        }
        // Sets all function call as unused as to not have to use them again unless mentionned
        graph.set_all_unused();
        // Give ownership back of the graph to the engine.
        self.graph = Some(graph);

        Ok(())
    }

    async fn execute_graph_functions(
        &mut self,
        graph: &mut RelationGraph<'a>,
        batch_size: usize,
    ) -> Result<IndexMap<FnRef, (SqlJoin, HashSet<FnRef>)>, EngineError> {
        // This hashmap will contain the ref to SqlJoin that can
        // be used to not have to recompute functions alongside their dependencies.
        let mut join_dep_map: IndexMap<FnRef, (SqlJoin, HashSet<FnRef>)> = IndexMap::new();

        let mut iter: AutoFnCallIterator<'_, '_> = graph.into_iter().into();
        // Iterate over all function references that are stored in the graph and that are needed in a topological ordering.
        while let Some(function_ref) = iter.next() {
            let fn_name = iter.get_manual_iter().get_function_name(&function_ref);
            // Get the module and the arguments that are linked to this function call.
            let (module, args) = iter.get_manual_iter().get_module_args(&function_ref);

            debug!("The function {fn_name} has to be executed.");
            // The list of all needed results to join for the selection query of this module.
            let mut needed_joins = Vec::new();
            // The set of already added function joined to the previous vec (used in order to not re-execute functions for no reason).
            let mut already_added = HashSet::new();

            // Find dependencies
            for expr in args {
                expr.map_primitives(&mut |prim| {
                    // For every other function calls located in the args of this function call
                    // add them (and their own function calls) to the list of needed joins
                    if let FnArg::FnCall(dependency) = prim {
                        add_rec_needed_joins(
                            dependency,
                            &mut needed_joins,
                            &mut already_added,
                            &join_dep_map,
                        );
                    }
                });
            }

            // Save the result as a join query for later uses
            // by saving the selected args first in a vector of comparison.
            {
                let mut join_conditions: Vec<Comparison<FnArg>> = Vec::with_capacity(args.len());
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
                        SqlJoin::new_from_name(
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

        Ok(join_dep_map)
    }

    fn build_extremal_result(
        &self,
        graph: &RelationGraph,
        flattened_extremal: ExtremalSelection<FnArg>,
        join_dep_map: IndexMap<FnRef, (SqlJoin, HashSet<FnRef>)>,
    ) -> SqlSelectQuery {
        let flattened_extremal_cp = flattened_extremal.clone();

        let mut all_expr_columns = Vec::from([
            (
                MathExpression::Primitif(FnArg::Constant(ConstantValue::Identifier(format!(
                    "{CANONICAL_TABLE_NAME}.{PK_NAME}"
                )))),
                None,
            )
                .into(),
            (Column {
                rename_as: Some("expr0".to_string()),
                value: ColumnValue::MathExpr(flattened_extremal_cp.aggregate_expr),
            }),
        ]);

        flattened_extremal_cp
            .grouping_expr
            .into_iter()
            .enumerate()
            .for_each(|(i, expr)| {
                all_expr_columns.push(Column {
                    rename_as: Some(format!("expr{}", i + 1)),
                    value: ColumnValue::MathExpr(expr),
                });
            });

        //  Collect all needed joins
        let all_joins: Vec<SqlJoin> = join_dep_map
            .into_iter()
            .map(|(_, (join, _))| join)
            .collect();

        let dataset_table =
            SqlTableSelection::new_rename(self.result.clone(), CANONICAL_TABLE_NAME);

        let mut inner_selection =
            SqlSelectQuery::select_columns_from_table(all_expr_columns, dataset_table);

        inner_selection.joins = all_joins;
        let flattened_extremal_cp = flattened_extremal.clone();
        // add window function (max or min)
        inner_selection.add_column(Column {
            value: ColumnValue::WindowFn(
                flattened_extremal_cp.class_type.to_string(),
                flattened_extremal_cp.aggregate_expr,
                flattened_extremal_cp.grouping_expr,
            ),
            rename_as: Some(String::from("extremal")),
        });

        // Outer query -> gives back all expressions their string values and reject all lines
        // where the aggregated expression is different from the window function's value.

        let mut all_function_columns = Vec::from([
            Column {
                rename_as: None,
                value: ColumnValue::MathExpr(MathExpression::Primitif(FnArg::Constant(
                    ConstantValue::Identifier(PK_NAME.to_string()),
                ))),
            },
            Column {
                rename_as: Some(
                    graph
                        .expression_to_string(&flattened_extremal.aggregate_expr)
                        .expect("everything present in the graph"),
                ),
                value: "expr0".to_string().into(),
            },
        ]);

        for (i, expr) in flattened_extremal.grouping_expr.into_iter().enumerate() {
            all_function_columns.push(Column {
                rename_as: Some(
                    graph
                        .expression_to_string(&expr)
                        .expect("everything present in the graph"),
                ),
                value: format!("expr{}", i + 1).to_string().into(),
            });
        }

        SqlSelectQuery::select_all_from_table(SqlTableSelection::new_rename(
            SqlSelectQuery::select_columns_from_table(
                all_function_columns,
                SqlTableSelection::new_rename(inner_selection, CANONICAL_TABLE_NAME),
            )
            .set_where_clause(Condition::Comparison(Comparison::<FnArg>::Equal(
                MathExpression::Primitif(FnArg::Constant(ConstantValue::Identifier(
                    "expr0".to_string(),
                ))),
                MathExpression::Primitif(FnArg::Constant(ConstantValue::Identifier(
                    "extremal".to_string(),
                ))),
                None,
            ))),
            CANONICAL_TABLE_NAME,
        ))
    }

    /// Builds the result query after updating the graph with a condition query.
    fn build_cond_result(
        &self,
        graph: &RelationGraph,
        flattened_cond: Condition<FnArg>,
        join_dep_map: IndexMap<FnRef, (SqlJoin, HashSet<FnRef>)>,
    ) -> SqlSelectQuery {
        // Fetch all columns needed for this query always starting with the signatures from the current dataset.
        let mut all_columns = Vec::from([(
            MathExpression::Primitif(FnArg::Constant(ConstantValue::Identifier(format!(
                "{CANONICAL_TABLE_NAME}.{PK_NAME}"
            )))),
            None,
        )]);

        //  Collect all needed joins
        let all_joins: Vec<SqlJoin> = join_dep_map
            .into_iter()
            .map(|(fn_ref, j)| {
                // as well as adding them to the resulting query outcome.
                all_columns.push((
                    MathExpression::Primitif(FnArg::Constant(ConstantValue::Identifier(format!(
                        "{}{}.{FUNCTION_OUTPUT_COL_NAME}",
                        fn_ref.0, fn_ref.1,
                    )))),
                    Some(graph.func_to_string(&fn_ref).expect("correct val")),
                ));
                j.0
            })
            .collect();

        let dataset_table =
            SqlTableSelection::new_rename(self.result.clone(), CANONICAL_TABLE_NAME);

        let mut selection =
            SqlSelectQuery::select_columns_with_rename_from_table(all_columns, dataset_table)
                .set_where_clause(flattened_cond);

        selection.set_distinct_values(true);
        // Thanks to the topological sort, this is already in the correct order.
        for join in all_joins {
            selection.add_join(join);
        }

        selection
    }

    /// Builds the result query after updating the graph with a list of additional expressions.
    fn build_expression_list_result(
        &self,
        graph: &RelationGraph,
        flattened_expressions: Vec<MathExpression<FnArg>>,
        join_dep_map: IndexMap<FnRef, (SqlJoin, HashSet<FnRef>)>,
    ) -> SqlSelectQuery {
        let mut res = SqlSelectQuery::select_column_from_table(
            format!("{CANONICAL_TABLE_NAME}.*"),
            None,
            SqlTableSelection::new_rename(self.result.clone(), CANONICAL_TABLE_NAME),
        );

        flattened_expressions.into_iter().for_each(|expr| {
            res.add_column(Column {
                rename_as: Some(
                    graph
                        .expression_to_string(&expr)
                        .expect("everything present in the graph"),
                ),
                value: ColumnValue::MathExpr(expr),
            });
        });

        //  Collect all needed joins
        res.joins.append(
            &mut join_dep_map
                .into_iter()
                .map(|(_, (join, _))| join)
                .collect(),
        );

        res
    }

    async fn get_result<O: SaveOutput>(&self, output: &mut O) -> Result<(), EngineError> {
        info!("Start fetching the result");
        match &self.db {
            #[cfg(any(feature = "sqlite", feature = "sqlite-unbundled"))]
            AllowedGraphDb::Sqlite(sqlite_db) => {
                sqlite_db.fetch_all_row_query(&self.result, output).await
            }
            #[cfg(feature = "postgres")]
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
    dataset_to_use.set_col_selection(format!("{CANONICAL_TABLE_NAME}.{PK_NAME}").into());

    match db {
        #[cfg(any(feature = "sqlite-unbundled", feature = "sqlite"))]
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
        #[cfg(feature = "postgres")]
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

fn flatten_selection(
    rel_graph: &mut RelationGraph,
    extremal_selection: ExtremalSelection<ParsedArgType>,
) -> Result<ExtremalSelection<FnArg>, RelationGraphError> {
    let class_type = extremal_selection.class_type;
    let aggregate_expr = flatten_expr(rel_graph, extremal_selection.aggregate_expr)?;
    let mut grouping_expr = Vec::new();

    for expr in extremal_selection.grouping_expr {
        grouping_expr.push(flatten_expr(rel_graph, expr)?);
    }
    Ok(ExtremalSelection::new(
        class_type,
        aggregate_expr,
        grouping_expr,
    ))
}

fn flatten_cond(
    rel_graph: &mut RelationGraph,
    cond: Condition<ParsedArgType>,
) -> Result<Condition<FnArg>, RelationGraphError> {
    Ok(match cond {
        Condition::Comparison(comparison) => flatten_comp(rel_graph, comparison)?.into(),
        Condition::Not(condition) => Condition::not(flatten_cond(rel_graph, *condition)?),
        Condition::Or(left_cond, right_cond) => Condition::or(
            flatten_cond(rel_graph, *left_cond)?,
            flatten_cond(rel_graph, *right_cond)?,
        ),
        Condition::And(left_cond, right_cond) => Condition::and(
            flatten_cond(rel_graph, *left_cond)?,
            flatten_cond(rel_graph, *right_cond)?,
        ),
    })
}

fn flatten_comp(
    rel_graph: &mut RelationGraph,
    comp: Comparison<ParsedArgType>,
) -> Result<Comparison<FnArg>, RelationGraphError> {
    Ok(match comp {
        Comparison::Greater(left_expr, right_expr) => Comparison::Greater(
            flatten_expr(rel_graph, left_expr)?,
            flatten_expr(rel_graph, right_expr)?,
        ),
        Comparison::GreaterEqual(left_expr, right_expr, e) => Comparison::GreaterEqual(
            flatten_expr(rel_graph, left_expr)?,
            flatten_expr(rel_graph, right_expr)?,
            e,
        ),
        Comparison::Less(left_expr, right_expr) => Comparison::Less(
            flatten_expr(rel_graph, left_expr)?,
            flatten_expr(rel_graph, right_expr)?,
        ),
        Comparison::LessEqual(left_expr, right_expr, e) => Comparison::LessEqual(
            flatten_expr(rel_graph, left_expr)?,
            flatten_expr(rel_graph, right_expr)?,
            e,
        ),
        Comparison::Equal(left_expr, right_expr, e) => Comparison::Equal(
            flatten_expr(rel_graph, left_expr)?,
            flatten_expr(rel_graph, right_expr)?,
            e,
        ),
        Comparison::NotEqual(left_expr, right_expr, e) => Comparison::NotEqual(
            flatten_expr(rel_graph, left_expr)?,
            flatten_expr(rel_graph, right_expr)?,
            e,
        ),
    })
}

fn flatten_expr(
    rel_graph: &mut RelationGraph,
    expr: MathExpression<ParsedArgType>,
) -> Result<MathExpression<FnArg>, RelationGraphError> {
    Ok(match expr {
        MathExpression::Primitif(p) => fill_graph_primitif(rel_graph, p)?.into(),
        MathExpression::Negation(math_expression) => {
            MathExpression::negation(flatten_expr(rel_graph, *math_expression)?)
        }
        MathExpression::Floor(math_expression) => {
            MathExpression::floor(flatten_expr(rel_graph, *math_expression)?)
        }
        MathExpression::Ceil(math_expression) => {
            MathExpression::ceil(flatten_expr(rel_graph, *math_expression)?)
        }
        MathExpression::Abs(math_expression) => {
            MathExpression::abs(flatten_expr(rel_graph, *math_expression)?)
        }
        MathExpression::Sqrt(math_expression) => {
            MathExpression::sqrt(flatten_expr(rel_graph, *math_expression)?)
        }
        MathExpression::BinOperation { left, op, right } => {
            let new_left = flatten_expr(rel_graph, *left)?;
            let new_right = flatten_expr(rel_graph, *right)?;
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
            let mut args = Vec::from([flatten_expr(rel_graph, *parsed_function.first_arg)?]);

            for arg in parsed_function.other_args {
                args.push(flatten_expr(rel_graph, arg)?);
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

/// Adds recursively all needed join for the `to_add` argument.
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
        panic!("join_dep_map: {join_dep_map:?} \n{to_add:?}: was supped to be present by the iterator topological sort logic")
    });
    // add all dependency for this join
    for dep in deps {
        add_rec_needed_joins(dep, res, already_in_res, join_dep_map);
    }
    // then add it
    res.push(join.clone());
    already_in_res.insert(to_add.clone());
}
