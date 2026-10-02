# Preparation

## Dataset


For this 

```bash
gquest add geng 1:8
```


## Modules

We will use the following functions:
* "`m(G)`" is the size of G;
* "`chromatic_nb(G)`" is the value of \\(\chi(G)\\);
* "`max_degree(G)`" is the value of the maximum degree in G, denoted by \\(\Delta(G)\\);
* "`is_connected(G)`" is equal to 1 if G is connected, 0 otherwise;
* "`is_cycle(G)`" is equal to 1 if G is a cycle, 0 otherwise;
* "`is_complete(G)`" is equal to 1 if G is a complete graph, 0 otherwise;
* "`eci(G)`" is the value of the eccentric connectivity index of a connected graph \\(G\\), denoted by \\(\xi^c(G)\\);
* "`d(n,m)`" is the value of [\\(d(n,m)\\)](conjecture.md);
* "`e_nm(n,m)`" is the value of [\\(E_{n,m}\\)](conjecture.md);
* "`iso(G,H)`" is equal to 1 if \\(G \simeq H\\), 0 otherwise;


The implementation of these modules can be found in the module examples repository.



## Configuration file

Then for this section we will use a configuration file called `configs.json`. 

```json
{
  "batch_size": 6500,
  "modules": [
    {
      "function": "d",
      "path": "modules/d_nm.py",
      "args": [
        {
          "name": "n",
          "class": "numeric"
        },
        {
          "name": "m",
          "class": "numeric"
        }
      ],
      "output": "numeric"
    },
    {
      "function": "e_nm",
      "path": "modules/e_nm.py",
      "args": [
        {
          "name": "n",
          "class": "numeric"
        },
        {
          "name": "m",
          "class": "numeric"
        }
      ],
      "output": "graph"
    },
    {
      "function": "m",
      "path": "modules/edges.py",
      "output": "numeric"
    },
    {
      "function": "n",
      "path": "modules/order.py",
      "args": [
        {
          "name": "sig",
          "class": "graph"
        }
      ],
      "output": "numeric"
    },
    {
      "function": "chromatic_nb",
      "path": "modules/chromatic_nb",
      "output": "numeric"
    },
    {
      "function": "is_connected",
      "path": "modules/is_connected.py",
      "output": "numeric"
    },
    {
      "function": "is_complete",
      "path": "modules/is_complete.py",
      "output": "numeric"
    },
    {
      "function": "is_cycle",
      "path": "modules/is_cycle.py",
      "output": "numeric"
    },
    {
      "function": "max_degree",
      "path": "modules/max_degree.py",
      "output": "numeric"
    },
    {
      "function": "eci",
      "path": "modules/eci.py",
      "output": "numeric"
    },
    {
      "function": "iso",
      "path": "modules/iso.py",
      "args": [
        {
          "name": "g1",
          "class": "graph"
        },
        {
          "name": "g2",
          "class": "graph"
        }
      ],
      "batch_size": 5000,
      "output": "numeric"
    }
  ]
}
```

Suppose that this file is located in the current directory.