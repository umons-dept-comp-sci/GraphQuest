#! /usr/bin/python3
"""Check if a given graph is isomorphic to B_{m,n} where m is the size and n the
order.

The graph B_{m,n} is defined in Legal coloring of graphs by N. Linial.

References :
- https://citeseerx.ist.psu.edu/document?repid=rep1&type=pdf&doi=11dbe486a34f8c154a89306769f3db51b1d4debe
"""
import sys
import networkx as nx
from math import comb

def create_Bnm(n, km, rm):
    G = nx.complete_graph(km)
    G.add_nodes_from(range(km, n))
    for i in range(rm):
        G.add_edge(i, km)
    return G

if __name__ == "__main__":
    memory = {}
    for sig, km, rm in map(str.split, map(str.strip, sys.stdin)):
        km, rm = int(km), int(rm)
        G = nx.from_graph6_bytes(sig.encode("utf-8"))
        n, m = G.number_of_nodes(), G.number_of_edges()
        if (n, m) not in memory:
            memory[(n, m)] = create_Bnm(n, km, rm)
        print(sig, nx.is_isomorphic(G, memory[(n, m)]), flush=True)