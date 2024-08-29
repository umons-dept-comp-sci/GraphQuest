#! /usr/bin/python3
"""Compute two topological indices from mathematical chemistry : Randić and Arithmetic-Geometric index.

Reference:
 - https://en.wikipedia.org/wiki/Randić%27s_molecular_connectivity_index
 - https://arxiv.org/abs/2403.05226
"""
import sys
from functools import reduce
import networkx as nx
from math import sqrt

randic = lambda i, j: 1 / sqrt(i * j)
ag = lambda i, j: (i + j) / (2 * sqrt(i * j))


def apply(G, formula):
    return reduce(lambda index, e: index + formula(G.degree(e[0]), G.degree(e[1])), G.edges(), 0)


if __name__ == "__main__":
    for sig in map(str.strip, sys.stdin):
        G = nx.from_graph6_bytes(sig.encode("utf-8"))
        print(sig, apply(G, randic), apply(G, ag))