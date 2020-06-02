use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub enum Status {
    StandingBy,
}



lazy_static! {
    pub static ref STATUS: Arc<Mutex<Status>> = Arc::new(Mutex::new(Status::StandingBy));
    pub static ref LOG: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
}