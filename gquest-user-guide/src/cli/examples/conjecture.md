# Refuting a conjecture


## Definitions:

Let \\(n\\) and \\(m\\) be two positive integers, with \\(n-1 \\geq m \\geq \\binom{n}{2}\\), then we define \\(d_{n,m}\\) as the result of:
\\[
    \left\lfloor{\frac{2n+1-\sqrt{17+8(m-n)}}{2}}\right\rfloor.
\\]


Let \\(n\\) and \\(m\\) be two positive integers, with \\(n-1 \\geq m \\geq \\binom{n}{2}\\), then we define \\(E_{n,m}\\) as the graph obtained from a path \\(P_{d_{n,m}+1}=v_0v_1\cdots v_{d_{n,m}}\\) by joining each vertex of \\( K_{n- (d_{n,m}) - 1} \\) to both \\(v_{d_{n,m}}\\) and \\(v_{d_{n,m}-1}\\), and by joining \\(m-n+1-\binom{n-d_{n,m}}{2}\\) vertices of \\(K_{n-(d_{n,m})-1}\\) to \\(v_{d_{n,m}-2}\\)



**Conjecture**\
Let \\(d_{n,m} \geq 3\\), then \\(E_{n,m}\\) is the unique graph with maximal eccentric connectivity index among all connected graphs with \\(n\\) vertices and \\(m\\) edges.


## Finding a counterexample:
To find a counterexample to this conjecture, we need to find a graph \\(G\\) with order \\(n\\) and size \\(m\\) such that:
* \\(n-1 \\geq m \\geq \\binom{n}{2}\\);
* \\(G\\) is connected;
* \\(d_{n,m} \geq 3\\);
* \\(G\\) has the maximum eccentric connectivity index among all connected graphs with \\(n\\) vertices and \\(m\\) edges; and
* \\(G \not\simeq E_{n,m}\\).

In other words, a counterexample is a connected graph \\(G\\) that respects the given preconditions and achieves the same maximum eccentric connectivity index as \\(E_{n,m}\\), while being non-isomorphic to \\(E_{n,m}\\).


We can easily find a graph fitting these criteria using the modules we defined in our  [configuration file](preparation.md#configuration-file) alongside the following query:
```bash
gquest q "n-1 <= m <= n*(n-1)/2 -> is_connected -> d(n,m) >= 3 -> max(eci;m,n) -> not iso(G, e_nm(n,m))" -a "eci(G)"
```
which outputs the following data:
| **i** | **sig**   | **m** | **n** | **e_nm(n,m)** | **iso(G,e_nm(n,m))** | **eci** |
| ----- | --------- | ----- | ----- | ------------- | -------------------- | ------- |
| 0     | ET\\w     | 10    | 6     | Eh\\w         | 0                    | 44      |
| 1     | FJ]\|w    | 15    | 7     | Fh\\zw        | 0                    | 65      |
| 2     | GJ\\\|\|{ | 21    | 8     | Gh\\zz{       | 0                    | 90      |
| 3     | GTlzz{    | 21    | 8     | Gh\\zz{       | 0                    | 90      |

We found four counterexamples !

Let us visualise them:
<table align="center">
  <tr>
    <td align="center">
      <img width="150" height="150" src="figures/counter0.png" />
      <br>
      <em>ET\w</em>
    </td>
    <td align="center">
      <img width="150" height="150" src="figures/counter1.png" />
      <br>
      <em>FJ]|w</em>
    </td>
    <td align="center">
      <img width="150" height="150" src="figures/counter2.png" />
      <br>
      <em>GJ\||{</em>
    </td>
    <td align="center">
      <img width="150" height="150" src="figures/counter3.png" />
      <br>
      <em>GTlzz{</em>
    </td>
  </tr>
</table>

As you can see they seem to be part of the same class of graphs.


Indeed, one can prove that they are complement of double star graphs \\(\overline{S_{\delta,n-\delta - 2}}\\).

```bash
gquest q "n-1 <= m <= n*(n-1)/2 and is_connected -> d(n,m) >= 3 -> max(eci;m,n) -> not iso(G, e_nm(n,m))" -a "comp(G)" -r  
```

| **i** | **sig**   | **comp** |
| ----- | --------- | -------- |
| 0     | ET\\w     | Eia?     |
| 1     | FJ]\|w    | Fs`A?    |
| 2     | GJ\\\|\|{ | GsaAA?   |
| 3     | GTlzz{    | GiQCC?   |


Let us visualise them:
<table align="center">
  <tr>
    <td align="center">
      <img width="150" height="150" src="figures/complement0.png" />
      <br>
      <em>Eia?</em>
      <br>
      <em>\(S_{2,2}\)</em>
    </td>
    <td align="center">
      <img width="150" height="150" src="figures/complement1.png" />
      <br>
      <em>Fs`A?</em>
      <br>
      <em>\(S_{2,3}\)</em>
    </td>
    <td align="center">
      <img width="150" height="150" src="figures/complement2.png" />
      <br>
      <em>GsaAA?</em>
      <br>
      <em>\(S_{2,4}\)</em>
    </td>
    <td align="center">
      <img width="150" height="150" src="figures/complement3.png" />
      <br>
      <em>GiQCC?</em>
      <br>
      <em>\(S_{3,3}\)</em>
    </td>
  </tr>
</table>


Let us exclude this category of graph from our query result and see if we find any other results.

```bash
gquest q "n-1 <= m <= n*(n-1)/2 and is_connected -> d(n,m) >= 3 -> max(eci;m,n) -> iso(G, e_nm(n,m)) or iso(comp(G), double_star(min_degree, n - min_degree-2))" -c 
```

| Empty table |
|-------------|
|      /      |




**New conjecture**\
Let \\(d_{n,m} \geq 3\\), then \\(E_{n,m}\\) is the unique graph with maximal eccentric connectivity index among all connected graphs with \\(n\\) vertices and \\(m\\) edges except if \\(G\\) is the complementary of \\(\overline{S_{\delta,n-\delta - 2}}\\), with \\(\delta\\) the minimum degree of \\(G\\).
