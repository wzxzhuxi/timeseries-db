use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use serde_json::Value;

use crate::db::{TimeSeriesDB, DataPoint};
use super::models::{
    CreateDataPointRequest, UpdateDataPointRequest, QueryRequest, 
    ApiResponse, DataPointResponse, SeriesListResponse, CompactRequest
};

pub type AppState = TimeSeriesDB;

// 创建数据点
pub async fn create_datapoint(
    State(db): State<AppState>,
    Json(request): Json<CreateDataPointRequest>,
) -> Json<ApiResponse<String>> {
    let tags = request.tags.unwrap_or_default();
    
    let datapoint = DataPoint {
        timestamp: request.timestamp,
        value: request.value,
        tags,
    };

    match db.insert(request.series_key.clone(), datapoint).await {
        Ok(_) => Json(ApiResponse::success(format!(
            "数据点已添加到系列: {} (时间戳: {})",
            request.series_key, request.timestamp
        ))),
        Err(e) => {
            tracing::error!("创建数据点失败: {}", e);
            Json(ApiResponse::error(format!("创建数据点失败: {}", e)))
        }
    }
}

// 批量创建数据点
pub async fn create_datapoints_batch(
    State(db): State<AppState>,
    Json(requests): Json<Vec<CreateDataPointRequest>>,
) -> Json<ApiResponse<String>> {
    let mut success_count = 0;
    let mut error_count = 0;

    for request in requests {
        let tags = request.tags.unwrap_or_default();
        
        let datapoint = DataPoint {
            timestamp: request.timestamp,
            value: request.value,
            tags,
        };

        match db.insert(request.series_key.clone(), datapoint).await {
            Ok(_) => success_count += 1,
            Err(e) => {
                tracing::error!("批量创建数据点失败: {}", e);
                error_count += 1;
            }
        }
    }

    Json(ApiResponse::success(format!(
        "批量创建完成: 成功 {} 个，失败 {} 个",
        success_count, error_count
    )))
}

// 查询数据点
pub async fn query_datapoints(
    State(db): State<AppState>,
    Path(series_key): Path<String>,
    Query(query): Query<QueryRequest>,
) -> Json<ApiResponse<Vec<DataPointResponse>>> {
    match db.query_range(&series_key, query.start_time, query.end_time).await {
        Ok(datapoints) => {
            let response_data: Vec<DataPointResponse> = datapoints
                .into_iter()
                .map(|dp| DataPointResponse {
                    timestamp: dp.timestamp,
                    value: dp.value,
                    tags: dp.tags,
                })
                .collect();
            
            tracing::info!("查询系列 {} 返回 {} 个数据点", series_key, response_data.len());
            Json(ApiResponse::success(response_data))
        }
        Err(e) => {
            tracing::error!("查询数据点失败: {}", e);
            Json(ApiResponse::error(format!("查询数据点失败: {}", e)))
        }
    }
}

// 更新数据点
pub async fn update_datapoint(
    State(db): State<AppState>,
    Path((series_key, timestamp)): Path<(String, u64)>,
    Json(request): Json<UpdateDataPointRequest>,
) -> Json<ApiResponse<String>> {
    match db.update(&series_key, timestamp, request.value).await {
        Ok(updated) => {
            if updated {
                tracing::info!("数据点已更新: {} at {} -> {}", series_key, timestamp, request.value);
                Json(ApiResponse::success(format!(
                    "数据点已更新: {} at {} -> {}",
                    series_key, timestamp, request.value
                )))
            } else {
                Json(ApiResponse::error(
                    "未找到指定的数据点".to_string()
                ))
            }
        }
        Err(e) => {
            tracing::error!("更新数据点失败: {}", e);
            Json(ApiResponse::error(format!("更新数据点失败: {}", e)))
        }
    }
}

// 删除数据点
pub async fn delete_datapoint(
    State(db): State<AppState>,
    Path((series_key, timestamp)): Path<(String, u64)>,
) -> Json<ApiResponse<String>> {
    match db.delete(&series_key, Some(timestamp)).await {
        Ok(deleted) => {
            if deleted {
                tracing::info!("数据点已删除: {} at {}", series_key, timestamp);
                Json(ApiResponse::success(format!(
                    "数据点已删除: {} at {}",
                    series_key, timestamp
                )))
            } else {
                Json(ApiResponse::error(
                    "未找到指定的数据点".to_string()
                ))
            }
        }
        Err(e) => {
            tracing::error!("删除数据点失败: {}", e);
            Json(ApiResponse::error(format!("删除数据点失败: {}", e)))
        }
    }
}

// 删除整个系列
pub async fn delete_series(
    State(db): State<AppState>,
    Path(series_key): Path<String>,
) -> Json<ApiResponse<String>> {
    match db.delete(&series_key, None).await {
        Ok(deleted) => {
            if deleted {
                tracing::info!("系列已删除: {}", series_key);
                Json(ApiResponse::success(format!(
                    "系列已删除: {}",
                    series_key
                )))
            } else {
                Json(ApiResponse::error(
                    "未找到指定的系列".to_string()
                ))
            }
        }
        Err(e) => {
            tracing::error!("删除系列失败: {}", e);
            Json(ApiResponse::error(format!("删除系列失败: {}", e)))
        }
    }
}

// 获取所有系列列表
pub async fn list_series(
    State(db): State<AppState>,
) -> Json<ApiResponse<SeriesListResponse>> {
    match db.get_all_series().await {
        Ok(series) => {
            let response = SeriesListResponse::new(series);
            Json(ApiResponse::success(response))
        }
        Err(e) => {
            tracing::error!("获取系列列表失败: {}", e);
            Json(ApiResponse::error(format!("获取系列列表失败: {}", e)))
        }
    }
}

// 手动触发compaction
pub async fn trigger_compaction(
    State(db): State<AppState>,
    Json(_request): Json<CompactRequest>,
) -> Json<ApiResponse<String>> {
    match db.compact().await {
        Ok(_) => {
            tracing::info!("手动compaction执行完成");
            Json(ApiResponse::success(
                "Compaction执行完成".to_string()
            ))
        }
        Err(e) => {
            tracing::error!("Compaction执行失败: {}", e);
            Json(ApiResponse::error(format!("Compaction执行失败: {}", e)))
        }
    }
}

// 健康检查
pub async fn health_check() -> Json<Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "时序数据库",
        "version": "1.0.0",
        "timestamp": chrono::Utc::now().timestamp(),
        "features": [
            "LSM-Tree存储引擎",
            "Gorilla压缩算法",
            "mmap零拷贝技术",
            "异步HTTP API"
        ]
    }))
}

// 数据库统计信息
pub async fn db_stats(
    State(db): State<AppState>,
) -> Json<ApiResponse<Value>> {
    match db.get_stats().await {
        Ok(stats) => {
            let response = serde_json::json!({
                "storage_engine": "LSM-Tree",
                "compression": "Gorilla",
                "memory_mapping": "mmap零拷贝",
                "status": "运行中",
                "memtable_size": stats.memtable_size,
                "sstable_count": stats.sstable_count,
                "total_series": stats.total_series,
                "timestamp": chrono::Utc::now().timestamp()
            });
            
            Json(ApiResponse::success(response))
        }
        Err(e) => {
            tracing::error!("获取数据库统计信息失败: {}", e);
            Json(ApiResponse::error(format!("获取数据库统计信息失败: {}", e)))
        }
    }
}

// 获取系列详细信息
pub async fn get_series_info(
    State(db): State<AppState>,
    Path(series_key): Path<String>,
) -> Json<ApiResponse<Value>> {
    match db.query_range(&series_key, None, None).await {
        Ok(datapoints) => {
            let count = datapoints.len();
            let min_timestamp = datapoints.iter().map(|dp| dp.timestamp).min();
            let max_timestamp = datapoints.iter().map(|dp| dp.timestamp).max();
            let min_value = datapoints.iter().map(|dp| dp.value).fold(f64::INFINITY, f64::min);
            let max_value = datapoints.iter().map(|dp| dp.value).fold(f64::NEG_INFINITY, f64::max);
            
            let info = serde_json::json!({
                "series_key": series_key,
                "count": count,
                "min_timestamp": min_timestamp,
                "max_timestamp": max_timestamp,
                "min_value": if min_value.is_finite() { Some(min_value) } else { None },
                "max_value": if max_value.is_finite() { Some(max_value) } else { None },
                "tags": if !datapoints.is_empty() { 
                    Some(&datapoints[0].tags) 
                } else { 
                    None 
                }
            });
            
            Json(ApiResponse::success(info))
        }
        Err(e) => {
            tracing::error!("获取系列信息失败: {}", e);
            Json(ApiResponse::error(format!("获取系列信息失败: {}", e)))
        }
    }
}

