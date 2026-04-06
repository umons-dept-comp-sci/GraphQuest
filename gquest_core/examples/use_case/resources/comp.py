#! /usr/bin/env python
"""Compute the eccentricity of a graph"""
import sys
import networkx as nx
from math import comb
from eci import eci


def create_E_nm(n, m, d):
    res: nx.Graph = nx.path_graph(d + 1) # path P_{d+1}
    to_link = m - n + 1 - comb(n-d, 2)
    
    # Add the clique
    for i in range(d+1, n):
        # (node added implicitly by add_edge method)
        for j in range(d, i):
            res.add_edge(i, j)
        res.add_edge(i, d)
        res.add_edge(i, d-1)

        # Add edge to the clique node to v_{d-2} if any
        if to_link > 0:
            res.add_edge(i, d-2)
            to_link -= 1
    return res

if __name__ == "__main__":
    memory = {}


    for sig, d in map(str.split, map(str.strip, sys.stdin)):
        G = nx.from_graph6_bytes(sig.encode("utf-8"))
        d = int(d)
        
        n, m = G.number_of_nodes(), G.number_of_edges()

        eci_g = eci(G)
        if (n, m) not in memory:
            memory[(n, m)] = create_E_nm(n, m, d)

        eci_e = eci(memory[(n,m)])
        
        if d>= 3 and eci_e == eci_g and not nx.is_isomorphic(G, memory[(n, m)]):
            print(sig, 0, flush=True)
        else:
            print(sig, int(eci_g <= eci_e), flush=True)