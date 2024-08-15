pub trait Observer {
    /// Notify the observer that a tick has passed
    fn notify_tick(&self);
    /// Notify the observer of the quantity of data read since the last call to this function
    fn notify_data_pushed(&self, progression: u64);

    /// Notify that the subject has moved on to the next step
    fn notify_iteration(&self);
    
}

pub trait Subject<'a> {
    fn set_graph_db_observer(&mut self, obs: &'a dyn Observer);
    fn remove_graph_db_observer(&mut self);
    fn notify_observator(&self, progression: u64);
    fn tick_observator(&self);
} 

