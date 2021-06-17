use serde::Deserialize;
use serde::Serialize;
use std::cmp::Reverse;
use std::fs::File;
use std::io::BufReader;
use std::io::Write;

#[derive(Serialize, Deserialize)]
pub struct Score {
    pub value: i16,
}

pub struct Scores {
    pub scores: Vec<Score>,
}

impl Scores {
    pub fn new(path: &str) -> Self {
        Scores::load(path)
    }

    pub fn load(path: &str) -> Self {
        let file = File::open(path).unwrap();
        let reader = BufReader::new(file);
        let scores: Vec<Score> = serde_json::from_reader(reader).unwrap();
        Self { scores }
    }

    pub fn save(&self, path: &str) {
        let j = serde_json::to_string(&self.scores).unwrap();
        let mut f = File::create(path).unwrap();
        f.write_all(&j.as_bytes()).unwrap();
    }

    pub fn sort(&mut self) {
        self.scores.sort_by_key(|s| s.value);
    }
}
