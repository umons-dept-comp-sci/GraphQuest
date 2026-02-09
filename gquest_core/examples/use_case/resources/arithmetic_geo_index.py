#! /usr/bin/env python

import sys
import networkx as nx
from math import sqrt

# While the stdin is open, read every line written
for sig in map(str.strip, sys.stdin):
    # Decode the received signature to a graph G
    G = nx.from_graph6_bytes(sig.encode("utf-8")) 
    # Compute AG(G)
    ag = 0
    # For every edges in G
    for (v, u) in G.edges: 
        deg_v, deg_u = nx.degree(G, v) , nx.degree(G, u) # Get the degree of v and u
        ag += (deg_v + deg_u) / (2*(sqrt(deg_v * deg_u)))
    # Write the signature alongside AG(G) to the stdout
    print(sig, ag, flush=True) 