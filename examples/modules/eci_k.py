#! /usr/bin/env python
"""Compute the eccentricity of a graph"""
import sys
import networkx as nx
from math import comb, floor, sqrt


def create_E_ndk(n, d, k):
    res: nx.Graph = nx.path_graph(d + 1) # path P_{d+1}
    clique_size = n-d-1
    clique = nx.complete_graph(clique_size)
    # Add clique to graph
    res = nx.disjoint_union(res, clique)


    # Join each vertex of u_0 to u_1
    for i in range(d+1, d+1 + clique_size):
        res.add_edge(i, 0)
        res.add_edge(i, 1)

    # Join k vertices to u_2
    for i in range(d+1, d+1 + k):
        res.add_edge(i, 2)
    return res


def eci(G: nx.Graph):
    eccs = nx.eccentricity(G)
    res = 0
    for vertice in eccs.keys():
        res += (eccs[vertice] *  G.degree(vertice))
    return res

mem = {}

if __name__ == "__main__":
    for sig in map(str.strip, sys.stdin):
        G = nx.from_graph6_bytes(sig.encode("utf-8"))
        n, m = G.order(), G.size()
        if (n,m) not in mem:
            D = floor((2* n + 1 - sqrt(17 + 8 * (m - n))) /2 )
            K = m - comb(n - D + 1, 2) - D + 1
            # print(D,K)

            mem[(n,m)] = create_E_ndk(n, D, K)
        # print(nx.to_graph6_bytes(mem[(n,m)]))
        print(sig, eci(G), eci(mem[(n,m)]), int(nx.is_isomorphic(G, mem[(n,m)])), flush=True)