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
To add new signatures, use the `add` command. 

```bash
gquest add [OPTIONS] <COMMAND>
```

A new options is added, `batch_size`, which lets the user decide the maximum number of data that can be held in the memory of $\texttt{gquest}$ before being sent to the database. The bigger this number gets, the more the memory will be used. But it can also improve the performances of the program depending on the limit of how much data a single INSERT query can carry in your database.

There are 3 possible sub-commands, `geng`, `file` and `pipe`. Each of these lets the user decide on how to import signatures to the dataset.


#### geng

```bash
gquest add geng [OPTIONS] <(order | range) list> [PARAMS]
```

#### file


```bash
gquest add file [OPTIONS] <PATH>
```

#### pipe


```bash
gquest add pipe [OPTIONS]
```



### Clearing the database

## Querrying the database :

### Modules :

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

Configuration files are used to specify multiple modules in one file alongside some other helpful settings.


#### Executable sorter :

Topological sort


### Module Execution :

After executing a module, $\texttt{gquest}$ communicates with it by doing the following :
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

## Database handling