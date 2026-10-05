# Modules

As explained in the [introduction](index.html#modules), modules are executable files so we will refer to the process resulting
from the execution of a module \\(M\\) as \\(P_M\\). And like for any process, \\(P_M\\) has three input and output
communication channels, each with a different purpose:
* a standard input, called the stdin, which is stream of data containing \\(P_M\\) 's input;
* a standard output, called stdout, which is the stream where \\(P_M\\) writes its outputted data; and
* a standard error, called stderr, which is stream where \\(P_M\\) writes its diagnostics or the errors
it faces.


Another process can be at the other end of these channels to communicate with \\(P_M\\), which is what
GraphQuest's process \\(P_{GQ}\\) does as shown by the following figure:
<p align="center">
  <img src="figures/module_com_simple.png" />
</p>


## Communications with GraphQuest

Suppose that the module \\(M\\) as \\(k\\) arguments, with \\(k \geq 0\\).

GraphQuest will input data by writing lines to \\(P_M\\)' stdin, each one formatted like this:
```
argument_0 ... argument_k−1
```
where each value is separated by a whitespace character. Then \\(P_M\\), upon receiving this line, should
write to its standard output a line containing all computed data for this given signature formatted
like this:
```
argument_0 ... argument_k−1 output
```

where once again each value is separated by a whitespace character. Note that if \\(P_{GQ}\\) receives a
different number of outputs than the one expected, if the format is not respected, or if one of the
outputted values is not of the expected type, it will throw an exception and stop its execution.


Additionally, if at any point an error arises from \\(P_M\\)' side, it can send it to \\(P_{GQ}\\) by writing it to the
stderr channel, which will signal GraphQuest to report the error to the user and cease its execution.

> [!warning]
> A module should not use its stderr channel unless it is to report an error to GraphQuest.

### Argument format

Values need to be written in plain text in the stdin and stdout.

For example, if a module requires one integer and one float as arguments, GraphQuest could write the following to the stdin:
```bash
3 12.2
```

Graph arguments are handled differently: they are written using their g6 format. For example, a module requiring a graph as an argument could receive:
```bash
E?Bw
```

This allows GraphQuest modules to receive both standard values and graph arguments through stdin, while using the g6 format as the standard representation for graphs.


GraphQuest also expects the same when reading values outputted by modules.

## Implementing modules

Now that we know how GraphQuest communicates with a module, let us see how to implement
one. For a module to be compatible with GraphQuest, it should follow the subsequent steps:
1. Wait for a line \\(l\\) to be read in the stdin.
2. Get the arguments from \\(l\\).
3. Compute a value using those arguments.
4. Write the arguments alongside the computed value to the stdout as a line and flush often.
5. If the stdin of the process is still open, go back to step 2.
6. Else exit the process.

> [!tip]
> Since GraphQuest keeps the module's process alive until all needed values were computed, 
> a module can use optimisation methods, such as memoization, in order to accelerate the computations of values.

### Flushing the stdout

Step 4 mentions that we need to often flush the stdout, but what does often truly mean ? 

GraphQuest
works by sending a certain number of inputs, referred to as a batch, to the stdin of the program before waiting for the module to output the same number of data. This step is then repeated until all
the necessary data was computed. So if the number of values of a batch is \\(b\\), then the number of
data written by the module before a flush must be less or equal than \\(b\\) and a divisor of \\(b\\), otherwise
there is a risk that both GraphQuest and the module are stuck waiting for each others until the user
manually interrupts one of the processes. 

You can of course flush after every received input, however it might slightly slow down the execution process.

> [!note]
> Most programming language like Rust or Java flush automatically when using print functions, but for
> some like Python it has to be explicitly specified.


## Defining a module

We will see in the next section how we can define this module in a [configuration file](configs.md) in order to use it as a function in queries.


## Example: creating a python module

Let us implement a module that will check whether two graphs \\(G\\) and \\(H\\) are isomorphic or not, i.e. \\(G \stackrel{?}{\simeq} H\\). Suppose that we will name this file `iso.py` and store it in a directory named "*modules*" located in the current directory.

```
.
├── env
│   └── ...
└── modules
    └── iso.py
```


To create this module, we can take advantage of Python's [networkx](https://networkx.org/en/) library which provides useful
methods related to graphs. Such as the [`from_graph6_bytes(S)`](https://networkx.org/documentation/stable/reference/readwrite/generated/networkx.readwrite.graph6.from_graph6_bytes.html) method that can translate a given
signature \\(S\\) into a graph \\(G\\), or the method [`is_isomorphic(G,H)`](https://networkx.org/documentation/stable/reference/algorithms/generated/networkx.algorithms.isomorphism.is_isomorphic.html) that returns True if \\(G \simeq H\\), False otherwise.


```py
#! /usr/bin/env python
"""Checks if two graphs are isomorphic"""
import sys
import networkx as nx

for sig1, sig2 in map(str.split, map(str.strip, sys.stdin)):
    G = nx.from_graph6_bytes(sig1.encode("utf-8"))
    H = nx.from_graph6_bytes(sig2.encode("utf-8"))

    print(sig1, sig2, int(nx.is_isomorphic(G, H)), flush=True)
```

> [!note]
> Remark that python files are not usually executable but can become so. 
> 
> For example, on a Unix system we can do it in two steps by:
> * turning the file executable, using the command "chmod +x module.py" for example;
> and by
> * adding a Shebang Line, which are lines starting with "`#!`" and that are placed as the first line
> of a text file to specify that it is a script and not a binary file. Then the rest of the line specifies
> the path of the program to execute the script with

So since we already added the shebang line in our example, we just have to make the file executable:
```bash
chmod +x modules/iso.py
```

Let us try our module with the following command:
```bash
printf "A~ BG\nCR Ck" | ./modules/iso.py
```
which should output
```
A~ BG 0
CR Ck 1
```

Now we simply need to define this module in a configuration file in order to use it, which we will do in the next section.