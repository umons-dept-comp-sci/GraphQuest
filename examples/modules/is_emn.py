#! /usr/bin/env python
"""Compute the eccentricity of a graph"""
import sys
import networkx as nx
from math import comb

from eci import create_E_nm

mem = {}

if __name__ == "__main__":
    for sig, d_nm in map(str.split, map(str.strip, sys.stdin)):
        G = nx.from_graph6_bytes(sig.encode("utf-8"))
        d_nm = int(d_nm)
        
        n, m = G.order(), G.size()
        if (n,m,d_nm) not in mem:
            mem[(n,m, d_nm)] = create_E_nm(n, m, d_nm)
        
        print(sig, int(nx.is_isomorphic(G, mem[(n,m, d_nm)])), flush=True)