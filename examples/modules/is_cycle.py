#! /usr/bin/env python

import sys
import networkx as nx

if __name__ == "__main__":
    for sig in map(str.strip, sys.stdin):
        G = nx.from_graph6_bytes(sig.encode("utf-8"))
        # print(nx.to_graph6_bytes(nx.cycle_graph(G.order())))
        print(sig, int(nx.is_isomorphic(nx.cycle_graph(G.order()), G)), flush=True)
