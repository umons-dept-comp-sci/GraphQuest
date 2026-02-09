#! /usr/bin/env python
"""Checks wether a graph is 4 colorable or not"""
import sys
import networkx as nx

def chromatic_number(G) -> int:
    colorations = nx.greedy_color(G)
    if type(colorations) == dict:
        colorations = [colorations]
    min_coloration = 0
    distinct_color = []
    for colors in colorations:
        for color in colors.values():
            if not color in distinct_color :
                distinct_color.append(color)
        if min_coloration == 0 or len(distinct_color) < min_coloration:
            min_coloration = len(distinct_color)
        distinct_color = []
    return min_coloration



if __name__ == "__main__":
    for sig in map(str.strip, sys.stdin):
        G = nx.from_graph6_bytes(sig.encode("utf-8"))
        print(sig, chromatic_number(G), flush=True)