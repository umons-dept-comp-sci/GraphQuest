#! env/bin/python
"""Compute the size of a graph, i.e., the number of edges."""
import sys
import networkx as nx

if __name__ == "__main__":
    flush_time = 10
    for sig in map(str.strip, sys.stdin):
        G = nx.from_graph6_bytes(sig.encode("utf-8"))
        sys.stdout.write(sig + " " + str(G.number_of_edges()) + "\n")
        flush_time -= 1
        if flush_time == 0:
            sys.stdout.flush()
            flush_time = 10

        