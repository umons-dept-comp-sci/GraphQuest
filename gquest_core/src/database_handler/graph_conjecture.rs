use std::collections::HashSet;

use crate::database_handler::{
    ArgType, CANONICAL_TABLE_NAME, PK_NAME, SqlComparison, SqlCondition, SqlSelectQuery,
    SqlTableSelection,
};

pub trait ToSql {
    fn to_sql(&self) -> String;
}

pub struct GraphConjecture {
    pub selection: ClassSelection,
    pub additional_condition: Option<SqlCondition>,
    pub conjecture_to_disprove: SqlCondition,
}

fn get_all_invariants_cond(cond: &SqlCondition) -> HashSet<String> {
    let mut res: HashSet<String> = HashSet::new();
    match cond {
        SqlCondition::Operation(sql_comparison) => match sql_comparison {
            SqlComparison::Greater(a, b)
            | SqlComparison::GreaterEqual(a, b)
            | SqlComparison::Less(a, b)
            | SqlComparison::LessEqual(a, b)
            | SqlComparison::Equal(a, b) => {
                if let ArgType::ColumnName(name) = a {
                    res.insert(name.to_string());
                }
                if let ArgType::ColumnName(name) = b {
                    res.insert(name.to_string());
                }
            }
        },
        SqlCondition::And(sql_condition, sql_condition1)
        | SqlCondition::Or(sql_condition, sql_condition1) => {
            res.extend(get_all_invariants_cond(sql_condition));
            res.extend(get_all_invariants_cond(sql_condition1));
        }
        SqlCondition::Not(sql_condition) => {
            res.extend(get_all_invariants_cond(sql_condition));
        }
        SqlCondition::AndVec(sql_condition, sql_conditions)
        | SqlCondition::OrVec(sql_condition, sql_conditions) => {
            res.extend(get_all_invariants_cond(sql_condition));
            for condition in sql_conditions {
                res.extend(get_all_invariants_cond(condition));
            }
        }
    };

    res
}

impl From<GraphConjecture> for SqlSelectQuery {
    fn from(value: GraphConjecture) -> Self {
        let mut all_columns = HashSet::new();

        // Get all from conditions :
        if let Some(add_cond) = &value.additional_condition {
            all_columns.extend(get_all_invariants_cond(add_cond));
        }
        all_columns.extend(get_all_invariants_cond(&value.conjecture_to_disprove));

        // Get all from selection :
        let mut selection_set = HashSet::new();
        selection_set.insert(value.selection.invariant_to_max.clone());
        selection_set.extend(value.selection.invariants_combination.clone());

        all_columns.extend(selection_set.clone());

        let mut all_columns = Vec::from_iter(all_columns);

        let all_eq_extremal_clause = SqlCondition::and_vec(
            SqlComparison::Equal(
                ArgType::ColumnName(format!("all_inv.{}", value.selection.invariant_to_max)),
                ArgType::ColumnName(format!("extremal.{}", value.selection.invariant_to_max)),
            ),
            selection_set
                .into_iter()
                .map(|column| {
                    SqlComparison::Equal(
                        ArgType::ColumnName(format!("all_inv.{column}")),
                        ArgType::ColumnName(format!("extremal.{column}")),
                    )
                })
                .collect(),
        );

        let extremal: SqlTableSelection = value.selection.into();

        let all_inv: SqlTableSelection = SqlTableSelection {
            selected_table: SqlSelectQuery::select_all_from_table(SqlTableSelection::new_join(
                all_columns[0].clone(),
                all_columns.split_off(1),
                PK_NAME,
                None,
            ))
            .into(),
            join_clause: None,
            rename_as: Some("all_inv".to_string()),
        };

        Self {
            select: vec!["all_inv.*".to_string()],
            from: vec![all_inv, extremal],
            where_clause: Some(SqlCondition::and_vec(all_eq_extremal_clause, {
                let mut res = if let Some(add_cond) = value.additional_condition {
                    vec![add_cond]
                } else {
                    vec![]
                };
                res.push(SqlCondition::not(value.conjecture_to_disprove));
                res
            })),
            group_by: Vec::new(),
            limit: None,
        }
    }
}

/// Example: max, "eci", vec!["n", "m"]
/// Means: the maximum value of eci for every combination of n and m
pub struct ClassSelection {
    class_type: ClassType,
    invariant_to_max: String,
    invariants_combination: Vec<String>,
}

impl ClassSelection {
    pub fn new(
        class_type: ClassType,
        extremal_inv: impl ToString,
        invariant_combination: Vec<impl ToString>,
    ) -> Self {
        Self {
            class_type,
            invariant_to_max: extremal_inv.to_string(),
            invariants_combination: Vec::from_iter(invariant_combination)
                .iter()
                .map(|f| f.to_string())
                .collect(),
        }
    }
}

impl From<ClassSelection> for SqlTableSelection {
    fn from(value: ClassSelection) -> Self {
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
                        value.invariant_to_max,
                        value.invariants_combination.clone(),
                        PK_NAME,
                        None,
                    )],
                    where_clause: None,
                    group_by: value.invariants_combination,
                    limit: None,
                }
                .into()
            },
            join_clause: None,
            rename_as: Some("extremal".to_string()),
        }
    }
}

pub enum ClassType {
    Min,
    Max,
    Extremal,
    None,
}

impl ToSql for ClassType {
    fn to_sql(&self) -> String {
        match self {
            ClassType::Min => "MIN",
            ClassType::Max => "MAX",
            ClassType::Extremal => todo!(),
            ClassType::None => "",
        }
        .to_string()
    }
}

/*
SELECT comp.canon FROM vertices, eci, comp, (SELECT vertices.value as vertices, MIN(eci.value) as eci FROM vertices, eci GROUP BY vertices.value) extremals where vertices.value = extremals.vertices and eci.value = extremals.eci and comp.value = 0;
 */

/*
SELECT comp.canon
    FROM vertices,
            m,
            eci,
            comp,
            (SELECT vertices.value as vertices,
                    m.value as m,
                    MIN(eci.value) as eci
                        FROM vertices,
                            m,
                            eci
                        GROUP BY vertices.value, m.value
            ) extremals
    where vertices.value = extremals.vertices
        and m.value = extremals.m
        and eci.value = extremals.eci
        and comp.value = 0;
*/

/*
WRONG (allInv doesn't exists)

SELECT extremals.* FROM
    (select canon,
        m.value as          m,
        comp.value as       comp,
        vertices.value as   vertices,
        eci.value as        eci
        FROM comp   INNER JOIN m using (canon)
                    INNER JOIN vertices using (canon)
                    INNER JOIN eci using (canon)) allInv,

    (select MAX(eci) as eci, vertices, m  FROM allInv GROUP BY vertices, m) extremals;

*/

/*
Correct but will force me to recompute allInv again :(


SELECT allInv.* FROM
    (select canon,
        m.value as          m,
        comp.value as       comp,
        d_nm.value as          d,
        vertices.value as   vertices,
        eci.value as        eci
        FROM comp   INNER JOIN m using (canon)
                    INNER JOIN vertices using (canon)
                    INNER JOIN eci using (canon)
                    INNER JOIN d_nm using (canon)) allInv,
    (select MAX(eci) as eci, vertices, m, d  FROM (select canon,
        m.value as          m,
        d_nm.value as          d,
        comp.value as       comp,
        vertices.value as   vertices,
        eci.value as        eci
        FROM comp   INNER JOIN m using (canon)
                    INNER JOIN vertices using (canon)
                    INNER JOIN eci using (canon)
                    INNER JOIN d_nm using (canon))
                        WHERE d >= 3
        GROUP BY vertices, m) extremals
    WHERE allInv.eci = extremals.eci and allInv.vertices = extremals.vertices and allInv.m = extremals.m
    AND comp = 0;

canon       | m | comp | d | vertices | eci
FJ]|w       |15|0|3|7|65
GTlzz{      |21|0|3|8|90
GJ\||{      |21|0|3|8|90
HJ\||}~     |28|0|3|9|119
HJ\z|}~     |28|0|3|9|119
ITmzz|~^w   |36|0|3|10|152
IJ\z|}~nw   |36|0|3|10|152
IJ\zz}~nw   |36|0|3|10|152

                    */

/*
FOR CONJ_1 :


SELECT allInv.* FROM
    (select canon,
        conj1.value as       conj1,
        r.value as          r,
        ag.value as        ag
        vertices.value as   vertices,
        FROM conj1  INNER JOIN vertices using (canon)
                    INNER JOIN ag using (canon)
                    INNER JOIN r using (canon)) allInv,
    (select MAX(ag) as ag, vertices, r  FROM (select canon,
        r.value as          r,
        conj1.value as       conj1,
        vertices.value as   vertices,
        ag.value as        ag
        FROM conj1  INNER JOIN vertices using (canon)
                    INNER JOIN ag using (canon)
                    INNER JOIN r using (canon))
        GROUP BY vertices, m) extremals
    WHERE allInv.ag = extremals.ag and allInv.vertices = extremals.vertices and allInv.r = extremals.r
    AND conj1 = 0;
                    */

/*
SELECT all_inv.* FROM (SELECT * FROM eci INNER JOIN vertices USING (canon) INNER JOIN m USING (canon)) all_inv,
                     (SELECT vertices, m, MIN(eci) as eci FROM (eci INNER JOIN vertices USING (canon) INNER JOIN m USING (canon)) GROUP BY vertices, m) extremal
            WHERE all_inv.eci = extremal.eci AND all_inv.m = extremal.m AND all_inv.vertices = extremal.vertices;

SELECT vertices, m, MIN(eci) as eci FROM (eci INNER JOIN vertices USING (canon) INNER JOIN m USING (canon)) GROUP BY vertices, m




SELECT all_inv.* FROM (SELECT * FROM eci INNER JOIN vertices USING (canon) INNER JOIN m USING (canon) INNER JOIN d_nm USING (CANON) INNER JOIN comp using (CANON)) all_inv,
                     (SELECT vertices, m, MAX(eci) as eci FROM (eci INNER JOIN vertices USING (canon) INNER JOIN m USING (canon)) GROUP BY vertices, m) extremal
            WHERE all_inv.eci = extremal.eci AND all_inv.m = extremal.m AND all_inv.vertices = extremal.vertices and comp = 0;

SELECT all_inv.* FROM (SELECT * FROM (comp INNER JOIN d_nm USING (canon) INNER JOIN eci USING (canon) INNER JOIN m USING (canon) INNER JOIN vertices USING (canon))) as all_inv,
                     (SELECT vertices, m, MAX(eci) as eci FROM (eci INNER JOIN vertices USING (canon) INNER JOIN m USING (canon)) GROUP BY vertices, m) as extremal
            WHERE (d_nm >= 3) AND (NOT (comp = 1));

// CORECT !!! !! ! !!
GTlzz{|90.0|8.0|21.0|3.0|0.0
GJ\||{|90.0|8.0|21.0|3.0|0.0
FJ]|w|65.0|7.0|15.0|3.0|0.0
*/
