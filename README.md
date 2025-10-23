# Gquest-core

## Data Loading :

## Invariants Executable :

We refer to as an *invariant executable* a file that matches the following conditions :
* Is an **executable** file
* Reads its `stdin` wile running
* Outputs data to its `stdout`

There are two way of creating an invariant executable, one is to use a dependency file as explained in this [section](#dependency-files-), or to create them one by one.

$\texttt{gquest}$ needs the following informations in order to correctly execute the file :
* `exec_path` : The path to the file to execute.
* `names` : The names of the invariants computed and returned by this program.
  * Each name must respect the following **regex** : `^([a-z]|[A-Z]|_)(_|[a-z]|[A-Z]|[0-9])*$` in order to store it in a database without any troubles or having to change the name.
  * Based on this $\texttt{gquest}$ will expect the same number of arguments to be returned alongside the graph canonical form.
* `dependencies` : The names of the **invariants** that need to be passed as *inputs* to this executable alongside the graph canonical form.
  * Do not worry about wheter or not they have already been added yet, this will checked at a later stage of the execution (see this [section](#executable-sorter-)).



### Dependency files :

These are files used to specify needed values for 


### Executable sorter :

Topological sort

### Inputs :



#### Batches :

At the end of a batch the following keyword will be sent :

```bash
flush
```

This means that the program should flush its output when receiving this value in order to **not** deadlock the main program.

#### End :
When all the data was sent by $\texttt{gquest}$, it will close the *stdin* linking it to the child program.


### Outputs :

<!-- 
> [!WARNING]
> Do not forget to often flush the stdout, otherwise a *deadlock* might arise because $\texttt{gquest}$ will wait for data that will never be accessible. -->

> [!TIP]
> Let $n$ the total number of lines of data sent, flush the stdout, after writing $m$ lines (with $m \leq n$).


While running, if the program needs the value from another (already) computed invariant then instead of writing the previously mentionned format in it's stdout, it can return :
```bash
query inv_name signature_0 ... signature_n
```

For example :
```bash
size D]w ... DUw
```

## Database handling