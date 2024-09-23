use gquest_core::db_handler::workplace::Workspace;
use gquest_core::db_handler::{graph_database::*, sqlite_handler::*};
use gquest_core::utils::table_handler::QueryTableOptions;

use crate::{cli_commands::*, try_connect_workspace};


pub async fn query(output_args: OutputQueryArgs, formula : String, path : DatabasePath)
{
    let wp: Workspace<SqliteGraphDatabase> = try_connect_workspace(path).await;

    let output_options: StdoutOptions = {
        if let Some(out_ch) = output_args.choice 
        {
            let x: StdoutOptions = match out_ch 
            {
                OutputChoice::Stream => StdoutOptions::Stdout(output_args.separator),
                OutputChoice::Table { full: full_table, partial } => 
                {
                    
                    if full_table {
                        StdoutOptions::PrettyTable(QueryTableOptions::Full)
                    }
                    else {
                        if let Some(partial_val) = partial {
                            let args: Vec<&str> = partial_val.split(":").collect();
                            if args.len() != 2 {
                                panic!("The given arguments for the partial table are not correct : {:?}", args);
                            }
                            let error_msg = |value: &str|{
                    
                                format!("Could not turn {} to an unsigned integer", value)
                            };
                            let (first_rows_count, last_rows_count) = (args[0].parse::<usize>().expect(error_msg(args[0]).as_str()), 
                                                                                     args[1].parse::<usize>().expect(error_msg(args[1]).as_str()));
        
                            StdoutOptions::PrettyTable(QueryTableOptions::Partial { first_rows_count, last_rows_count })
                        }
                        else {
                            StdoutOptions::PrettyTable(QueryTableOptions::Full)
                        }
                    }
                }
            };
            x

        }else {
            StdoutOptions::Stdout(output_args.separator)
        }
    };
    // Executes the query with the given args
    wp.execute_query( &formula, Some(output_args.separator), output_args.file, output_options, false).await;

    wp.close_workspace().await;
}
