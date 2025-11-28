#! /home/axel/GitProject/GraphQuest/gquest_core/examples/use_case/resources/env/bin/python3

import sys
import networkx as nx
from numpy import sqrt

if __name__ == "__main__":
    for sig in map(str.strip, sys.stdin):
        G = nx.from_graph6_bytes(sig.encode("utf-8"))
        
        res = 0
        for (v, u) in G.edges():
            res += 1/ sqrt(nx.degree(G, v) * nx.degree(G, u))
                
        print(sig, res, flush=True)