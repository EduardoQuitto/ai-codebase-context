use std::collections::HashMap;
use crate::model::{Symbol, Visibility};
use serde::Serialize as Ser;

pub const LIMIT: usize = 100;
const PRIVATE: u32 = 1;
static GLOBAL: &str = "fake-only";

pub type Alias = HashMap<String, String>;

pub struct Config {
    pub root: String,
    limit: usize,
}

pub enum Mode {
    Fast,
    Slow,
}

pub trait Store {
    fn get(&self, key: &str) -> Option<String>;
    fn put(&mut self, key: String, value: String);
}

pub struct Memory {
    map: HashMap<String, String>,
}

impl Memory {
    pub fn new() -> Self {
        Memory { map: HashMap::new() }
    }

    fn helper(&self) -> usize {
        0
    }
}

pub mod nested {
    pub fn inner() -> u32 {
        7
    }
}

pub fn top() -> u32 {
    1
}
