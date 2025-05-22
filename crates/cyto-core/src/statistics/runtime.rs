use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RuntimeStatistics {
    #[serde(rename = "total_runtime_sec")]
    total_runtime: f64,
    #[serde(rename = "init_time_sec")]
    init_time: f64,
    #[serde(rename = "map_time_sec")]
    map_time: f64,
    #[serde(rename = "map_throughput_reads_per_sec")]
    map_throughput: f64,
}

impl RuntimeStatistics {
    pub fn new(total_runtime: f64, init_time: f64, map_time: f64, map_throughput: f64) -> Self {
        Self {
            total_runtime,
            init_time,
            map_time,
            map_throughput,
        }
    }
}
