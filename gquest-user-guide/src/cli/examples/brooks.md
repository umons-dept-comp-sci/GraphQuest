# Rediscovering Brook's theorem

For this example, we will demonstrate how GraphQuest can be used to explore datasets to try to discover new results, as well as to rediscover existing theorems, in this case we will look at Brooks' theorem.

**Brook's theorem**\
Let \\(G=(V,E)\\) be a connected graph with \\(\Delta\\) being its maximum degree. If \\(G\\) is neither complete nor an odd cycle, then:
\\[\chi(G) \leq \Delta,\\]
else:
\\[\chi(G) = \Delta + 1.\\]


Suppose for now that this theorem was not proven and that we want to check its validity on our current dataset.


Let us try to find the graph which have a \\(\chi(G)\\) equal to \\(\Delta + 1.\\) in our dataset using:
```bash
gquest q "is_connected -> chromatic_nb == max_degree + 1" -vv -f markdown -p 3:3
```
which results in:

| **i** | **sig** | **max_degree** | **chromatic_nb** |
| ----- | ------- | -------------- | ---------------- |
| 0     | @       | 0              | 1                |
| 1     | A_      | 1              | 2                |
| 2     | Bw      | 2              | 3                |
| ...   | ...     | ...            | ...              |
| 7     | FoDPO   | 2              | 3                |
| 8     | F~~~w   | 6              | 7                |
| 9     | G~~~~{  | 7              | 8                |

As you can see, there are 10 graphs fitting these criteria.


```bash
gquest q "n == 7 -> is_connected -> chromatic_nb > max_degree" -vv -c
```

| **i** | **sig** | **max_degree** | **chromatic_nb** |
| ----- | ------- | -------------- | ---------------- |
| 0     | FoDPO   | 2              | 3                |
| 1     | F~~~w   | 6              | 7                |




## Definitions:



```bash
gquest query "is_connected -> not(chromatic_nb <= max_degree)" -a "n;is_complete;is_cycle;n%2" configs.json
```

| **i** | **sig** | **max_degree** | **chromatic_nb** | **n** | **is_complete** | **is_cycle** | **n % 2** |
| ----- | ------- | -------------- | ---------------- | ----- | --------------- | ------------ | --------- |
| 0     | @       | 0              | 1                | 1     | 1               | 0            | 1         |
| 1     | A_      | 1              | 2                | 2     | 1               | 0            | 0         |
| 2     | Bw      | 2              | 3                | 3     | 1               | 1            | 1         |
| 3     | C~      | 3              | 4                | 4     | 1               | 0            | 0         |
| 4     | DqK     | 2              | 3                | 5     | 0               | 1            | 1         |
| 5     | D~{     | 4              | 5                | 5     | 1               | 0            | 1         |
| 6     | E~~w    | 5              | 6                | 6     | 1               | 0            | 0         |
| 7     | FoDPO   | 2              | 3                | 7     | 0               | 1            | 1         |
| 8     | F~~~w   | 6              | 7                | 7     | 1               | 0            | 1         |
| 9     | G~~~~{  | 7              | 8                | 8     | 1               | 0            | 0         |