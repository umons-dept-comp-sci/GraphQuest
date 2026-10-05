# Configuration files

Configuration files are an indispensable component of GraphQuest, as they allow the users to customise the behaviour of the application as well as letting them define a set of available module.

A configuration file is formatted using the JSON format. This means that each setting is represented as a key-value pair, where the key is the name of the parameter and the value specifies what
to set it to. 

### Fieds
The following paragraphs will go over the currently supported configuration fields.


#### Epsilon

This parameter defines the tolerance used for floating-point comparisons in queries. The precision to use is declared using the optional field "`epsilon`" which is set to 0 by default.

For example:
```json
{
    "epsilon": 0.001
}
```
will consider two floating-point values to be equal if their absolute difference is less than or equal to 0.001.

#### Batch size

The batch size determines the number of values sent to a module during its execution before waiting for the same number of results to be returned and stored in the database. Lowering this value will cause GraphQuest to save the data more frequently, at the cost of performing more frequent database operations.

Adjusting this parameter enables users to balance the memory used and the speed of the compu-
tations. The batch size is declared using the optional field "`batch_size`" which is set to \\(1000\\) by
default.

This value can overriden for a specific module using the [`batch_size` field](#module-batch-size).

For example:
```json
{
    "batch_size": 6500
}
```
will send up to 6500 values to a module at a time before waiting for the module to return the corresponding results.

> [!warning]
> A batch size that is too large can also cause communication issues between GraphQuest and the module. In particular, if the module's stdin or stdout buffers become full, both processes may end up waiting for the other to continue, resulting in a deadlock. Using a smaller batch size can help prevent this situation by limiting the amount of data exchanged between GraphQuest and the module at once.

#### Modules

Each module is described by several fields specifying how it should be executed, what function should be called, and how its input and output should be handled.


##### Executable path

The path to the module. It can be either an absolute or a relative path.
For example:
```bash
{
    "executable": "modules/my_module.py"
}
```

> [!note]
> GraphQuest checks whether or not their exists a file at the given path only when the module has to be executed. 

##### Function name

The name used to refer to this module in queries.

```json
{
    "function": "is_complete" 
}
```

##### Arguments

The `args` field, defines the arguments that GraphQuest should provide to the module's function. 

Arguments can be graph arguments or standard values such as integers, floating-point numbers, or strings.


Currently GraphQuest allows 3 classes of arguments:
* `numeric` represent both integers and floating point values, 
* `string` represent string values, and
* `graph` represent graphs, formatted using their g6 notation.

> [!note]
> Boolean values need to be formatted as numeric values, 
> * 1 for True, 0 for False.

For example:
```json
{
    "args": [
    {
        "name": "n",
        "class": "numeric"
    },
    {
        "name": "m",
        "class": "numeric"
    }
    ],
    "output": "numeric"
}
```


If this field is not specified, GraphQuest will automatically add a argument of type `graph`.

This is useful when defining functions with only a graph as an arguments, which we will later refer to as *invariants*.  


##### Output

Defines the type of value returned by the module. This allows GraphQuest to correctly interpret and store the result in the database.

For example:
```bash
{
    "output": "numeric"
}
```

##### Module batch size

Defines the number of values sent to this module in a single batch. This value overrides the global `batch_size` field when specified.

For example:
```json
{
    "batch_size": 5000
}
```
will send up to 5000 values to the module before waiting for the corresponding results.

This can be useful when dealing with modules that take a significant amount of time to compute a value. After receiving an entire batch, GraphQuest stores the results in the database and will never ask the module to compute those values again.

Therefore, reducing the batch size allows GraphQuest to store results more frequently, reducing the amount of work that would need to be repeated in the event of a crash.


## Example: configuring a python module

Let us continue the example we started in the [module section](modules.md#example-creating-a-python-module), where we created a module `iso.py`, by creating a configuration file to use this module, called `configs.json`.
```
.
├── configs.json
├── env
│   └── ...
└── modules
    └── iso.py
```


The module `iso.py` requires two arguments, two graphs \\(G\\) and \\(H\\) and returns 1 if they are, 0 otherwise. 

So this means that this module:
* requires two arguments of class `graph`, and
* returns a value with a `numeric` class.


And since checking whether or not two graphs are isomorphic can take significant time, we will decrease the value of a batch size for this module.

```json
{
  "modules": [
    {
      "function": "iso",
      "path": "modules/iso.py",
      "args": [
        {
          "name": "G",
          "class": "graph"
        },
        {
          "name": "H",
          "class": "graph"
        }
      ],
      "batch_size": 5000,
      "output": "numeric"
    }
  ]
}
```