use std::sync::{Arc, Weak, Mutex};

// Arc Mutex
pub type AM<T> = Arc<Mutex<T>>;
// Arc Weak Mutex
pub type AWM<T> = Weak<Mutex<T>>;