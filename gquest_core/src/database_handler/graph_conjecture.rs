pub trait ToSql {
    fn to_sql(&self) -> String;
}

struct GraphConjecture {
    selection: ClassSelection,
    // inv_conditions: Quer,
    // should_meet: InvariantCondition,
}

impl ToSql for GraphConjecture {
    fn to_sql(&self) -> String {
        todo!()
    }
}


/// Example: max, "eci", vec!["n", "m"]
/// Means: the maximum value of eci for every combination of n and m
struct ClassSelection {
    class_type: ClassType,
    invariant_to_max: String,
    invariants_combination: Vec<String>,
}


enum ClassType {
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

// fn test() {
//     let cond = InvariantCondition::or(
//         InvariantOperation::greater_equal("d", "3"),
//         InvariantCondition::and(
//             InvariantOperation::greater("n", "4"),
//             InvariantOperation::less("d", "1"),
//         ),
//     );
//     let conjecture = InvariantCondition::not(InvariantOperation::equal("comp", "1"));

//     let conjecture_query = GraphConjecture {
//         selection: ClassSelection {
//             class_type: ClassType::Max,
//             invariant_to_max: "eci".to_string(),
//             invariants_combination: vec!["n".to_string(), "m".to_string()],
//         },
//         inv_conditions: cond,
//         should_meet: conjecture,
//     };
// }

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

