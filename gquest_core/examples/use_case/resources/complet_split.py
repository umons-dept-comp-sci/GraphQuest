#! /usr/bin/env python
"""Compute a complete split for a graph"""
import sys
import networkx as nx


def complete_split(n:int, k:int):
    res: nx.Graph = nx.complete_graph(k)

    for i in range(k, n):
        for node in range(k):
            res.add_edge(i, node)

    return res

memory = {}


# if __name__ == "__main__":
#     for sig, ag in map(str.split, map(str.strip, sys.stdin)):
#         G = nx.from_graph6_bytes(sig.encode("utf-8"))
#         ag = float(ag)

                
        # print(sig, int(ag <= (2* r**2) - r), flush=True)


# print(nx.to_graph6_bytes(complete_split(8, 4)).decode("utf-8") )
