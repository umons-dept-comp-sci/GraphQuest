#! /usr/bin/env python

import sys
import networkx as nx


def double_star(n: int, m: int) -> nx.Graph:
    star1 = nx.star_graph(n)

    star2 = nx.star_graph(m)

    ds = nx.union(star1, star2, rename=("G", "H"))
    ds.add_edge("G0", "H0")

    return ds

if __name__ == "__main__":
    for n, m in map(str.split, map(str.strip, sys.stdin)):
        n = int(n)
        m = int(m)
        
        print(n, m, nx.to_graph6_bytes(double_star(n,m), header=False)[:-1].decode(), flush=True)