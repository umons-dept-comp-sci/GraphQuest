# Installing the CLI

In this section, we will explain how to install `gquest` and add it to your PATH. 

## Features:

* `sqlite`: Uses the bundled SQLite library. No SQLite installation is required.
* `sqlite-unbundled`: Uses the system SQLite library. Requires SQLite to be installed. Can be faster than the bundled SQLite library in some situations. Also reduces the size of the resulting executable.
* `postgres`: Enables PostgreSQL support. Requires a PostgreSQL server.


> [!IMPORTANT]
> At least one of the above feature must be provided in order for the program to compile.

> [!WARNING]
> The features `sqlite` and `sqlite-unbundled` are mutually exclusive. The program will not compile when both are provided.

## Using Cargo


Download the project from its [GitHub repository](https://github.com/umons-dept-comp-sci/GraphQuest).

Then go into the main root of the project and execute the following command:
```bash
cargo install --path gquest_cli -F <FEATURES>
```
The feature can be changed depending on the database backend you want:
```bash
cargo install --path gquest_cli -F sqlite
cargo install --path gquest_cli -F sqlite-unbundled
cargo install --path gquest_cli -F postgres
```

You can even install both `sqlite` (or `sqlite-unbundled`) and `postgres` using:
```bash
cargo install --path gquest_cli -F sqlite -F postgres
```

And to uninstall it:
```bash
cargo uninstall gquest_cli
```

## Trial Run

Try to check the version of `gquest` installed using the following command:
```bash
gquest -V
```

If you can see the version, then the installation was successful.