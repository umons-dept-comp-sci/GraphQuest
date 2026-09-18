#! /usr/bin/env python
"""Compute the eccentricity of a graph"""
import sys
import networkx as nx


if __name__ == "__main__":
    for sig in map(str.strip, sys.stdin):
        G = nx.from_graph6_bytes(sig.encode("utf-8"))
        print(sig, nx.to_graph6_bytes(nx.complement(G), header=False)[:-1].decode(), flush=True)