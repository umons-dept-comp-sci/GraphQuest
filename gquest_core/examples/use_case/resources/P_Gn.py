#! /usr/bin/python3
"""Compute the chromatic polynomial of a graph evaluated at n.

Reference:
 - https://en.wikipedia.org/wiki/Chromatic_polynomial
"""
import sys
import networkx as nx
from sympy import Symbol

def P(G):
    n = G.number_of_nodes()
    return int(nx.chromatic_polynomial(G).subs({Symbol("x"): n}))


if __name__ == "__main__":
    for sig in map(str.strip, sys.stdin):
        G = nx.from_graph6_bytes(sig.encode("utf-8"))
        print(sig, P(G), flush=True)