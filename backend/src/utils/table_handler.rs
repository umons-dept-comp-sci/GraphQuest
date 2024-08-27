use tabled::{builder::Builder, settings::Style};


pub enum TableQueryOptions
{
    Full,
    Partial{first:usize, last: usize},
}



pub struct TableQuery
{
    builder: Builder
}


impl TableQuery {
    pub fn new() // -> Self
    {
        let mut builder = Builder::default();
        let v1 = vec![String::from("header 1"), String::from("header 2")];

        let v2 = vec![String::from("column 1"), String::from("column 2")];


        builder.push_record(v1);
        builder.insert_record(0,v2);
        let table = builder.build();
        println!("{}", table);

        //Self{builder}
    }
}