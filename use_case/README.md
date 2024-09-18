# GraphQuest - Use Case

This demo highlights how GraphQuest can be used to disprove graph theoretic results. Specifically, we will focus on **Theorem 5** from the paper [*Legal Coloring of Graphs*](https://citeseerx.ist.psu.edu/document?repid=rep1&type=pdf&doi=11dbe486a34f8c154a89306769f3db51b1d4debe) by **Nati Linial**, which characterizes the extremal (lower bound) graphs for the chromatic polynomial. We will show how GraphQuest identified additional extremal graphs not mentioned in the paper.

### Files Overview

The following Python scripts compute various graph invariants:

- **[`m_km_rm.py`](./m_km_rm.py)**: Computes the size $m$, the greatest triangle number $k_m$ less than $m$ and the remainder $m - \binom{k_m}{2}$.
- **[`P_Gn.py`](./P_Gn.py)**: Computes the chromatic polynomial evaluated at the order $n$ of the graph.
- **[`is_Bmn.py`](./is_Bmn.py)**: Checks whether a given graph is extremal based on the conditions described in Linial's paper.

For more information on the dependencies file, see the [`dependencies.json`](./dependencies.json) file and GraphQuest's documentation.

### Setting Up the Database

We set up the graph database and compute the invariants:
```bash
gquest init geng 1:6
gquest compute -d dependencies.json
```

### Querying the database

*THIS SHOULD BE DELETED* - We create a table `allinv` containing all invariants:
```bash
gquest query "CREATE TABLE allinv AS SELECT * FROM ( SELECT signature, vertices FROM Dataset) INNER JOIN m USING (signature) INNER JOIN P_Gn USING (signature) INNER JOIN is_Bmn USING (signature) INNER JOIN km USING (signature) INNER JOIN rm USING (signature);" table
```

We retrieve all extremal graphs:
```bash
gquest query "SELECT allinv.* FROM allinv JOIN ( SELECT vertices, m, MIN(P_Gn) AS min FROM allinv GROUP BY vertices, m ) extremals ON allinv.vertices = extremals.vertices AND allinv.m = extremals.m AND allinv.P_Gn = extremals.min" table
```

We identify graphs that contradict Theorem 5 from Linial's paper (where is_Bmn is false):
```bash
gquest query "SELECT allinv.* FROM allinv JOIN ( SELECT vertices, m, MIN(P_Gn) AS min FROM allinv GROUP BY vertices, m ) extremals ON allinv.vertices = extremals.vertices AND allinv.m = extremals.m AND allinv.P_Gn = extremals.min WHERE allinv.is_Bmn = 0" table
```