use sqlx::{migrate::MigrateDatabase, Database};
use thiserror::Error;

use crate::{
    data_handler::invariant_execs::{ExecutableSorter, InvariantError, InvariantsExecutable},
    database_handler::{DbQuerySystem, GraphDatabase, GraphDbRuntimeError},
    utils::{config_file::ConfigFile, table_handler::QueryTable},
};

#[derive(Debug, Error)]
pub enum WorkplaceError {
    #[error("Encountered an error from the database : \"{0}\"")]
    GraphDatabaseError(#[from] GraphDbRuntimeError),
    #[error("Encountered an error from an invariant executable : \"{0}\"")]
    InvariantError(#[from] InvariantError),
}

pub struct Workplace<DB: Database + DbQuerySystem<DB>> {
    _name: Option<String>,
    db: GraphDatabase<DB>,
    execs: Vec<InvariantsExecutable>,
    _nb_threads: usize,
}

impl<DB: Database + DbQuerySystem<DB>> Workplace<DB>
where
    DB: MigrateDatabase,
{
    pub async fn init_workplace(_config: Option<ConfigFile>) -> Result<Self, WorkplaceError> {
        todo!()
    }

    /// Connects to the workspace using the given url
    pub async fn connect_workplace(
        _db_url: &str,
        _config: Option<ConfigFile>,
    ) -> Result<Self, WorkplaceError> {
        todo!()
    }

    pub async fn init_connect_workplace(
        _db_url: &str,
        _config: Option<ConfigFile>,
    ) -> Result<Self, WorkplaceError> {
        todo!()
    }

    /// Adds multiple executable to the workplace
    pub async fn add_executables(&mut self, mut executables: Vec<InvariantsExecutable>) {
        self.execs.append(&mut executables);
    }

    /// Properly closes the worspace
    pub async fn close_workspace(self) {
        self.db.close_connection().await;
    }

    pub async fn execute_all_executables(&mut self) -> Result<(), WorkplaceError> {
        // Sort all executables
        let sorter: ExecutableSorter = self.execs.clone().try_into()?;
        // Group them
        let man = sorter.group_execs()?;

        // Execute them by groups
        for _group in man.get_groups() {}
        Ok(())
    }

    /// Gets a table that will summarize this workplace invariant computation progress.
    ///
    /// For example :
    /// ```b
    ///╭───┬────────────┬────────┬─────╮
    ///│ i │ Table Name │ Size   │ %   │
    ///├───┼────────────┼────────┼─────┤
    ///│ 0 │ Dataset    │ 288266 │ 100 │
    ///│ 1 │ size       │ 13598  │ 5   │
    ///│ 2 │ is_planar  │ 13598  │ 5   │
    ///│ 3 │ num_col    │ 3000   │ 1   │
    ///╰───┴────────────┴────────┴─────╯
    /// ```
    pub async fn summary(&self) -> QueryTable {
        todo!()
    }
}
