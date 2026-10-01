# Refuting a conjecture


## Definitions:

Let \\(n\\) and \\(m\\) be two positive integers, with \\(n-1 \\geq m \\geq \\binom{n}{2}\\), then we define \\(d_{n,m}\\) as the result of:
\\[
    \left\lfloor{\frac{2n+1-\sqrt{17+8(m-n)}}{2}}\right\rfloor.
\\]


Let \\(n\\) and \\(m\\) be two positive integers, with \\(n-1 \\geq m \\geq \\binom{n}{2}\\), then we define \\(E_{n,m}\\) as the graph obtained from a path \\(P_{d_{n,m}+1}=v_0v_1\cdots v_{d_{n,m}}\\) by joining each vertex of \\( K_{n- (d_{n,m}) - 1} \\) to both \\(v_{d_{n,m}}\\) and \\(v_{d_{n,m}-1}\\), and by joining \\(m-n+1-\binom{n-d_{n,m}}{2}\\) vertices of \\(K_{n-(d_{n,m})-1}\\) to \\(v_{d_{n,m}-2}\\)



**Conjecture**\
Let \\(d_{n,m} \geq 3\\), then \\(E_{n,m}\\) is the unique graph with maximal eccentric connectivity index among all connected graphs with \\(n\\) vertices and \\(m\\) edges.


```bash
gquest q "n-1 <= m <= n*(n-1)/2 -> is_connected -> d(n,m) >= 3 -> max(eci;m,n) -> iso(G, e_nm(n,m))" -c
```


```bash
gquest q "n-1 <= m <= n * (n-1)/2 -> is_connected -> d(n,m) >= 3 -> eci(G) == eci(e_nm(n,m)) -> iso(G, e_nm(n,m))" -c
```