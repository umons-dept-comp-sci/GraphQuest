#! /usr/bin/env python
"""Computes the size m, the greatest triangle number k_m less than m and the remainder m - comb(k_m, 2)."""
import sys
import networkx as nx
from math import comb

def greatest_triangle_number_less_than(n):
    k = 0
    while k*(k-1)//2 <= n:
        k += 1
    return k - 1

def remainder_m(m):
    return m - comb(greatest_triangle_number_less_than(m), 2)

max_flush_count = 1

if __name__ == "__main__":
    flush_count = 0
    memory = {}
    for sig in map(str.strip, sys.stdin):
        G = nx.from_graph6_bytes(sig.encode("utf-8"))
        m = G.number_of_edges()
        if m not in memory:
            memory[m] = (greatest_triangle_number_less_than(m), remainder_m(m))
        if flush_count >= max_flush_count-1:
            print(sig, m, *memory[m], flush=True)
            flush_count = 0
        else:
            print(sig, m, *memory[m], flush=False)
            flush_count += 1
        
    print()