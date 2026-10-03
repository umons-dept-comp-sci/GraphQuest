# Manipulating the dataset

The `gquest add` and `gquest remove` commands can be used to manage the graphs stored in the database.

### Adding graphs

Graphs can be added to the current dataset using:
```bash
gquest add [OPTIONS] <SOURCE>
```

The shortcut to the *add* command is `a`.

GraphQuest supports three different sources:

| Source | Shortcut | Description                                                                                                    |
| ------ | -------- | -------------------------------------------------------------------------------------------------------------- |
| `geng` | `g`      | Generates graph signatures using the `geng` command from the [nauty](https://pallini.di.uniroma1.it/) package. |
| `file` | `f`      | Imports graph signatures from a file.                                                                          |
| `pipe` | `p`      | Imports graph signatures from a pipe.                                                                          |


> [!important]
> If no database exists at the given database URL when using the `add` command,
> `gquest` will try to create one at this URL automatically.
>
> For example, suppose that we are using `gquest` with one of the SQLite features, and the database `test.db` does not exist in the current directory then:
> ```bash
> gquest "sqlite://test.db" add geng 1  
> ```
> will create the database `test.db` before adding the given dataset.

#### Using geng


```bash
gquest add geng [OPTIONS] <(order | range) list> [COMMAND_NAME] <(order | range) list>
```


> [!Important]
> In order to use the `geng` source, the `geng` executable must be present in the path. If not, see the [COMMAND_NAME] argument.



The required argument argument `<(order | range) list>` specifies which graph orders should be generated.

It can contain either a list of:
* individual orders, such as `"5, 7"`, or a list of
* ranges of orders, such as `"1:10"`.

For example:

```bash
gquest add geng 5
```

generates graphs of order 5, while:

```bash
gquest add geng 3:6
```

generates graphs of orders 3, 4, 5, and 6.


The optional `COMMAND_NAME` argument allows you to specify the geng command that should be used to generate the graphs.

For example, some users might use the command `nauty-geng` instead of `geng`:
```bash
gquest add geng 5 nauty-geng
```

If it is omitted, GraphQuest uses its default `geng` command.

<!-- 
The `-b` option can be used to control the batch size used when importing graphs. Its default value is *5000*.

For example, the following command adds all graphs from order 1 to 10 by storing a maximum of 10000 graphs each call to the database.
```bash
gquest add -b 10000 geng "1:10"
``` 

This can improve the performances depending on your machine. 
-->

#### Using files

```bash
gquest add file [OPTIONS] <PATH>
```

GraphQuest 


For example:
```
B?
BO
BW
Bw
```

> [!tip]
> You can use the `geng` command to create files with certain graph classes. 
> 
> For example, using bash, we can store all connected graphs with an order of 3 and 4 in a file:
> ```bash
> geng 3 -c > dataset.txt
> geng 4 -c >> dataset.txt
> ```
> And then add this dataset in GraphQuest:
> ```bash
> gquest a f dataset.txt
> ```


#### Using pipes


> [!tip]
> You can pipe the output of the `geng` command to `gquest`. 
> 
> For example, using bash, we can add all connected graphs with an order of 8 to the dataset:
> ```bash
> geng 8 -c | gquest a p
> ```



### Removing data

The `gquest remove` command can be used to remove data from the database:

```text
gquest remove [OPTIONS] <TARGET>
```

The shortcut to the *remove* command is `r`.



GraphQuest provides several targets, depending on what should be removed:

| Target          | Shortcut | Description                                                                     |
| --------------- | -------- | ------------------------------------------------------------------------------- |
| `invariant`     | `i`      | Removes a specific invariant from the database.                                 |
| `all-invariant` | `ai`     | Removes all invariant tables while keeping the dataset.                         |
| `dataset`       | `d`      | Removes the dataset from the database.                                          |
| `all`           | `a`      | Removes all tables from the database, including tables unrelated to GraphQuest. |

For example, to remove the current dataset:

```text
gquest remove dataset
```

If only the computed invariant data should be removed while keeping the graphs themselves, `all-invariant` can be used:

```text
gquest remove all-invariant
```

> [!warning]
> The `all` target should be used with care, as it removes **every** table in the database, including tables that were not created by GraphQuest.
