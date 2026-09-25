SELECT
    sig,
    all_expr.expr0 as "m",
    all_expr.expr1 as "n"
FROM
    (
        (
            SELECT
                dataset.sig,
                m0.output as "expr0",
                n0.output as "expr1"
            FROM
                (
                    (
                        SELECT
                            *
                        FROM
                            (dataset)
                    )
                ) as dataset
                JOIN (n) as n0 ON n0.sig = dataset.sig
                JOIN (m) as m0 ON m0.graph = dataset.sig
        )
    ) as all_expr
    JOIN (
        (
            SELECT
                MAX(expr0) as "expr0",
                expr1
            FROM
                (
                    (
                        SELECT
                            dataset.sig,
                            m0.output as "expr0",
                            n0.output as "expr1"
                        FROM
                            (
                                (
                                    SELECT
                                        *
                                    FROM
                                        (dataset)
                                )
                            ) as dataset
                            JOIN (n) as n0 ON n0.sig = dataset.sig
                            JOIN (m) as m0 ON m0.graph = dataset.sig
                    )
                )
            GROUP BY
                expr1
        )
    ) as extr ON all_expr.expr0 = extr.expr0
    AND all_expr.expr1 = extr.expr1;