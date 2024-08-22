
/// Trait used to represent the observer in the observer design pattern
pub trait Observer: Send + Sync
{
    /// Notify the observer that a tick has passed
    fn notify_tick(&self);
    /// Notify the observer of the quantity of data read since the last call to this function
    fn notify_data_pushed(&self, progression: u64);

    /// Notify that the subject has moved on to the next step
    fn notify_iteration(&self);
    
}


/// Trait used to represent the subject in the observer design pattern
pub trait Subject<'a> {
    /// Set the current graph observer 
    fn set_graph_db_observer(&mut self, obs: &'a dyn Observer);
    /// Remove the current graph observer
    fn remove_graph_db_observer(&mut self);
    /// Updates the observator of the current progress made
    fn update_observator(&self, progression: u64);
    /// Send a tick to the current graph observator
    fn tick_observator(&self);
} 

