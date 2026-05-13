#! /usr/bin/env python
"""Compute the eccentricity of a graph"""
import sys
import networkx as nx
from math import comb


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


def eci(G: nx.Graph):
    eccs = nx.eccentricity(G)
    res = 0
    for vertice in eccs.keys():
        res += (eccs[vertice] *  G.degree(vertice))
    return res

mem = {}

if __name__ == "__main__":
    for sig, d_nm in map(str.split, map(str.strip, sys.stdin)):
        G = nx.from_graph6_bytes(sig.encode("utf-8"))
        d_nm = int(d_nm)
        
        n, m = G.order(), G.size()
        if (n,m,d_nm) not in mem:
            mem[(n,m, d_nm)] = create_E_nm(n, m, d_nm)
        
        print(sig, eci(G), eci(mem[(n,m, d_nm)]), int(nx.is_isomorphic(G, mem[(n,m, d_nm)])), flush=True)