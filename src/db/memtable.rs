use std::collections::BTreeMap;
use super::DataPoint;

#[derive(Debug)]
pub struct Memtable {
    data: BTreeMap<String, Vec<DataPoint>>,
    size: usize,
    threshold: usize,
}

impl Memtable {
    pub fn new(threshold: usize) -> Self {
        Self {
            data: BTreeMap::new(),
            size: 0,
            threshold,
        }
    }

    pub fn insert(&mut self, series_key: String, datapoint: DataPoint) {
        let entry = self.data.entry(series_key).or_default();
        entry.push(datapoint);
        self.size += 1;
    }

    pub fn update(&mut self, series_key: &str, timestamp: u64, new_value: f64) -> bool {
        if let Some(datapoints) = self.data.get_mut(series_key) {
            if let Some(dp) = datapoints.iter_mut().find(|dp| dp.timestamp == timestamp) {
                dp.value = new_value;
                return true;
            }
        }
        false
    }

    pub fn delete(&mut self, series_key: &str, timestamp: Option<u64>) -> bool {
        match timestamp {
            Some(ts) => {
                if let Some(datapoints) = self.data.get_mut(series_key) {
                    let original_len = datapoints.len();
                    datapoints.retain(|dp| dp.timestamp != ts);
                    let removed = original_len > datapoints.len();
                    if removed {
                        self.size -= 1;
                    }
                    if datapoints.is_empty() {
                        self.data.remove(series_key);
                    }
                    removed
                } else {
                    false
                }
            }
            None => {
                if let Some(datapoints) = self.data.remove(series_key) {
                    self.size -= datapoints.len();
                    true
                } else {
                    false
                }
            }
        }
    }

    pub fn is_full(&self) -> bool {
        self.size >= self.threshold
    }

    pub fn clear(&mut self) {
        self.data.clear();
        self.size = 0;
    }

    pub fn get_data(&self) -> &BTreeMap<String, Vec<DataPoint>> {
        &self.data
    }

    pub fn query(&self, series_key: &str, start_time: Option<u64>, end_time: Option<u64>) -> Vec<DataPoint> {
        if let Some(datapoints) = self.data.get(series_key) {
            datapoints.iter()
                .filter(|dp| {
                    if let Some(start) = start_time {
                        if dp.timestamp < start {
                            return false;
                        }
                    }
                    if let Some(end) = end_time {
                        if dp.timestamp > end {
                            return false;
                        }
                    }
                    true
                })
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }
}

