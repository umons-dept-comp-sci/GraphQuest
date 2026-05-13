#! /usr/bin/env python

import sys
import networkx as nx
from math import floor, sqrt

if __name__ == "__main__":
    for sig in map(str.strip, sys.stdin):
        G = nx.from_graph6_bytes(sig.encode("utf-8"))
        m = G.size()
        n = G.order()
        

        d_nm = floor((2* n + 1 - sqrt(17 + 8 * (m - n))) /2 )
        print(sig, d_nm, flush=True)
