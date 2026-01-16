use std::{collections::HashSet, vec};

use sqlx::Database;
use thiserror::Error;

use crate::database_handler::{
    ArgType, CANONICAL_TABLE_NAME, DbQuerySystem, EXTREMAL_TABLE_NAME, FULL_TABLE_NAME, PK_NAME,
    SqlComparison, SqlCondition, SqlSelectQuery, SqlTableSelection,
};

pub trait ToSql {
    fn to_sql(&self) -> String;
}

/// Represents a query that could be used to find counterexamples from a [`crate::database_handler::GraphDatabase`].
///
/// Can be turned into a [`SqlSelectQuery`] in order to be executed by a database system.
/// # Errors
/// The given [`SqlCondition`]s cannot contain an [`SqlCondition::Exists`] clause since finding invariant names would be harder as of now.
#[derive(Clone, Debug, PartialEq)]
pub struct ExtremalCounterQuery {
    /// The condition that the graph of the dataset have to respect for the conjecture.
    pub selection: ClassSelection,
    /// The optional additional condition that can further restrict the graph to explore.
    pub additional_condition: Option<SqlCondition>,
    /// The conjecture to disprove.
    /// When building the query, this will be encapslulated as [`SqlCondition::Not`] in order to try to find a counter example.
    pub conjecture_to_disprove: SqlCondition,
}

/// Gets all the names of the invariants to compute in order to find the extremal graphs.
pub fn get_extremal_invariants(
    selection: &ClassSelection,
    additional_condition: &Option<SqlCondition>,
) -> HashSet<String> {
    let mut all_inv = HashSet::new();
    all_inv.insert(selection.invariant_to_max.clone());
    all_inv.extend(selection.invariants_combination.clone());

    if let Some(additional_cond) = &additional_condition {
        all_inv.extend(additional_cond.get_all_identifiers());
    }

    all_inv
}

impl ExtremalCounterQuery {
    /// Gets all the names of the invariants to compute in order to find the extremal graphs. (So the invariants from the conjecture are not taken into account here).
    /// * If the result isn't empty this means that the conjecture invariants could only be computed using these extremal graphs thus greatly reducing the number of values to compute.
    /// * Otherwise, it means that the entire Dataset should be computed to find a counter example for this conjecture.
    pub fn get_invariants_to_compute(&self) -> HashSet<String> {
        get_extremal_invariants(&self.selection, &self.additional_condition)
    }

    pub fn as_sql<DB>(&self) -> String
    where
        DB: Database + DbQuerySystem<DB>,
    {
        let select_query: SqlSelectQuery = self.into();

        select_query.to_sql::<DB>()
    }

    /// Gets all the names of the invariants specified in the conjecture to disprove.
    pub fn get_invariants_from_conjecture(&self) -> HashSet<String> {
        self.conjecture_to_disprove.get_all_identifiers()
    }

    //FIXME: This function is too similar to another one
    pub fn get_extremal_input_selection(&self) -> SqlSelectQuery {
        // Find all tables needed for this invariant by looking at the name of every selected column/invariant.
        let mut all_columns = HashSet::new();

        // Get all from conditions :
        if let Some(add_cond) = &self.additional_condition {
            all_columns.extend(add_cond.get_all_identifiers());
        }

        // Get all from selection :
        let mut selection_set = HashSet::new();

        selection_set.insert(self.selection.invariant_to_max.clone());
        selection_set.extend(self.selection.invariants_combination.clone());

        all_columns.extend(selection_set.clone());

        let mut all_columns = Vec::from_iter(all_columns);

        let all_eq_extremal_clause = SqlCondition::and_vec(
            SqlComparison::Equal(
                ArgType::Identifier(format!(
                    "{FULL_TABLE_NAME}.{}",
                    self.selection.invariant_to_max
                ))
                .into(),
                ArgType::Identifier(format!(
                    "{EXTREMAL_TABLE_NAME}.{}",
                    self.selection.invariant_to_max
                ))
                .into(),
            ),
            selection_set
                .into_iter()
                .map(|column| {
                    SqlComparison::Equal(
                        ArgType::Identifier(format!("{FULL_TABLE_NAME}.{column}")).into(),
                        ArgType::Identifier(format!("{EXTREMAL_TABLE_NAME}.{column}")).into(),
                    )
                })
                .collect(),
        );

        let extremal: SqlTableSelection = self.selection.clone().into();

        let all_inv: SqlTableSelection = SqlTableSelection {
            selected_table: SqlSelectQuery::select_all_from_table(SqlTableSelection::new_join(
                all_columns[0].clone(),
                all_columns.split_off(1),
                PK_NAME,
                None,
            ))
            .into(),
            join_clause: None,
            rename_as: Some(FULL_TABLE_NAME.to_string()),
        };

        let mut query = SqlSelectQuery {
            select: vec![format!("{FULL_TABLE_NAME}.{PK_NAME}")],
            from: vec![all_inv, extremal],
            where_clause: Some(all_eq_extremal_clause),
            group_by: Vec::new(),
            limit: None,
        };
        if let Some(add_cond) = &self.additional_condition {
            // To prevent any ambiguity, restrict to a table
            let mut safe_add_cond = add_cond.clone();
            safe_add_cond.add_prefix_identifier(format!("{FULL_TABLE_NAME}."));
            query.add_and(safe_add_cond);
        }
        query
    }

    fn get_extremal_graphs(
        selection: &ClassSelection,
        additional_condition: &Option<SqlCondition>,
        conjecture_to_disprove: Option<&SqlCondition>,
    ) -> SqlSelectQuery {
        // Find all tables needed for this invariant by looking at the name of every selected column/invariant.
        let mut all_columns = HashSet::new();

        // Get all from conditions :
        if let Some(add_cond) = &additional_condition {
            all_columns.extend(add_cond.get_all_identifiers());
        }
        if let Some(conj_disp) = conjecture_to_disprove {
            all_columns.extend(conj_disp.get_all_identifiers());
        }

        // Get all from selection :
        let mut selection_set = HashSet::new();

        selection_set.insert(selection.invariant_to_max.clone());
        selection_set.extend(selection.invariants_combination.clone());

        all_columns.extend(selection_set.clone());

        let mut all_columns = Vec::from_iter(all_columns);

        let all_eq_extremal_clause = SqlCondition::and_vec(
            SqlComparison::Equal(
                ArgType::Identifier(format!("{FULL_TABLE_NAME}.{}", selection.invariant_to_max))
                    .into(),
                ArgType::Identifier(format!(
                    "{EXTREMAL_TABLE_NAME}.{}",
                    selection.invariant_to_max
                ))
                .into(),
            ),
            selection_set
                .into_iter()
                .map(|column| {
                    SqlComparison::Equal(
                        ArgType::Identifier(format!("{FULL_TABLE_NAME}.{column}")).into(),
                        ArgType::Identifier(format!("{EXTREMAL_TABLE_NAME}.{column}")).into(),
                    )
                })
                .collect(),
        );

        let extremal: SqlTableSelection = selection.into();

        let all_inv: SqlTableSelection = SqlTableSelection {
            selected_table: SqlSelectQuery::select_all_from_table(SqlTableSelection::new_join(
                all_columns[0].clone(),
                all_columns.split_off(1),
                PK_NAME,
                None,
            ))
            .into(),
            join_clause: None,
            rename_as: Some(FULL_TABLE_NAME.to_string()),
        };

        let mut query = SqlSelectQuery {
            select: vec![format!("{FULL_TABLE_NAME}.*")],
            from: vec![all_inv, extremal],
            where_clause: Some(all_eq_extremal_clause),
            group_by: Vec::new(),
            limit: None,
        };
        if let Some(add_cond) = additional_condition {
            // To prevent any ambiguity, restrict to a table
            let mut safe_add_cond = add_cond.clone();
            safe_add_cond.add_prefix_identifier(format!("{FULL_TABLE_NAME}."));
            query.add_and(safe_add_cond);
        }
        query
    }
}

impl From<ExtremalCounterQuery> for SqlSelectQuery {
    fn from(value: ExtremalCounterQuery) -> Self {
        (&value).into()
    }
}
impl From<&ExtremalCounterQuery> for SqlSelectQuery {
    fn from(value: &ExtremalCounterQuery) -> Self {
        let mut query = ExtremalCounterQuery::get_extremal_graphs(
            &value.selection,
            &value.additional_condition,
            Some(&value.conjecture_to_disprove),
        );
        // The conjecture to disprove might cause some ambiguous column name
        // for this we rename it to prevent any issues
        let mut safe_conjecture_disprove = value.conjecture_to_disprove.clone();
        safe_conjecture_disprove.add_prefix_identifier(format!("{FULL_TABLE_NAME}."));
        query.add_and(SqlCondition::not(safe_conjecture_disprove));

        query
    }
}

/// Example: max(`eci`; `n`, `m`)
///
/// Means: the maximum value of `eci` for every combination of `n` and `m`
#[derive(Clone, Debug, PartialEq)]
pub struct ClassSelection {
    class_type: ClassType,
    invariant_to_max: String,
    invariants_combination: Vec<String>,
}
#[derive(Error, Debug)]
pub enum ClassSelectionError {
    #[error("Cannot group the following invariant with itself \"{0}\"")]
    CombineWithItself(String),
    #[error("Invariant appears twice : \"{0}\"")]
    DuplicateInv(String),
}

impl ClassSelection {
    pub fn new(
        class_type: ClassType,
        extremal_inv: impl ToString,
        invariants_combination: Vec<impl ToString>,
    ) -> Result<Self, ClassSelectionError> {
        let invariant_to_max = extremal_inv.to_string();
        let invariants_combination: Vec<String> = Vec::from_iter(invariants_combination)
            .iter()
            .map(|f| f.to_string())
            .collect();

        if invariants_combination.contains(&invariant_to_max) {
            return Err(ClassSelectionError::CombineWithItself(invariant_to_max));
        }
        if let Some(val) = check_all_unique(&invariants_combination) {
            return Err(ClassSelectionError::DuplicateInv(val));
        }

        Ok(Self {
            class_type,
            invariant_to_max,
            invariants_combination,
        })
    }
}

impl From<ClassSelection> for SqlTableSelection {
    fn from(value: ClassSelection) -> Self {
        (&value).into()
    }
}

impl From<&ClassSelection> for SqlTableSelection {
    fn from(value: &ClassSelection) -> Self {
        SqlTableSelection {
            selected_table: {
                let mut select = value.invariants_combination.clone();
                select.push(format!(
                    "{}({}) as {}",
                    value.class_type.to_sql(),
                    value.invariant_to_max,
                    value.invariant_to_max
                ));

                SqlSelectQuery {
                    select,
                    from: vec![SqlTableSelection::new_join(
                        value.invariant_to_max.clone(),
                        value.invariants_combination.clone(),
                        PK_NAME,
                        None,
                    )],
                    where_clause: None,
                    group_by: value.invariants_combination.clone(),
                    limit: None,
                }
                .into()
            },
            join_clause: None,
            rename_as: Some(EXTREMAL_TABLE_NAME.to_string()),
        }
    }
}

fn check_all_unique(iter: &Vec<String>) -> Option<String> {
    let mut hash_set = HashSet::new();

    for val in iter {
        if !hash_set.insert(val.to_string()) {
            return Some(val.to_string());
        }
    }

    None
}

#[derive(Clone, Debug, PartialEq)]
pub enum ClassType {
    Min,
    Max,
}

impl ToSql for ClassType {
    fn to_sql(&self) -> String {
        match self {
            ClassType::Min => "MIN",
            ClassType::Max => "MAX",
        }
        .to_string()
    }
}

/// Gets a selection query that can be use to get all graphs that respect the given condition.
/// If no identifier is present in the condition then a simple selection of the dataset with the condition is returned.
pub fn select_all_graph_cond(cond: impl Into<SqlCondition>) -> SqlSelectQuery {
    let cond = cond.into();
    let invariants = cond.get_all_identifiers();

    if invariants.is_empty() {
        SqlSelectQuery::select_all_from_table(CANONICAL_TABLE_NAME.to_string())
    } else {
        let invariant = Vec::from_iter(invariants);
        SqlSelectQuery::select_all_from_table(SqlTableSelection::new_join(
            CANONICAL_TABLE_NAME,
            invariant,
            PK_NAME,
            None,
        ))
    }
    .set_where_clause(cond)
}

/// Returns a query that can be used to fetch all extremal graphs
pub fn select_all_extremal_graphs(
    selection: &ClassSelection,
    additional_condition: &Option<SqlCondition>,
) -> SqlSelectQuery {
    ExtremalCounterQuery::get_extremal_graphs(selection, additional_condition, None)
}
