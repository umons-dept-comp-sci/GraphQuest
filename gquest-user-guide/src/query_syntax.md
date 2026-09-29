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

The dataset argument, denoted by `G`, is 


## Functions & identifiers



### Functions

A function takes one or more argument and returns a value. Each function is linking to a [module](modules.md) and is defined in a given [configuration file](configs.md).

For example:
```
m(G),
d(n,m)
d(2,4)
iso(G, comp(G))
```

### Invariant
A function that takes only one argument of type Graph can be written without explicitly passing the dataset argument `G`.

Invariants refer to functions that take a graph as their only argument. For example:
* `m(G)` is equivalent to `m`, and
* `chromatic_nb(G)` is equivalent to `chromatic_nb`.

This shorthand allows graph invariants to be used directly in expressions and conditions without explicitly specifying `G`.



## Expressions

| **Operations:** | **Syntax:**       |
| --------------- | ----------------- |
| Addition        | x + y             |
| Multiplication  | x * y             |
| Subtraction     | x - y             |
| Power           | x ** y `\|` x ^ y |
| Modulo          | x % y             |
| Division        | x / y             |
| Floor division  | x // y            |




| **Unary functions:** | **Syntax:** |
| -------------------- | ----------- |
| Floor                | floor(x)    |
| Ceil                 | ceil(x)     |
| Absolute             | abs(x)      |
| Square Root          | sqrt(x)     |
| Negation             | -x          |



## Conditions & comparisons


| Operator:        | **Syntax:** |
| ---------------- | ----------- |
| Equal            | ==          |
| Not Equal        | !=          |
| Less             | <           |
| Less or Equal    | <=          |
| Greater          | >           |
| Greater or Equal | >=          |

| Operator:       | **Syntax:** |
| --------------- | ----------- |
| **And**         | and         |
| **Or**          | or          |
| **Xor**         | xor         |
| **Negation**    | not         |
| **Implication** | ==>         |
| **Equivalence** | <==>        |




## Extremal selections

## If-Then statements