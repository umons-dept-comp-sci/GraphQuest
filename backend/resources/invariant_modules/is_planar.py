#! /usr/bin/python3
"""Determine if a graph is planar.

Reference:
 - https://en.wikipedia.org/wiki/Planar_graph
"""
import sys
import networkx as nx


if __name__ == "__main__":
    for sig in map(str.strip, sys.stdin):
        G = nx.from_graph6_bytes(sig.encode("utf-8"))
        print(sig, nx.is_planar(G), sep=",")