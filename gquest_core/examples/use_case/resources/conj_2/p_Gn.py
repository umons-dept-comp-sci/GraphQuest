#! /home/axel/GitProject/GraphQuest/gquest_core/examples/use_case/resources/env/bin/python3
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


max_flush_count = 1

if __name__ == "__main__":
    flush_count = 0
    for sig in map(str.strip, sys.stdin):
        G = nx.from_graph6_bytes(sig.encode("utf-8"))

        if flush_count >= max_flush_count-1:
            print(sig, P(G), flush=True)
            flush_count = 0
        else:
            print(sig, P(G), flush=False)
            flush_count += 1