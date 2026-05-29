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
            # And cannot share a common end
            has_kn = len(vertices) == 2 and not G.has_edge(vertices[0], vertices[1]) and len(nx.common_neighbors(G, vertices[0], vertices[1])) == 0

            # Their combined degrees need to sum to n-2:
            has_kn = has_kn and (G.degree(vertices[0]) + G.degree(vertices[1]) == n-2)
        
        print(sig, int(has_kn), flush=True)