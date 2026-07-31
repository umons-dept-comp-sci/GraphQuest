#! /usr/bin/env python
"""Compute the eccentricity of a graph"""
import sys
import networkx as nx

if __name__ == "__main__":
    for sig1, sig2 in map(str.split, map(str.strip, sys.stdin)):
        G1 = nx.from_graph6_bytes(sig1.encode("utf-8"))
        G2 = nx.from_graph6_bytes(sig2.encode("utf-8"))

        print(sig1, sig2, int(nx.is_isomorphic(G1, G2)), flush=True)