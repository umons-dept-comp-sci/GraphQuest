#! /usr/bin/env python

import sys
import networkx as nx

if __name__ == "__main__":
    for sig, ag, r in map(str.split, map(str.strip, sys.stdin)):
        G = nx.from_graph6_bytes(sig.encode("utf-8"))
        ag, r = float(ag), float(r)
                
        print(sig, int(ag <= (2* r**2) - r), flush=True)