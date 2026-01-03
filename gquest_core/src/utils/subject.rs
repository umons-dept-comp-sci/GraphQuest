/// Trait used to represent the observer in the observer design pattern
pub trait Observer: Send + Sync {
    /// Notify the observer that a tick has passed
    fn notify_tick(&mut self);
    /// Notify the observer of the quantity of things processed since the last call to this function
    fn notify_data_pushed(&mut self, delta: u64);
}
