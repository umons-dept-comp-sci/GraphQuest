pub trait Observer {
    fn notify(&self, progression: u64);
    //fn notify_data_pushed(&self, progression: u64);
}

pub trait Subject<'a> {
    fn set_graph_db_observer(&mut self, obs: &'a dyn Observer);
    fn remove_graph_db_observer(&mut self);
    fn notify_observator(&self, progression: u64);
} 

