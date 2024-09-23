#! env/bin/python
"""Determine if a graph is planar.

Reference:
 - https://en.wikipedia.org/wiki/Planar_graph
"""
import sys
import networkx as nx



if __name__ == "__main__":
    flush_time = 10 
    for sig in map(str.strip, sys.stdin):
        sig = sig.split()
        sig = sig[0]
        G = nx.from_graph6_bytes(sig.encode("utf-8"))

        sys.stdout.write(sig + " " + str(nx.is_planar(G)) + "\n")
        flush_time -= 1
        if flush_time == 0:
            sys.stdout.flush()
            flush_time = 10