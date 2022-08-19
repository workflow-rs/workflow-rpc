use std::sync::Arc;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct Identifier {
    v : Arc<Mutex<u64>>
}

impl Default for Identifier {
    fn default() -> Self {
        Self { v: Arc::new(Mutex::new(0)) }
    }
}

impl Into<u64> for Identifier {
    fn into(self) -> u64 {
        *self.v.lock().unwrap()
    }
}

impl Identifier {
    pub fn next(&self) -> u64 {
        let mut lock = self.v.lock().unwrap();
        let v = *lock;
        *lock += 1;
        v
    }
}
