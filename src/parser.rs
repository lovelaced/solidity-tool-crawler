use serde_json::Value;
use std::fs::File;
use flate2::read::GzDecoder;
use std::io::{BufReader, BufRead};
use rayon::prelude::*;

pub fn parse_push_events(file_path: &str) -> Vec<Value> {
    let file = File::open(file_path).expect("Unable to open file");
    let decoder = GzDecoder::new(file);
    let reader = BufReader::new(decoder);

    let events = reader.lines()
        .par_bridge() // Convert to parallel iterator
        .filter_map(|line| {
            if let Ok(line) = line {
                serde_json::from_str::<Value>(&line).ok()
            } else {
                None
            }
        })
        .filter(|event| event["type"] == "PushEvent")
        .collect::<Vec<_>>();

    events
}

