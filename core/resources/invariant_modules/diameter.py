#! env/bin/python
"""Compute the diameter of a connected graph.

Reference:
 - https://en.wikipedia.org/wiki/Distance_(graph_theory)
"""
import sys
import networkx as nx


if __name__ == "__main__":
    for sig in map(str.strip, sys.stdin):
        G = nx.from_graph6_bytes(sig.encode("utf-8"))
        print(sig, nx.diameter(G))