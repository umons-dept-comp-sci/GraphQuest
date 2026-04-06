# GraphQuest CLI

Here we discuss how to use the command-line interface of GraphQuest, referred to as $\texttt{gquest}$. 

## General usage

```bash
gquest [OPTIONS] [URL] <COMMAND>
```

* The options include the verbosity settings and the help setting.
* The URL represents the URL of the database to connect to. By default, $\texttt{gquest}$ uses an **sqlite** database, which will be automatically created in the current repository when adding signatures. If necessary, this URL should contain the necessary authorisations to allow GraphQuest to connect to it and have Read & Write permissions.
* We will discuss the commands in the following sections.

## Database manipulation:

In order to find counter-examples on a huge number of graphs, $\texttt{gquest}$ uses a database of your choice. Currently, only **Sqlite** and **PostgreSQL** are supported but we plan to add new ones in the near future. 

Depending on the queries used and the provided modules, this database will be filled with the values $\texttt{gquest}$ needs to provide an answer.



> [!WARNING]
> Do not use a database containing data unrelated to GraphQuest since it will suppose that every table can be deleted or modify when needed.
<!-- * a one column table called: **dataset**; and
* multiple two columns tables, each created to store the values returned by the given modules. Note that:
  * the primary column contains the signature of a graph, and
  * the value column contains the result of the computation for the graph with the  name of the column be the same as the name of the table which is the name of the invariant (in order to simplify future queries).
 -->

### Adding graphs
To add new signatures to a dataset, or to initialise it, use the "$\texttt{add}$" command. 

```bash
gquest add [OPTIONS] <SOURCE>
```

Besides the already mentionned options, a *batch_size* option is added, which lets the user decide the maximum number of data that can be stored in the memory of $\texttt{gquest}$ before being sent to the database. The bigger this number gets, the more the memory will be used which if parametered correctly can improve the performances of the program depending on the limit of how much data a single INSERT query can carry in the database system being used.

There are 3 possible sub-commands, "*geng*", "*file*" and "*pipe*". Each of these lets the user decide on how to import signatures to the dataset.


#### geng
Using the `geng` tool from the [Nauty \& Traces collection](https://pallini.di.uniroma1.it/), generates and directly stores graphs in the dataset.
```bash
gquest add geng [OPTIONS] <(order | range) list> [COMMAND_NAME]
```

* The order of the graphs to generate always needs to be specified. Provide either a list of:
  - orders (like `1,5,7,3`, which will generate graphs of order 1, 3, 5 and 7).
  - or ranges of orders (like `1:5,8:10`, which will generate graphs of orders going from 1 to 10 with the exception of 6 and 7).
* The $\texttt{ARGS}$ option is present to allow additional parameters that can be provided to `geng` to control the class of the graph to generate (ex: "c" for connected graphs).
* Sometimes the $\texttt{geng}$ program has a different name in the path, such as $\texttt{nauty-geng}$ which will prevent the cli from calling it, so the $\texttt{COMMAND\_NAME}$ field can be used to change the name of the program to execute. This also allows the user to use a diferent graph generation programs from the nauty suit like $\texttt{gentreeg}$ for example.
> [!NOTE]
> To use all of `geng` features, such as controlling the number of edges, you can always use the original tool alongside the "*file*" and "*pipe*" commands. This command is simply a quick shortcut targeted for a simple usage. 

#### file

Imports all signatures contained within a text file.

```bash
gquest add file [OPTIONS] <PATH>
```
The path of the file to read the data from has to be provided.
  * This file is expected to contain one signature per line.

#### pipe

Reads signatures from the stdin until it closes.

```bash
gquest add pipe [OPTIONS]
```

* The value read must only contain one signature per line.


### Removing data

Instead of deleting the database to entirely clear it, $\texttt{gquest}$ lets you either remove specific tables:

```bash
gquest remove [OPTIONS] <TARGET>
```
There are three possibles choices:
* "*all-invariants*": Removes all tables other than the dataset (and related vertices table).
* "*dataset*": Removes the dataset (and related vertices table).
* "*all*": Removes **all** tables present in the dataset.

> [!WARNING]
> GraphQuest can delete tables that do not have anything to do with invariants. Because of this, it is not recommended to use a database not entirely controlled by it since it could delete or overwrite any table.


## Modules:

We refer to as an *invariant executable*,  or *module*, a file that is
* an **executable** file, 
* reads its `stdin` wile running until it closes, and
* outputs data to its `stdout`.
<!-- 
There are two way of creating an invariant executable, one is to use a dependency file as explained in this [section](#dependency-files-), or to create them one by one. -->

To correctly use one, $\texttt{gquest}$ needs the following informations:
* `exec_path`: The path to the module.
* `names`: The names of the invariants computed and returned by this program.
  * Each name must start with a letter (or \'\_\') while the rest can only contain letters, numbers and \'\_\'. This is in order to store it in a database without any troubles or having to change the name.
  * Based on the number of provided names $\texttt{gquest}$ will expect the same number of arguments to be returned **alongside** the graph signature.
* `dependencies`: The names of the **invariants** that need to be passed as *inputs* to this executable alongside graph signatures.
  <!-- * `^([a-z]|[A-Z]|_)(_|[a-z]|[A-Z]|[0-9])*$` in order to store it in a database without any troubles or having to change the name. -->



#### Configuration files:

Configuration files are used to specify multiple modules in one file alongside some other helpful settings. These files are written using the json format:
1. The optional batch size. It is similar to what was explained prior but for queries. Set to 1000 by default.
2. The optional maximum number of threads to use when computing invariants. Set to 1 by default.
3. The optional epsilon which changes how queries handle floating point errors. Set to 0 by default.
4. The list of modules, each is also an object made up of three name-value pairs:
   1. A path to the executable.
   2. A list with the names of the returned invariants.
   3. The optional list with the names of the dependencies to provide to this module.
5. The list of aliases that will be replaced in a queyr by a given value. 
   1. Each element of the list is a pair of strings: [alias, value to replace it by in the query].


##### Example of a configuration file: `example.json`
```json
{
  "batch_size": 5000,
  "nb_threads": 10,
  "epsilon": 0.1,
  "modules": [
    {
      "path": "P_Gn.py",
      "names": [
        "P_Gn"
      ]
    },
    {
      "path": "km_rm.py",
      "names": [
        "km",
        "rm"
      ]
    },
    {
      "path": "rust_module/chromatic_nb",
      "names": [
        "chromatic_nb"
      ]
    },
    {
      "path": "is_Bmn.py",
      "names": [
        "is_Bmn"
      ],
      "dep": [
        "km",
        "rm"
      ]
    }
  ],  
  "aliases": [
    [
      "is_four_colorable",
      "chromatic_nb <= 4"
    ]
  ]
}
```

### Module Execution:

After starting the execution of a module, $\texttt{gquest}$ communicates with it by doing the following:
```
1. While data is left to compute:
2.    Write a data batch to the standard input of the progam.
3.    Flush the standard input
4.    While the same number of data was not received:
5.        Read a line the standard output of the program.
6.    Save the content of the received batch to the database.
7. Close the stdin of the program.
```
If during any of the steps, the modules crashes then the error it wrote to its stderr will be encapsulated and shown to the user.


Meanwhile, the module only has to do the following:
```
1. While its stdin is still open:
2.    Read a line from the stdin
3.    Compute its invariant(s)
4.    Write the value(s) to its stdout. And if necessary flush it.
5. Ends the program
```
Step 4 is really important since because the cli cannot access the content of the stdout without it being flushed, meaning there is a risk of **deadlock** if the module doesn't often flush its content.

> [!WARNING]
> Do not forget to remove any function printing debug content to the stdout, such as `print(..)` in Python. Because then this content will be interpreted as a returned value of the module by the CLI.

> [!NOTE]
> The programs always saves all data returned by the modules it called, even for invariants it did not really need at the time. But it only executes a module if one of the values it computes is needed for a user query.
 

In the following sections, we will go over how the inputs and outputs need to be formated by all parties.

Suppose that the module has $m$ dependencies and return $n$ values per graph signatures.

#### Inputs:

Modules should expect a line with the signature and the values of the required dependencies. 
```bash
signature dependency_1 ... dependency_m
```

#### Outputs:

Depending on the number of invariant computed, the module should write to the stdout a line of the following form:
```bash
signature value_1 ... value_n
```

> [!WARNING]
> Do not forget to often flush the stdout, otherwise a *deadlock* might arise because $\texttt{gquest}$ will wait for data that will never be accessible.

> [!TIP]
> Let $n$ the total number of lines of data sent, flush the stdout, after writing $m$ lines (with $m \leq n$).

## Expressions:

Expressions come in two forms:

| **Operations:** | **Syntax:**      |
| ---------------- | ----------------- |
| Addition         | x + y             |
| Multiplication   | x * y             |
| Subtraction      | x - y             |
| Power            | x ** y `\|` x ^ y |
| Modulo           | x % y             |
| Division         | x / y             |
| Floor division   | x // y            |

| **Unary functions:** | **Syntax:** |
| --------------------- | ------------ |
| Floor                 | floor(x)     |
| Ceil                  | ceil(x)      |
| Absolute              | abs(x)       |
| Square Root           | sqrt(x)      |
| Negation              | -x           |

Where `x` and `y` represent either:
* A value or an identifier
* Another expression.

> [!NOTE]
> Parenthesis are also supported and operations respect the classical operator priority.



## Queries:


GraphQuest uses a simplistic condition syntax in order to allow you to write most simple queries (note that whitespace character are ignored during parsing). All of the following query syntaxes are considered valid and can be executed by $\texttt{gquest}$.

```bash
gquest query [OPTIONS] <CONFIG_FILE> <QUERY> [OUTPUT]
```
To correctly run a query, we need to provide:

* A config file, which should contain executables so that it is possible to compute every invariants referenced in the provided query.
* The query to execute (we will talk about this later).
* The output either as a "*file*", to the "*stdout*" or in a pretty "*table*".


#### Comparisons: 

```ebnf
comparison = expression, comp_operator, expression
           | identifier;

comp_operator = "=" | "!=" | "<" | ">" | "<=" | ">" | ">=";
```
With :

| Operator:           | Equal | Not Equal | Less  | Less or Equal | Greater | Greater or Equal |
| -------------------- |:---: |:-------: |:---: |:-----------: |:-----: |:--------------: |
| **Allowed syntax:** |   =   |    !=     |   <   |      <=       |    >    |        >=        |
|                      |  ==   |     ≠     |       |       ≤       |         |        ≥         |

##### Example:

Let the comparison be: `inv == "a"`.

This can be translated to: Keep graph whose value of `inv` is equal to `"a"`.

#### Conditions:

```ebnf
condition = condition, cond_operator, condition
          | equivalence
          | implication 
          | logical_negation
          | "(", condition, ")"
          | comparison;

equivalence = condition, "<==>", condition;
implication = condition, "==>", condition;
logical_negation = "not", condition;

cond_operator = "and", "or", "xor";
```
<!-- 
| Operator:           | **And** | **Or**| **Xor**  | **Negation**  |  **Implication**  |  **Equivalence**  | 
| --------------------|:-----: |:------:|:------------: |:------------: | :------------:    | :------------:    | 
| **Allowed syntax:** |   and  |   or   |     xor       |     not       |      ==>          |      <==>         | 
|                     |    ∧   |   v    |      ⊕        |      !        |                   |      iff          |  -->



##### Examples:

Let a condition be: `inv_1 != inv2 or !(inv_2 <= 3)`

This can be translated to: The value of invariant `inv_1` must be different from the value of `inv2` or the value of `inv_2` must be strictly greater than 3.

#### Extremal values search:

You can also retrict the research space to only use extremal graphs using the following syntax :

```ebnf
extremal_condition = extremal ,  ["and" , condition] 
                    | condition;

extremal = extremal_func, "(", id, [":", id, {",", id}], ")";

extremal_func = "max" | "min";
```

##### Examples: 

Let the selection be: `max(inv)`

This can be translated to: *find the graphs with the maximum value for the given `inv`*.

Let the selection be: `min(inv_1: inv_2, inv_3) and inv_2 >= 4`

This can be translated to: *find the graphs with the minimum value for a given `inv_1` for each combination of `inv_2` and `inv_3` where the value of `inv_2` is bigger than 4* 
> If you know a bit of SQL think of it as a selection query using the MIN function combined with a GROUP BY clause. 



#### If-then queries:

Another feature is the search of counter-examples for a given **extremal** conjecture, which can be expressed using the following syntax:

```py
if_then_cond = "if", extremal_condition, "then", condition 
              | extremal_condition, "->", condition;
```
Let $\texttt{x}$ be an extremal condition and $\texttt{y}$ be a condition, then the statement "$\texttt{if x then y}$" will select all graphs for which $\texttt{x}$ evaluates to true and evaluates $\texttt{y}$ only for those graphs, and selects only the graphs for which both conditions are true. These queries are useful since they will allow GraphQuest to only compute the values of the identifiers from the left extremal condition for the entire dataset while the values of the identifiers in the right-condition only have to be computed for the graphs which satisfied the first condition. This is similar to the concept of lazy evaluation used in some programming languages like Java.

##### Example: 

Le the following query be `min(inv1: inv2, inv3), inv4 >= 3 -> inv5 = 1`.

GraphQuest will:
1. Compute every value of `inv1`, `inv2`, `inv3` and `inv4` (and if needed any dependencies they have) for all values contained in the dataset.
2. Find the graphs that have the `min` value of `inv1` for each combination of `inv2` and `inv3` and whose `inv4` value is superior or equal to 3. 
3. Compute `inv5` only for those extremal graphs (and its required dependencies if any).
4. Try to find a graph for which the value of `inv5` will not be equal to 1. 

