#! /usr/bin/env python
"""Checks wether a graph is 4 colorable or not"""
import sys
import networkx as nx

def is_k_colorable(G, k:int):
    colorations = nx.greedy_color(G)
    if type(colorations) == dict:
        colorations = [colorations]
    distinct_color = []
    for colors in colorations:
        for color in colors.values():
            if not color in distinct_color :
                distinct_color.append(color)
        if len(distinct_color) <= k:
            return True
        distinct_color = []
    return False



if __name__ == "__main__":
    for sig in map(str.strip, sys.stdin):
        G = nx.from_graph6_bytes(sig.encode("utf-8"))
        print(sig, int(is_k_colorable(G, 4)), flush=True)