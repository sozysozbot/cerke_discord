use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub enum Status {
    StandingBy,
}


use render_cerke_board::*;

lazy_static! {
    pub static ref STATUS: Arc<Mutex<Status>> = Arc::new(Mutex::new(Status::StandingBy));
    pub static ref FIELD: Arc<Mutex<Field>> = Arc::new(Mutex::new(Field::new()));
    pub static ref LOG: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
}