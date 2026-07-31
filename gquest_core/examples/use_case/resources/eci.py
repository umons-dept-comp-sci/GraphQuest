#! /usr/bin/env python
"""Compute the eccentricity of a graph"""
import sys
import networkx as nx


def eci(G: nx.Graph):
    eccs = nx.eccentricity(G)
    res = 0
    for vertice in eccs.keys():
        res += (eccs[vertice] *  G.degree(vertice))
    return res

mem = {}

if __name__ == "__main__":
    for (sig,) in map(str.split, map(str.strip, sys.stdin)):
        G = nx.from_graph6_bytes(sig.encode("utf-8"))
        
        print(sig, eci(G), flush=True)