# GraphQuest CLI

Here we discuss how to execute the command-line interface of GraphQuest, referred to as $\texttt{gquest}$. 



## Prerequisite

In order to use this program a few things have to installed on your machine first. This includes:
* the Rust programming language, alongside Cargo (which is Rust packet manager),  in order to compile the application,
  * you can follow the official tutorial here: https://rust-lang.org/learn/get-started/;
* one of the supported database system by GraphQuest, which currently includes: 
   * SQLite3: https://sqlite.org/download.html which is *recommended* for new users, and
   * PostgreSQL: https://www.postgresql.org/download/; and
* (*optional but highly recommended*) the $\texttt{geng}$ tool from the nauty and traces collection of tools,
  * more information on the official website: https://pallini.di.uniroma1.it/.


## Compiling the project

To build in release mode the project use the following command in the main directory:
```bash
cargo build -r
```
This will create an executable file of the CLI `gquest_cli` at `./target/release`.


We can then move this binary file to the example folder to test it, and we will also rename it to `gquest`:  
```bash
mv target/release/gquest_cli examples/gquest
```

Then we can finally move to the example directory and try GraphQuest:
```bash
cd examples
./gquest -h
```

## Executing modules


### Written in Python

The modules used here expect to be executed from a virtual environment as shown by their shebang line:
```python
#! /usr/bin/env python
```

So it is recommended to create one with the `pip3` package manager and to install the packages listed in the `examples/requirements.txt` file.

Of course, if all the packages listed here are installed globally on your machine, this should not be a problem.


### Written in Rust

There is currently one module written in rust that computes the chromatic number of a graph. In order to use it as a module it has to be compiled into an execute file first.

To do this, we simply have to go to the Rust project of this module, use Cargo to compile it in release mode and move it to the module folder.
```bash
cd examples/modules/chromatic_nb_crate/
cargo build -r
mv target/release/chromatic_nb ../ 
```