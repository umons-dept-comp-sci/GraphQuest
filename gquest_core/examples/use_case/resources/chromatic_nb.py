#! /usr/bin/env python
"""Checks wether a graph is 4 colorable or not"""
import sys
import networkx as nx
import constraint as cs

def is_k_colorable(G, k) -> bool:
    return _get_col_problem(G, k).getSolution() is not None

def _get_col_problem(G: nx.Graph, k: int) -> cs.Problem:
    pb = cs.Problem()
    # Create name of the variables: index of node to string
    variables = []
    for i in range(G.order()):
        variables.append(str(i))
    
    pb.addVariables(variables, list(range(k)))

    # Add contraints
    for i in range(G.order()):
        for j in G.neighbors(i):
            # If i and j are neighbors then i and j cannot be the same
            #print(i, " and ", j, " cannot have the same color")
            pb.addConstraint(lambda col1, col2 : col1 != col2, [str(i), str(j)])
    
    # The graph has to be colored using k distinct colors !
    pb.addConstraint(lambda *x : len(set(x)) == k, variables)

    return pb


def chromatic_number(G: nx.Graph) -> int :
    if G.number_of_edges() == 0:
        return 1
    return _dict_search(G, 0, G.order(), None)

def _dict_search(G: nx.Graph, s: int, f: int, prev):
    size_left = abs(f-s) + 1
    
    current_nb_colors = s + (size_left // 2)
    if prev == None:
        prev = current_nb_colors
    if size_left == 1:
        return current_nb_colors if is_k_colorable(G, current_nb_colors) else prev
    if current_nb_colors == 0:
        return prev
    
    if is_k_colorable(G,current_nb_colors):
        return _dict_search(G, s, current_nb_colors-1, current_nb_colors)
    else:
        return _dict_search(G, current_nb_colors+1, f, prev)




if __name__ == "__main__":
    for sig in map(str.strip, sys.stdin):
        G = nx.from_graph6_bytes(sig.encode("utf-8"))
        print(sig, chromatic_number(G), flush=True)