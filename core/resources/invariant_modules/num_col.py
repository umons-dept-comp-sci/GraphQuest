#! env/bin/python
"""Compute the number of non-equivalent colorings of a graph.

Reference:
 - https://www.sciencedirect.com/science/article/pii/S0166218X1500476X
"""
import sys
import networkx as nx


def P(G):
    n, m = G.number_of_nodes(), G.number_of_edges()
    if m == n*(n-1)//2:
        return 1
    else:
        e = next(iter(nx.complement(G).edges))
        H = G.copy()
        H.add_edge(*e)
        return P(H) + P(nx.contracted_edge(H, e, self_loops=False))



if __name__ == "__main__":
    flush_time = 10
    for sig in map(str.strip, sys.stdin):
        G = nx.from_graph6_bytes(sig.encode("utf-8"))
        sys.stdout.write(sig + " " + str(nx.is_planar(G)) + "\n")
        
        flush_time -= 1
        if flush_time == 0:
            flush_time = 10
            sys.stdout.flush()
        
    
    