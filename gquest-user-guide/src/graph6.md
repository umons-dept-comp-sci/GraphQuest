# Graph signatures

The [graph6](https://users.cecs.anu.edu.au/~bdm/data/formats.html) format, also referred to as `g6`, was created by Brendan McKay and
is used to store simple undirected graphs by using their adjacency matrix, as compact string values only containing printable ASCII character

Given an isomorphic class, we can select one of the value of this set to represent it, which will be referred to as the canonical member of the class. This selected graph is
often the minimal member under some defined ordering, but for this application we will follow [McKay canonical selection](https://www.math.unl.edu/~aradcliffe1/Papers/Canonical.pdf). The canonical graph's g6 format will then be referred to as the signature of this isomorphic class.

> [!note]
> Let two graphs \\(G\\) and \\(H\\) such that \\(G \simeq H\\). Let the signature of \\(G\\) and \\(H\\) be \\(s_G\\) and \\(s_H\\)
> respectively, then \\(s_G = s_H\\).

Finding those signatures also requires a lot of consideration which is why we highly recommend the use of tools provided by the [nauty and traces](https://pallini.di.uniroma1.it/) collection such as the tool called `geng`, to generate simple undirected graphs. This tool allows us to control the number of vertices, edges and even the classes of
the generated graphs among many other things.


> [!important]
> We will refer to a **dataset** as the set of graph signatures, using Nauty’s g6 format, stored in a database and that can be used to compute values.

## Visualising graph signatures

Currently, the GraphQuest project has no graphical interface, meaning that the resulting graph from a query are output as their signatures. 

Therefore, to visualise a graph signature, we recommend the use of [House of Graphs](https://houseofgraphs.org), a website providing access to a database of graphs considered to be "relevant to the study of some graph theoric problem". House of Graphs also provides a  [graph drawing tool](https://houseofgraphs.org/draw_graph) that allows users to visualise a graph by providing its `g6` format. 


> [!tip]
> It is recommended to check whether a graph found using GraphQuest has already been added to the House of Graphs database. 
> 
> This can provide additional information about the graph, such as its properties, references, or its role in a known graph-theoretic problem.

## Unsupported formats

We currently do not support the *sparse6* format as we typically work with small graphs (from order 1 up to 10).


And since GraphQuest only works with undirected graphs, the *digraph6* format is also not supported.

