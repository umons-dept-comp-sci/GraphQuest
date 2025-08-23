use crate::utils::subject::Subject;

use super::database_error::GraphDatabaseError;



/// The GraphDatabase trait is used to facilitate the communication with databases for the user.
pub trait GraphDatabase<'a> : Subject<'a> + Clone 
{
    /// Creates the database that will be storing the project.
    /// 
    /// Returns a struct implementing the [GraphDatabase] trait.
    /// ## Exceptions
    /// Returns :
    /// * [GraphDatabaseError::DatabaseAlreadyCreated] if the database was already created
    /// * [GraphDatabaseError::UnknownError] if an unknown error was uncountered when trying to create it
    async fn create_graph_database(db_url: &str) -> Result<Self, GraphDatabaseError>;
}