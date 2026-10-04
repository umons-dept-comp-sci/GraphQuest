# Introduction

The core of GraphQuest is essentially a Rust library, also referred to as a crate, that was designed to be a quick and reliable exploration tool for graph theory researchers. A command-line interface called `gquest` was developed alongside it.

It was made to help rapidly compute any invariants and to answer simple questions researchers can encounter during a proof or even when trying to refute a given conjecture using a counterexample. It also tries to stay as modular and user-friendly as possible, by allowing users to customize a lot of their experience, such as the tools used to compute values, and the database system to use. 

It even has its own simple query language.


Also note that it is not the goal of GraphQuest to replace any of the already existing tools in this
field. Instead, it is more meant to be used either as the first step when developing an idea before
going further using other tools, or to simply reinforce an intuition during a proof, and it can even
to act as a substitute when working on something not replicable in [House of Graphs](https://houseofgraphs.org/) or [PHOEG](https://phoeg.umons.ac.be/phoeg/) for
example. 


## Quick Start examples

If you are simply looking for examples of how to use GraphQuest, you can [click here](cli/examples) to access the examples section.


## Components

GraphQuest makes uses of three main components, an SQL database, a
configuration file and a set of modules.

*  To start, we have an SQL database which can be either provided by the user with its URL or
created automatically. Either way, it will be used to store any computed values and will be
queried when in need of said data or to answer a user query. Currently, only PostgreSQL and
SQLite databases are supported with plans to add MySQL in the future.
Note that GraphQuest needs sufficient permissions to manipulate a given database.

* Then [Modules](modules.md) are executable files that the user provide and that are used to compute any
invariants or conjecture result for a given graph.

* And finally, a [configuration file](configs.md) is a JSON formatted file containing information that GraphQuest
needs to correctly interact with modules like the path leading to them, their return values and
arguments. But it also has additional information that can affect the performances of the
application such as the maximum number of data that can be saved in the database at a time.

## Main features overview

The core and CLI of GraphQuest have multiple features, which will documented in detail in this guide.


### Query language

GraphQuest's main purpose is to find the graphs in a dataset that fit some desired criteria, which
can be done with the use of queries. They are also what the user will be mostly using when working
with this tool. 


For example, the following query:
```
3 <= n <= 7 and chromatic_nb(G) <= 4
```
will find the set of graphs in the dataset with an order between 3 and 7, whose chromatic number is lesser or equal to 4.

This is just a simple example, see the sections [Query Syntax](query_syntax.md) and [Hands-On Examples](cli/examples) for more informations about what you can do.

### Query translation


GraphQuest translates the given query into an equivalent SQL query which, when executed on a sufficiently populated database, returns the graphs satisfying the query.

However, GraphQuest does not start with a database containing every possible property of every graph. Instead, the database initially contains only a set of graphs, referred to as a dataset, alongside the values that have already been computed. GraphQuest computes additional values only when they are required to evaluate a query.

To achieve this, GraphQuest analyses the query and determines which values are necessary to evaluate it. It then schedules the execution of the appropriate modules to compute these values. Once a module has finished computing a batch of values, the results are stored in the database and can be reused by subsequent queries.

This approach avoids computing unnecessary properties and allows the database to progressively grow as new queries require additional information. Consequently, the same computation does not need to be performed again when a previously computed value is required by a later query.


### Dataset manipulation

Similar tools like [House of Graphs](houseofgraphs.org) and [PHOEG](https://phoeg.umons.ac.be/phoeg/) make use of a dataset of graphs to help you find graphs that meet certain criteria. 
However:
* House of Graphs's dataset is limited to a few interresting graphs, while
* PHOEG's is restricted to all graphs with a degree of at most 10.

To remedy this, GraphQuest's users can provide themselves the dataset they want to use either using tools provided by [nauty and Taces](https://pallini.di.uniroma1.it/) like geng or by importing them from a file. This allows researchers to have complete
control over the content and size of the research space.


### Modules

There are a lot of programming languages to choose from to compute invariants or conjecture results
from a graph signature:
* some have dedicated graph libraries like Python with NetworkX,
* others might lead to faster computations like Rust or C.

Ultimately, programming language are often up to the preferences of the user, so restricting the
program to one will only lead to a less modular experience. For these reasons, GraphQuest instead
uses executable files, referred to as modules.


See the section about [Modules](modules.md) for more informations.