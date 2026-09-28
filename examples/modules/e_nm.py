#! /usr/bin/env python
"""Compute the eccentricity of a graph"""
import sys
import networkx as nx
from d_nm import d_nm
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


mem = {}

if __name__ == "__main__":
    for n, m in map(str.split, map(str.strip, sys.stdin)):
        n = int(n)
        m = int(m)
        d = d_nm(n,m)
        
        if (n,m) not in mem:
            mem[(n,m)] = create_E_nm(n, m, d)
        
        print(n, m, nx.to_graph6_bytes(mem[(n,m)], header=False)[:-1].decode(), flush=True)