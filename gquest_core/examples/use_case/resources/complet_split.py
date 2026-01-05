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



print(nx.to_graph6_bytes(complete_split(8, 4)).decode("utf-8") )
