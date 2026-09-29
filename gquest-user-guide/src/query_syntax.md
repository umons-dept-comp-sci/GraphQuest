# Query syntax

GraphQuest queries are composed of values, identifiers, functions, expressions, and logical conditions. This section describes the syntax accepted by the GraphQuest query language.

## Primitive values

GraphQuest supports the following primitive values:

| Type                  | Examples        |
| --------------------- | --------------- |
| Integer               | `0`, `42`, `-3` |
| Floating-point number | `3.14`, `-0.5`  |
| String                | `"hello"`       |
| Dataset argument      | `G`             |


### Dataset Argument

The dataset argument, denoted by `G`, represents the graph currently being
evaluated.

It can be passed to functions that expect a graph as an argument, for example:
* `m(G)`
* `chromatic_nb(G)`
* `iso(G, comp(G))`


The dataset argument can also be omitted for [invariants](#invariant).


## Functions & identifiers



### Functions

A function takes one or more argument and returns a value. Each function is linked to a [module](modules.md) and is defined in a given [configuration file](configs.md).

For example:
* `m(G)`,
* `d(n(G),m(G))` (altough we will see in the next [section](#invariant) that this can be simplified),
* `d(2,4)`,
* `iso(G, comp(G))`.

### Invariant
Invariants refer to functions that take a graph as their only argument.

A function that takes only one argument of type Graph can be written without explicitly passing the dataset argument `G`.

For example:
* `m(G)` is equivalent to `m`, and
* `chromatic_nb(G)` is equivalent to `chromatic_nb`.

This shorthand allows graph invariants to be used directly in expressions and conditions without explicitly specifying `G`.


For example, instead of having to write `d(n(G),m(G))`, we can simply write `d(n,m)`. 


This syntactic sugar can be used anywhere that allows the use of functions.

## Expressions

Expressions can be constructed using arithmetic operators.

| **Operations:** | **Syntax:**        |
| --------------- | ------------------ |
| Addition        | x + y              |
| Multiplication  | x * y              |
| Subtraction     | x - y              |
| Power           | x ** y `and` x ^ y |
| Modulo          | x % y              |
| Division        | x / y              |
| Floor division  | x // y             |


For example:
* n + 1
* m * 2
* chromatic_nb + max_degree
* (n + m) // 2



### Unary functions

GraphQuest also provides several built-in unary functions. These functions take a number as an argument and return another number.

| **Unary functions:** | **Syntax:** |
| -------------------- | ----------- |
| Floor                | floor(x)    |
| Ceil                 | ceil(x)     |
| Absolute             | abs(x)      |
| Square Root          | sqrt(x)     |
| Negation             | -x          |


For example:
```
floor(m / n)
abs(chromatic_nb - max_degree)
sqrt(n)
-max_degree
```

## Conditions & comparisons

Expressions can be compared using the following operators:

| Operator:        | **Syntax:** |
| ---------------- | ----------- |
| Equal            | ==          |
| Not Equal        | !=          |
| Less             | <           |
| Less or Equal    | <=          |
| Greater          | >           |
| Greater or Equal | >=          |

For example:
```
n == 5
m > 10
chromatic_nb <= max_degree
```


Functions that return a numerical value can be used directly as conditions. When a function is used without an explicit comparison, GraphQuest currently interprets it as a comparison with 1. So for example:
* `is_complete` is equivalent to `is_complete = 1`, and
* `iso(G, comp)` is equivalent to `iso(G, comp) = 1`


> [!warning]
> Currently, any function that is used as a condition without an explicit comparison is implicitly compared to 1, even if they do not return a numerical value.
> 
> For example, `is_connected and d(n,m)` is equivalent to `is_connected = 1 and d(n,m) = 1`
> 
> Therefore, this query selects connected graphs for which d(n,m) evaluates to 1.


### Logical operators

Comparisons can be combined using logical operators:

| Operator:       | **Syntax:** |
| --------------- | ----------- |
| **And**         | x and y     |
| **Or**          | x or y      |
| **Xor**         | x xor y     |
| **Negation**    | not x       |
| **Implication** | x ==> y     |
| **Equivalence** | x <==> x    |

For example:
```
n == 5 and m > 4
is_connected ==> chromatic_nb <= max_degree
```

Parentheses can be used to group conditions:
```
(n == 5 or n == 6) and is_connected
```

## Extremal selections

GraphQuest can select graphs according to an extremal value.

The general syntax for `max` is:
* `max(expression)`, and
* `max(expression; grouping_expression_1, grouping_expression_2, ...)`.

Similarly, the general syntax for `min` is:
* `min(expression)`, and
* `min(expression; grouping_expression_1, grouping_expression_2, ...)`.

The function `max` selects the graphs for which the given expression reaches its maximum value, while `min` selects the graphs for which it reaches its minimum value.


For example:
* `max(chromatic_nb)`: selects the graphs that have the maximum value of `chromatic_nb` in the *entire* dataset.
* `max(chromatic_nb;n,m)`: selects the graphs that have the maximum value of `chromatic_nb` for each pair of `(n,m)`.
* `max(chromatic_nb - max_degree; n, m % n)`: selects the graphs that maximize the difference beween `chromatic_nb` and `max_degree` for each
pair `(n, m % n)`.

## Dataset filtering



**Filters** can be used to chain conditions and extremal selections, progressively
reducing the current dataset and thus the number of values that GraphQuest has to compute.

<!-- Each condition (or extremal selection) is evaluated on the dataset produced by the previous
condition (or extremal selection). The result of one expression therefore becomes the input dataset
for the next expression. -->


Let `X`, `Y`, and `Z` be conditions or extremal selections. A filter has the
following syntax:
```text
X -> Y
```
Here, `Y` is evaluated on the dataset resulting from `X`.

Filters can be chained to perform multiple operations sequentially:
```
X -> Y -> Z
```
In this case, `X` is applied first, followed by `Y` on the resulting dataset,
and finally `Z` on the dataset produced by `Y`.


For example, consider the following query:
```
n % 2 == 0 -> m > 5 -> chromatic_nb > 3
```

The query is evaluated from left to right, with each filter being applied to the dataset produced by the previous one.
1. `n % 2 == 0` keeps only graphs whose number of vertices `n` is even.
2. `m > 5` then keeps, among those graphs, only the ones having more than
   5 edges.
3. `chromatic_nb > 3` finally keeps only the graphs whose chromatic number is
   greater than 3.

The final dataset therefore contains graphs satisfying **all three conditions**: an even number of vertices, more than 5 edges, and a chromatic number greater than 3. By applying the filters sequentially, GraphQuest can reduce the number of graphs for which the chromatic number needs to be computed.


Because GraphQuest only computes values based on the current dataset, filtering it can also be useful when dealing with invariants that are only defined for certain classes of graphs.


For example, suppose that the invariant `eci(G)` is the eccentric connectivity index, denoted by \\(\xi^c(G)\\), and is defined only for connected graphs. If we wanted to select the set of connected graphs that maximise the value of `eci(G)` for each order present in the dataset, we could do:
```
is_connected -> max(eci;n)
```

Here, the first expression restricts the dataset to connected graphs. The second expression then selects the graphs with the maximum `eci` for each value of `n` within this filtered dataset.

The order of the filters is therefore important: `max(eci;n)` is evaluated **only** after disconnected graphs have been removed, so `eci` only needs to be computed for graphs for which it is defined.
