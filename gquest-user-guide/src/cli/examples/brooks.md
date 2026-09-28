# Rediscovering Brook's theorem

For this example, we will *rediscover* Brooks' theorem using the gquest tool.


**Brook's theorem**\
Let \\(G=(V,E)\\) be a connected graph with \\(\Delta\\) being its maximum degree. If \\(G\\) is neither complete nor an odd cycle, then:
\\[\chi(G) \leq \Delta,\\]
else:
\\[\chi(G) = \Delta + 1.\\]



```bash
gquest query "n == 7 -> is_connected -> not chromatic_nb <= max_degree" configs.json
```


**i**   |**sig**    |**max_degree**   |**chromatic_nb**   |**n**
------- |---------- |---------------- |------------------ |-------
0       |FoDPO      |2                |3                  |7
1       |F\~\~\~w   |6                |7                  |7






```bash
gquest query "is_connected -> not(chromatic_nb <= max_degree)" -a "n;is_complete;is_cycle;n%2"
```