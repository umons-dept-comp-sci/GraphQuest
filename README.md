# GraphQuest CLI

Here we discuss how to use the command-line interface of GraphQuest, referred to as $\texttt{gquest}$. 

## General usage

```bash
gquest [OPTIONS] [URL] <COMMAND>
```

* The optional options include the verbosity settings and the print help setting.
* The optional URL represents the URL of the database to connect to. By default, $\texttt{gquest}$ uses an **sqlite** database, which can be automatically created when adding signatures.
* We will discuss the command in the following sections.

## Database manipulation :

In order to find counter-examples on a huge number of graphs, $\texttt{gquest}$ uses a database of your choice. Currently, only **Sqlite** is supported but we plan to add new ones in the near future. This database will hold computed values in order to be used later, which includes tables containing : 
* The provided graph signatures. Called the **dataset**.
* Invariants, such as :
  * The order of each graph stored in the dataset.
  * Computed invariants values depending on the provided modules.

The invariant tables always have two columns :
* The primary column contains the signature of a graph.
* The value column contains the computed invariant of this graph.
  * The name of this column will always be the same as the name of the table which is the name of the invariant (in order to simplify future queries).


### Adding graphs
To add new signatures, use the "*add*" command. 

```bash
gquest add [OPTIONS] <SOURCE>
```

A new options is present, *batch_size*, which lets the user decide the maximum number of data that can be stored in the memory of $\texttt{gquest}$ before being sent to the database. The bigger this number gets, the more the memory will be used. But it can also improve the performances of the program depending on the limit of how much data a single INSERT query can carry in your database.

There are 3 possible sub-commands, "*geng*", "*file*" and "*pipe*". Each of these lets the user decide on how to import signatures to the dataset.


#### geng
Using the `geng` tool from the [Nauty \& Traces collection](https://pallini.di.uniroma1.it/), generates and directly stores graphs in the dataset.
```bash
gquest add geng [OPTIONS] <(order | range) list> [PARAMS]
```

* The order of the graphs to generate always needs to be specified. Provide either a list of :
  - orders (like `1,5,7,3`, which will generate graphs of order 1, 3, 5 and 7).
  - or ranges of orders (like `1:5, 8:10`, which will generate graphs of orders going from 1 to 10 with the exception of 6 and 7).
* The optional params fields represent the additional parameters that can be provided to `geng` to control the class of the graph to generate (ex: "c" for connected graphs).

> [!NOTE]
> To use all of `geng` features, such as controlling the number of edges, you can always use the original tool alongside $\texttt{gquest}$ other commands ("*file*" and "*pipe*"). This command is simply a quick shortcut targeted for simple usage. 

> [!WARNING]
> The `geng` program needs to be part of your $PATH to use this feature.

#### file

Imports all signatures contained within a text file.

```bash
gquest add file [OPTIONS] <PATH>
```
* With the path of the file to read the data from.
  * The file must contain a signature per line.

#### pipe

Reads signatures from the stdin until it closes.

```bash
gquest add pipe [OPTIONS]
```

* The value read must only contain one signature per line.


### Removing data

Instead of deleting the database to entirely clear it, $\texttt{gquest}$ lets you either remove specific tables :

```bash
gquest remove [OPTIONS] <TARGET>
```
There are three possibles choices :
* "*invariants*" : Removes all tables other than the dataset (and related vertices table).
* "*dataset*" : Removes the dataset (and related vertices table).
* "*all*" : Removes **all** tables present in the dataset.

> [!WARNING]
> GraphQuest can delete tables that do not have anything to do with invariants. Because of this, it is not recommended to use a database not entirely controlled by it since it could delete or overwrite any table.


## Modules :

We refer to as an *invariant executable*,  or *module*, a file that matches the following conditions :
* An **executable** file
* Reads its `stdin` wile running
* Outputs data to its `stdout`
<!-- 
There are two way of creating an invariant executable, one is to use a dependency file as explained in this [section](#dependency-files-), or to create them one by one. -->

To correctly use one, $\texttt{gquest}$ needs the following informations :
* `exec_path` : The path to the module.
* `names` : The names of the invariants computed and returned by this program.
  * Each name must start with a letter (or \'\_\') while the rest can only contain letters, numbers and \'\_\'. This is in order to store it in a database without any troubles or having to change the name.
  * Based on the number of provided names $\texttt{gquest}$ will expect the same number of arguments to be returned **alongside** the graph signature.
* `dependencies` : The names of the **invariants** that need to be passed as *inputs* to this executable alongside the graph canonical form (see this [section](#executable-sorter-) for more informations).
  <!-- * `^([a-z]|[A-Z]|_)(_|[a-z]|[A-Z]|[0-9])*$` in order to store it in a database without any troubles or having to change the name. -->



#### Configuration files :

Configuration files are used to specify multiple modules in one file alongside some other helpful settings. These files are written using the json format and are made up of three name-value fields :
1. The optional batch size. It is similar to what was explained prior but for queries. Set to 1000 by default.
2. The optional maximum number of threads to use when computing invariants. Set to 1 by default.
3. The list of modules, each is also an object made up of three name-value pairs :
   1. A path to the executable.
   2. A list with the names of the returned invariants.
   3. The optional list with the names of the dependencies to provide to this module.


##### Example config file : `example.json`
```json
{
  "batch_size": 5000,
  "nb_threads": 10,
  "executables": [
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
      "path": "is_Bmn.py",
      "names": [
        "is_Bmn"
      ],
      "dep": [
        "km",
        "rm"
      ]
    }
  ]
}
```

### Module Execution :

After starting the execution of a module, $\texttt{gquest}$ communicates with it by doing the following :
```
1. While data is left to compute :
2.    Write a data batch to the standard input of the progam.
3.    Flush the standard input
4.    While the same number of data was not received :
5.        Read a line the standard output of the program.
6.    Save the content of the received batch to the database.
7. Close the stdin of the program.
```
If during any of the steps, the modules crashes then the error it wrote to its stderr will be encapsulated and shown to the user.


Meanwhile, the module only has to do the following :
```
1. While its stdin is still open :
2.    Read a line from the stdin
3.    Compute its invariant(s)
4.    Write the value(s) to its stdout. And if necessary flush it.
5. Ends the program
```
Step 4 is really important since because the cli cannot access the content of the stdout without it being flushed, meaning there is a risk of **deadlock** if the module doesn't often flush its content.

> [!WARNING]
> Do not forget to remove any function printing debug content to the stdout, such as `print(..)` in Python. Because then this content will be interpreted as a returned value of the module by the CLI.

> [!NOTE]
> The programs always saves all data returned by the modules it called, even for invariants it did not really needed at the time. But it only calls a module if one of the values it computes is needed for a user query.
 

In the following sections, we will go over how the inputs and outputs need to be formated by all parties.

#### Inputs :

Modules should expect a line with the signature and the values of the required dependencies. 
```bash
signature dep_1 ... dep_m
```

#### Outputs :

Depending on the number of invariant computed, the module should write to the stdout a line of the following form :
```bash
signature inv_1 ... inv_n
```

> [!WARNING]
> Do not forget to often flush the stdout, otherwise a *deadlock* might arise because $\texttt{gquest}$ will wait for data that will never be accessible.

> [!TIP]
> Let $n$ the total number of lines of data sent, flush the stdout, after writing $m$ lines (with $m \leq n$).


<!-- While running, if the program needs the value from another (already) computed invariant then instead of writing the previously mentionned format in it's stdout, it can return :
```bash
query inv_name signature_0 ... signature_n
```

For example :
```bash
size D]w ... DUw
``` -->

## Queries :

Now that the concepts of modules and configuration files were introduced, we now have everything we need to query the database.

```bash
gquest query [OPTIONS] <CONFIG_FILE> <QUERY> [OUTPUT]
```
To correctly run a query, we need to provide :

* A config file, which should contain executables so that it is possible to compute every invariants referenced in the provided query.
* The query to execute (we will talk about this later).
* The output either as a "*file*", to the "*stdout*" or in a pretty "*table*".


### Syntax :

GraphQuest uses a simplistic condition syntax in order to allow you to write most simple queries (note that whitespace character are ignored during parsing).

#### Extremal values search :

// Work in progress

```
<extremal selection> [, <Additional condition>] 
```

With `extremal_selection`:
```py
(min|max) '(' inv (':' inv (',' inv)*)? ')'
```


#### Counter-example search :


```py
extremal_selection (',' optional_condition)? '=>' condition_to_disprove
```

For example the following query, `min(inv1: inv2, inv3), inv4 >= 3 => inv5 = 1` :
1. Will compute every value of `inv1`, `inv2`, `inv3` and `inv4` (and if needed any dependencies they have) for all values contained in the dataset.
2. Will find the graphs that have the `min` value of `inv1` for each combination of `inv2` and `inv3` and whose `inv4` value is superior or equal to 3. 
3. Then compute `inv5` only for those extremal graphs
4. And finally, it tries to find a graph for which the value of `inv5` will not be equal to 1. 



### Modules sorting



## Examples :

```
gquest add geng 1:4, 6:8 "c"
```


```bash
gquest query "path_to_config.json" "min(P_Gn: m,n), n>=3 => is_Bmn = 1" table -p 2:
```