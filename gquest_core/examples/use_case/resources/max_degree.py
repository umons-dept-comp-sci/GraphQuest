#! /usr/bin/env python
"""Computes the size m, the greatest triangle number k_m less than m and the remainder m - comb(k_m, 2)."""
import sys
import networkx as nx


if __name__ == "__main__":
    flush_count = 0
    memory = {}
    for sig in map(str.strip, sys.stdin):
        G = nx.from_graph6_bytes(sig.encode("utf-8"))
        max_deg = 0
        for vertice in range(G.number_of_nodes()):
            max_deg = max(G.degree[vertice], max_deg)

        
        print(sig, max_deg, flush=True)
        
    print()