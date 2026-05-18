#! /usr/bin/env python
import sys
import networkx as nx


mem = {}

if __name__ == "__main__":
    for sig in map(str.strip, sys.stdin):
        G = nx.from_graph6_bytes(sig.encode("utf-8"))
        n = G.order()

        # Find all maximal cliques
        cliques = list(nx.find_cliques(G))
        biggest_clique = max(cliques, key=len)

        # Check if there is a complete graph K_{n-2} present inside
        has_kn = len(biggest_clique) >= n - 2

        if has_kn:
            # list of vertices not present in the complete graph
            vertices = [item for item in list(range(G.order())) if item not in biggest_clique]
            # These two vertices cannot be adjacent
            has_kn = len(vertices) == 2 and not G.has_edge(vertices[0], vertices[1]) and len(nx.common_neighbors(G, vertices[0], vertices[1])) == 0
            # And cannot share a common end


        # # Checks that the other 
        # print(cliques)
        
        print(sig, int(has_kn), flush=True)


# sig n m is_connected eci_G eci_E is_Enm
# FJ]|w 7 15 1 65 65 0
# GTlzz{ 8 21 1 90 90 0
# GJ\||{ 8 21 1 90 90 0
# ET\w 6 10 1 44 44 0


# "precond and n <= 8 -> d_nm >= 3 and has_complete and eci_G == eci_E and is_Enm"


# sig n m is_connected eci_G eci_E is_Enm
# ITm~vvz}w 10 36 1 152 152 0
# ITm|~z|~W 10 36 1 152 152 0
# ITm||~}~g 10 36 1 152 152 0
# FJ]|w 7 15 1 65 65 0
# GTlzz{ 8 21 1 90 90 0
# GJ\||{ 8 21 1 90 90 0
# ET\w 6 10 1 44 44 0
# HJ\||}~ 9 28 1 119 119 0
# HJ\z|}~ 9 28 1 119 119 0







# sig n m is_connected eci_G eci_E is_Enm
# FJ]|w 7 15 1 65 65 0
# GTlzz{ 8 21 1 90 90 0
# GJ\||{ 8 21 1 90 90 0
# ET\w 6 10 1 44 44 0
# HJ\||}~ 9 28 1 119 119 0
# HJ\z|}~ 9 28 1 119 119 0
# ITm~vvz}w 10 36 1 152 152 0
# ITm|~z|~W 10 36 1 152 152 0
# ITm||~}~g 10 36 1 152 152 0
