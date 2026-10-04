# Refuting a conjecture

In this section, we will examine a conjecture which can found in the paper called "On the maximal eccentric connectivity indices of graphs"[^fn1] published in 2014 and written by Zhang Jian-bin, Liu Zhong-zhu and Zhou Bo.

## Definitions:

Let \\(n\\) and \\(m\\) be two positive integers, with \\(n-1 \\leq m \\leq \\binom{n}{2}\\), then we define \\(d_{n,m}\\) as the result of:
\\[
    \left\lfloor{\frac{2n+1-\sqrt{17+8(m-n)}}{2}}\right\rfloor.
\\]


Let \\(n\\) and \\(m\\) be two positive integers, with \\(n-1 \\leq m \\leq \\binom{n}{2}\\), then we define \\(E_{n,m}\\) as the graph obtained from a path \\(P_{d_{n,m}+1}=v_0v_1\cdots v_{d_{n,m}}\\) by joining each vertex of \\( K_{n- (d_{n,m}) - 1} \\) to both \\(v_{d_{n,m}}\\) and \\(v_{d_{n,m}-1}\\), and by joining \\(m-n+1-\binom{n-d_{n,m}}{2}\\) vertices of \\(K_{n-(d_{n,m})-1}\\) to \\(v_{d_{n,m}-2}\\)


The authors then leave open the following conjecture:

**Conjecture**\
Let \\(d_{n,m} \geq 3\\), then \\(E_{n,m}\\) is the unique graph with maximal eccentric connectivity index among all connected graphs with \\(n\\) vertices and \\(m\\) edges.


## Finding a counterexample:
To find a counterexample to this conjecture, we need to find a graph \\(G\\) with order \\(n\\) and size \\(m\\) such that:
* \\(n-1 \\leq m \\leq \\binom{n}{2}\\);
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

This means that we found four counterexamples !

Let us visualise them, using House of Graphs:
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


> [!tip]
> For the previous query, we used a module to compute the value of \\(d_{n,m}\\) in order to keep the query legible. 
> 
> But we could have also computed this value using the following mathematical expression:
> `floor((2*n+1-sqrt(17+8*(m-n)))/2)`.



### Classifying our counterexamples

We will now attempt to classify our examples in order to alter the original conjecture.

#### As complete-but-two graphs

We can classify this set of graphs using the following definition:

**Complete-but-two**\
Let \\(n\\) and \\(\delta\\) be two positive integers, with \\(n \geq 6\\) and \\(\delta \geq 2\\). We define the complete-but-two graph \\(n,\delta\\), denoted by \\(CBT_{n,\delta}\\), as the graph obtained from a complete graph \\(K_{n-2}\\) by joining two vertices \\(a\\) and \\(b\\) such that they are non-adjacent, share no common neighbours:
* \\(d(a) = \delta\\), and
* \\(d(b) = n - \delta - 2\\).

Furthermore, one can prove that the size of a graph \\(CBT_{n,\delta}\\) is:
\\[m = \frac{n^2 - 3n + 2}{2}.\\]

> We can verify that this is the case on our set of counterexamples:
> ```bash
> gquest q "n-1 <= m <= n*(n-1)/2 and is_connected -> d(n,m) >= 3 -> max(eci;m,n) -> not iso(G, e_nm(n,m))" -r -a "m;(n**2-3*n+2)/2"
> ```
> 
> | **i** | **sig**   | **m** | **(((n ** 2) - (3 * n)) + 2) / 2** |
> | ----- | --------- | ----- | ---------------------------------- |
> | 0     | FJ]\|w    | 15    | 15                                 |
> | 1     | ET\\w     | 10    | 10                                 |
> | 2     | GTlzz{    | 21    | 21                                 |
> | 3     | GJ\\\|\|{ | 21    | 21                                 |


Finally, we propose the following theorem.

**Theorem**\
Let \\(CBT_{n,\delta}\\) be a complete-but-two graph, then we have that:
\\[
  \xi^c(CBT_{n,\delta}) = \xi^c\left(E_{n, \frac{n^2 - 3n + 2}{2}}\right) = 2n^2 - 5n + 2,
\\]
with \\(CBT_{n,\delta} \not\simeq E_{n, \frac{n^2 - 3n + 2}{2}}\\)


> Once again, let us verify this theorem on our counterexamples:
> ```bash
> gquest q "n-1 <= m <= n*(n-1)/2 and is_connected -> d(n,m) >= 3 -> max(eci;m,n) -> not iso(G, e_nm(n,m))" -r -a "eci(G); 2*n**2 - 5*n + 2" 
> ```
> | **i** | **sig**   | **eci** | **((2 * (n ** 2)) - (5 * n)) + 2** |
> | ----- | --------- | ------- | ---------------------------------- |
> | 0     | FJ]\|w    | 65      | 65                                 |
> | 1     | ET\\w     | 44      | 44                                 |
> | 2     | GTlzz{    | 90      | 90                                 |
> | 3     | GJ\\\|\|{ | 90      | 90                                 |


Let us exclude this category of graph from our query result and see if we find any other results.

```bash
gquest q "n-1 <= m <= n*(n-1)/2 and is_connected -> d(n,m) >= 3 -> max(eci;m,n) -> iso(G, e_nm(n,m)) or m == (n**2 - 3*n +2)/2" -c
```

| Empty table |
|-------------|
|      /      |

Since we didn't find any other counterexamples, we can modify the original conjecture to include this class of graphs.


**New conjecture**\
Let \\(d_{n,m} \geq 3\\), then \\(E_{n,m}\\) is the unique graph with maximal eccentric connectivity index among all connected graphs with \\(n\\) vertices and \\(m\\) edges except if \\(m= \frac{n^2 - 3n + 2}{2}\\).





#### As complement of double star graphs

```bash
gquest q "n-1 <= m <= n*(n-1)/2 and is_connected -> d(n,m) >= 3 -> max(eci;m,n) -> not iso(G, e_nm(n,m))" -r -a "comp(G); min_degree(G)"
```

| **i** | **sig**   | **comp** | **min_degree** |
| ----- | --------- | -------- | -------------- |
| 0     | ET\\w     | Eia?     | 2              |
| 1     | FJ]\|w    | Fs`A?    | 2              |
| 2     | GJ\\\|\|{ | GsaAA?   | 2              |
| 3     | GTlzz{    | GiQCC?   | 3              |

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


One can prove that a \\(CBT_{n,\delta}\\) graph is the complement of the double star graph \\(S_{\delta,n-\delta - 2}\\).



Let us exclude this category of graph from our query result and see if we find any other results.

```bash
gquest q "n-1 <= m <= n*(n-1)/2 and is_connected -> d(n,m) >= 3 -> max(eci;m,n) -> iso(G, e_nm(n,m)) or iso(comp(G), double_star(min_degree, n-min_degree-2))" -c 
```

| Empty table |
|-------------|
|      /      |


**New conjecture**\
Let \\(d_{n,m} \geq 3\\), then \\(E_{n,m}\\) is the unique graph with maximal eccentric connectivity index among all connected graphs with \\(n\\) vertices and \\(m\\) edges except if \\(G \simeq \overline{S_{\delta,n-\delta - 2}}\\), with \\(\delta\\) the minimum degree of \\(G\\).


### Disproving the new conjectures

We invite the readers to try to disprove the given conjectures with the help GraphQuest.




[^fn1]: Zhang, J. B., Liu, Z. Z., & Zhou, B. (2014). On the maximal eccentric connectivity indices of graphs. Applied Mathematics-A Journal of Chinese Universities, 29(3), 374-378.