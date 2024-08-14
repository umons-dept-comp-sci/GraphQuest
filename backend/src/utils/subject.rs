pub trait Observer {
    fn notify(&self, has_progressed: bool);
}

pub trait Subject<'a> {
    fn set_graph_db_observer(&mut self, obs: &'a dyn Observer);
    fn remove_graph_db_observer(&mut self);
    fn notify_observator(&self, has_progressed: bool);
} 




pub struct TempGraphObs
{
    pub function_to_notify : Box<dyn Fn(bool) >
}



impl Observer for TempGraphObs {
    fn notify(&self, has_progressed: bool) {
        (self.function_to_notify)(has_progressed)   
    }
}


fn test(has_progressed: bool)
{
    println!("hi");
}





fn m()
{
    let t = TempGraphObs {function_to_notify : Box::new(test)};
    let x = t.function_to_notify.as_ref();
    // call function
    (x)(true);
}