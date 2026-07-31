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
    }, database_handler::{
        AllowedGraphDb, CANONICAL_TABLE_NAME, FUNCTION_OUTPUT_COL_NAME, GraphDbRuntimeError, GraphDbStartupError, PK_NAME, SqlJoin, SqlSelectQuery, SqlTableSelection, VERTICES_TABLE_NAME,
    }, parser::parsed_expression::{Comparison, Condition, MathExpression, ParsedArgType}, utils::{SaveOutput, config_file2::ConfigFile, subject::Observer},
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

    pub async fn exec_condition_no_multithread<O: SaveOutput>(
        &mut self,
        cond: Condition<ParsedArgType>,
        output: &mut O,
    ) -> Result<(), WorkplaceError> {
        let mut rel_graph = RelationGraph::new(self.config.get_module_refs());
        let flattened_cond = Self::fill_graph_cond(&mut rel_graph, cond)?;
        // This hashmap will contain the ref to SqlJoin that can
        // be used to not have to recompute functions for no reasons alongside their dependencies.
        let mut join_dep_map: IndexMap<FnRef, (SqlJoin, HashSet<FnRef>)> = IndexMap::new();

        let mut iter: AutoFnCallIterator<'_> = rel_graph.into_iter().into();

        while let Some(re) = iter.next() {
            let (module, args) = iter.get_module_args(&re);

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
        }

        let mut all_output = vec![MathExpression::Primitif(FnArg::Constant(
            ConstantValue::Identifier(format!("{CANONICAL_TABLE_NAME}.{PK_NAME}")),
        ))];

        let all_joins: Vec<SqlJoin> = join_dep_map
            .into_iter()
            .map(|(fn_ref, j)| {
                all_output.push(MathExpression::Primitif(FnArg::Constant(
                    ConstantValue::Identifier(format!("{}{}.{FUNCTION_OUTPUT_COL_NAME} as {}{}", fn_ref.0, fn_ref.1, fn_ref.0, fn_ref.1)),
                )));
                j.0
            })
            .collect();

        let mut selection =
            SqlSelectQuery::select_columns_from_table(all_output, CANONICAL_TABLE_NAME)
                .set_where_clause(flattened_cond);
        selection.set_distinct_values(false);
        // Thanks to the topological sort, this is already in the correct order.
        for join in all_joins {
            selection.add_join(join);
        }

        self.fetch_all_row_query(&selection, output).await?;

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
            ParsedArgType::Dataset => FnArg::Dataset,
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

fn add_rec_needed_joins(
    to_add: &FnRef,
    res: &mut Vec<SqlJoin>,
    already_in_res: &mut HashSet<FnRef>,
    join_dep_map: &IndexMap<FnRef, (SqlJoin, HashSet<FnRef>)>,
) {
    if already_in_res.contains(to_add) {
        return;
    }
    let add_join_and_dep = join_dep_map
        .get(to_add)
        .expect("present by the iterator topological sort logic");
    // add all dependency for this join
    for dep in &add_join_and_dep.1 {
        add_rec_needed_joins(dep, res, already_in_res, join_dep_map);
    }
    // then add it
    res.push(add_join_and_dep.0.clone());
    already_in_res.insert(to_add.clone());
}
