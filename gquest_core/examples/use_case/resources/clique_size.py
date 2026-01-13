#! /usr/bin/env python
"""Return the number of vertices of the maximum clique contained inside a given graph"""
import sys
import networkx as nx

if __name__ == "__main__":
    for sig in map(str.strip, sys.stdin):
        G = nx.from_graph6_bytes(sig.encode("utf-8"))

        print(sig, max(len(c) for c in nx.find_cliques(G)), flush=True)