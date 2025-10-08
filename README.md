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

### Inputs/Output :




## Database handling