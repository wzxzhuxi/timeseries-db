use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateDataPointRequest {
    pub series_key: String,
    pub timestamp: u64,
    pub value: f64,
    pub tags: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateDataPointRequest {
    pub value: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryRequest {
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompactRequest {
    pub force: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
    pub timestamp: i64,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            message: "操作成功".to_string(),
            data: Some(data),
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            message,
            data: None,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataPointResponse {
    pub timestamp: u64,
    pub value: f64,
    pub tags: BTreeMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SeriesListResponse {
    pub series: Vec<String>,
    pub count: usize,
}

impl SeriesListResponse {
    pub fn new(series: Vec<String>) -> Self {
        let count = series.len();
        Self { series, count }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchInsertRequest {
    pub datapoints: Vec<CreateDataPointRequest>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: u16,
    pub timestamp: i64,
}

impl ErrorResponse {
    pub fn new(error: String, code: u16) -> Self {
        Self {
            error,
            code,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}

