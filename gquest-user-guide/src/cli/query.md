## Querying the dataset

The `gquest query` command is used to execute a query on the current dataset and display the resulting graphs.

```bash
gquest query [OPTIONS] <QUERY> [CONFIG_FILE]
```

### `<QUERY>`

The required `QUERY` argument contains the [query](../query_syntax.md) to execute on the database.

For example:

```bash
gquest query "n == 5 and is_connected"
```

selects all the connected graphs with 5 vertices present in the dataset.



#### Displayed result

By default, gquest returns a table containing resulting graphs alongside the values needed for the last condition or extremal selection. 

For instance, take the following example:
```bash
gquest query "iso(comp,G)" configs.json -f markdown
```

which will output a table similar to:

| **i** | **sig** | **comp** | **iso(comp,G)** |
| ----- | ------- | -------- | --------------- |
| ...   | ...     | ...      | ...             |

with `i`, a column containing the index of the displayed row.


### `[CONFIG_FILE]`

The optional `CONFIG_FILE` argument specifies the path to the [configuration file](../configs.md) containing the definitions of the modules and functions used by the query.

If no configuration file is specified, GraphQuest uses `configs.json` by default:

```bash
gquest query "chromatic_nb > 3" configs.json
```

### Options


#### Finding counterexamples

The `-c` option can be used to negate the last condition of the provided queries without modifying the previous filters.

For example:

```bash
gquest query "n == 5" -c
```

will return all graph with an order from 5, while the query:

```bash
gquest query "is_connected -> n == 5" -c
```

will return all *connected* graph with an order from 5.


> [!CAUTION]
> Currently, trying to get the counter of an extremal selection will result in an error of type `not yet implemented` and crash the application.
> This is because this feature is still a work in progress, so this should be fixed in a future release.


#### Limiting displayed rows

The `-p` option can be used to display only a portion of the result:

```bash
-p <PARTIAL>
```

The value has the form `n:m`, where `n` is the number of rows displayed at the beginning of the result and `m` is the number of rows displayed at the end.

For example:

```bash
gquest query -p 5:5 "n == 5"
```

displays the first 5 and last 5 rows of the result, while:

```bash
gquest query -p :10 "n == 5"
```

only displays the last 10 rows of the result.

This only affects what is displayed; it does not limit the query itself.

#### Hiding the index column

By default, the result table includes an index column. The `-n` or `--no-id` option can be used to hide it:

```bash
gquest query --no-id "n == 5"
```

#### Choosing the output format

The `-f` or `--format` option specifies how the result is displayed:

```bash
-f <FORMAT>
```

The available formats are:

* `table` — formatted table output (default)
* `plain-text` — plain-text output
* `latex` — LaTeX table
* `markdown` — Markdown table

For example:

```bash
gquest query --format markdown "n == 5"
```

produces a Markdown representation of the result.

#### Saving the result to a CSV file

The `-o` or `--output-path` option stores the complete result as a CSV file:

```bash
-o <OUTPUT_PATH>
```

For example:

```bash
gquest query --output-path results.csv "n == 5"
```

Unlike `-p`, this option stores the **entire query result**, rather than only the rows displayed in the terminal.

The `-s` option can be used to specify the separator used in the CSV file. The default separator is `,`:

```bash
gquest query -s ";" --output-path results.csv "n == 5"
```

#### Adding expressions to the result

The `-a` or `--add-expr` option allows additional expressions to be appended to the query result:

```bash
-a <ADD_EXPR>
```

For example:

```bash
gquest query -a "max_degree - chromatic_nb" "n == 5"
```

adds the value of `max_degree - chromatic_nb` as an additional column in the displayed result.

This can be useful when an expression is needed only for displaying or analysing the result and does not need to be part of the filtering condition itself.
