use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

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
        AllowedGraphDb, CANONICAL_TABLE_NAME, FUNCTION_OUTPUT_COL_NAME, GraphDbRuntimeError,
        GraphDbStartupError, PK_NAME, SqlJoin, SqlSelectQuery, SqlTableSelection,
        VERTICES_TABLE_NAME,
    },
    parser::parsed_expression::{
        Comparison, Condition, MathExpression, ParsedArgType, QueryStatement,
    },
    utils::{SaveOutput, config_file2::ConfigFile, subject::Observer},
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

    pub async fn exec_query<O: SaveOutput>(
        &mut self,
        mut query: QueryStatement,
        output: &mut O,
    ) -> Result<(), EngineError> {
        let mut cond_engine = ConditionEngine::new(&mut self.db, self.config.get_module_refs());

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
                    println!("MOVING ON TO {query:?}");
                }
            };
        }

        cond_engine.get_result(output).await
    }

    pub async fn exec_condition<O: SaveOutput>(
        &mut self,
        cond: Condition<ParsedArgType>,
        output: &mut O,
    ) -> Result<(), EngineError> {
        let mut cond_engine = ConditionEngine::new(&mut self.db, self.config.get_module_refs());

        cond_engine
            .add_new_cond(cond, self.config.get_batch_size())
            .await?;

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
            // modules: Some(modules),
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
        // Borrow the relation graph to use it as a mutable variable alongside the engine.
        let mut graph = self.graph.take().expect("present");
        {
            // Add cond to the relation graph.
            let flattened_cond = fill_graph_cond(&mut graph, cond)?;

            // This hashmap will contain the ref to SqlJoin that can
            // be used to not have to recompute functions for no reasons alongside their dependencies.
            let mut join_dep_map: IndexMap<FnRef, (SqlJoin, HashSet<FnRef>)> = IndexMap::new();

            // Execute the necessary modules if needed :

            let mut iter: AutoFnCallIterator<'_, '_> = graph.into_iter().into();

            while let Some(re) = iter.next() {
                let (module, args) = iter.get_module_args(&re);
                println!(
                    "DOING {re:?} with args: {args:?} \n* Already computed: {:?} | join dep map : {:?} | result_cond: {}",
                    self.already_computed,
                    join_dep_map,
                    self.result.to_sql::<Sqlite>()
                );
                println!();

                // Fetch all needed function to join for this module.
                let mut needed_joins = Vec::new();
                let mut already_added = HashSet::new();
                for expr in args {
                    for prim in expr.get_all_primitives_rec() {
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

                // Save the result as a join query for later uses (as well as its dependencies).
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

                join_dep_map.insert(
                    re.clone(),
                    (
                        SqlJoin::new(re.0.clone(), format!("{}{}", re.0, re.1), join_conditions),
                        already_added,
                    ),
                );

                if self.already_computed.contains(&re) {
                    println!("\t -=-=-=-> but it was already computed so i'm skiping this");
                    continue;
                }

                // Save this function as already computed.
                self.already_computed.insert(re.clone());

                // Execute the function
                self.compute_module(&re, module, args, needed_joins, batch_size, None)
                    .await?;
            }

            // Update the result query :

            let mut all_output = vec![MathExpression::Primitif(FnArg::Constant(
                ConstantValue::Identifier(format!("{CANONICAL_TABLE_NAME}.{PK_NAME}")),
            ))];

            let all_joins: Vec<SqlJoin> = join_dep_map
                .into_iter()
                .map(|(fn_ref, j)| {
                    all_output.push(MathExpression::Primitif(FnArg::Constant(
                        ConstantValue::Identifier(format!(
                            "{}{}.{FUNCTION_OUTPUT_COL_NAME} as {}{}",
                            fn_ref.0, fn_ref.1, fn_ref.0, fn_ref.1
                        )),
                    )));
                    j.0
                })
                .collect();

            let mut new_dataset = self.result.clone();
            new_dataset.set_col_selection(PK_NAME);
            let dataset_table =
                SqlTableSelection::new_rename(self.result.clone(), CANONICAL_TABLE_NAME);

            let mut selection =
                SqlSelectQuery::select_columns_from_table(all_output, dataset_table)
                    .set_where_clause(flattened_cond);

            // selection.set_distinct_values(false);
            // Thanks to the topological sort, this is already in the correct order.
            for join in all_joins {
                selection.add_join(join);
            }

            self.result = selection;
        }
        // Give ownership back of the graph to the engine.
        graph.set_all_unused();
        self.graph = Some(graph);

        Ok(())
    }

    async fn get_result<O: SaveOutput>(&self, output: &mut O) -> Result<(), EngineError> {
        match &self.db {
            AllowedGraphDb::Sqlite(sqlite_db) => {
                sqlite_db.fetch_all_row_query(&self.result, output).await
            }
            AllowedGraphDb::Postgres(pg_db) => {
                pg_db.fetch_all_row_query(&self.result, output).await
            }
        }?;
        Ok(())
    }

    async fn compute_module(
        &mut self,
        fn_ref: &FnRef,
        module: &Module,
        args: &[MathExpression<FnArg>],
        join_list: Vec<SqlJoin>,
        batch_size: usize,
        optional_obs: Option<&mut dyn Observer>,
    ) -> Result<(), GraphDbRuntimeError> {
        let mut dataset_to_use = self.result.clone();
        dataset_to_use.set_col_selection(format!("{CANONICAL_TABLE_NAME}.{PK_NAME}"));

        match self.db {
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
                        Some(self.result.clone()),
                        join_list,
                        batch_size,
                        optional_obs,
                    )
                    .await?;
            }
        }
        Ok(())
    }
}

// ___________________________ UTILS ___________________________

fn fill_graph_cond(
    rel_graph: &mut RelationGraph,
    cond: Condition<ParsedArgType>,
) -> Result<Condition<FnArg>, EngineError> {
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
) -> Result<Comparison<FnArg>, EngineError> {
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
) -> Result<MathExpression<FnArg>, EngineError> {
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
) -> Result<FnArg, EngineError> {
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

fn add_rec_needed_joins(
    to_add: &FnRef,
    res: &mut Vec<SqlJoin>,
    already_in_res: &mut HashSet<FnRef>,
    join_dep_map: &IndexMap<FnRef, (SqlJoin, HashSet<FnRef>)>,
) {
    if already_in_res.contains(to_add) {
        return;
    }
    let add_join_and_dep = join_dep_map.get(to_add).unwrap_or_else(|| {
        panic!("join_dep_map: {join_dep_map:?} \n{to_add:?}: present by the iterator topological sort logic")
    });
    // add all dependency for this join
    for dep in &add_join_and_dep.1 {
        add_rec_needed_joins(dep, res, already_in_res, join_dep_map);
    }
    // then add it
    res.push(add_join_and_dep.0.clone());
    already_in_res.insert(to_add.clone());
}
