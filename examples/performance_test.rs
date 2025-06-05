//! 性能测试示例
//! 
//! 本示例专门用于测试时序数据库的各项性能指标：
//! - 写入性能测试
//! - 查询性能测试
//! - 并发性能测试
//! - 内存使用测试
//! - 压缩性能测试

use std::collections::BTreeMap;
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use std::time::{Instant, Duration, SystemTime, UNIX_EPOCH};
use tempfile::TempDir;
use timeseries_db::{TimeSeriesDB, DataPoint};
use tokio::task::JoinSet;

/// 性能测试结果
#[derive(Debug)]
struct PerformanceResult {
    operation: String,
    duration: Duration,
    operations_count: u64,
    ops_per_second: f64,
    avg_latency_ms: f64,
    memory_used_mb: Option<f64>,
}

impl PerformanceResult {
    fn new(operation: String, duration: Duration, operations_count: u64) -> Self {
        let ops_per_second = operations_count as f64 / duration.as_secs_f64();
        let avg_latency_ms = duration.as_millis() as f64 / operations_count as f64;
        
        Self {
            operation,
            duration,
            operations_count,
            ops_per_second,
            avg_latency_ms,
            memory_used_mb: None,
        }
    }

    fn with_memory(mut self, memory_mb: f64) -> Self {
        self.memory_used_mb = Some(memory_mb);
        self
    }

    fn print(&self) {
        println!("📊 {}", self.operation);
        println!("  操作数量: {}", self.operations_count);
        println!("  总耗时: {:?}", self.duration);
        println!("  OPS: {:.2}", self.ops_per_second);
        println!("  平均延迟: {:.2} ms", self.avg_latency_ms);
        if let Some(memory) = self.memory_used_mb {
            println!("  内存使用: {:.2} MB", memory);
        }
        println!();
    }
}

/// 生成测试数据点
fn generate_datapoint(series_key: &str, timestamp: u64, base_value: f64) -> DataPoint {
    let mut tags = BTreeMap::new();
    tags.insert("test".to_string(), "performance".to_string());
    tags.insert("series".to_string(), series_key.to_string());
    
    // 添加一些变化以模拟真实数据
    let variation = ((timestamp % 100) as f64 - 50.0) * 0.1;
    
    DataPoint {
        timestamp,
        value: base_value + variation,
        tags,
    }
}

/// 获取当前进程内存使用量（简化版本）
fn get_memory_usage_mb() -> f64 {
    // 这里使用一个简化的内存估算
    // 在实际应用中，可以使用系统调用获取准确的内存使用量
    let stats = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    for line in stats.lines() {
        if line.starts_with("VmRSS:") {
            if let Some(size_str) = line.split_whitespace().nth(1) {
                if let Ok(size_kb) = size_str.parse::<u64>() {
                    return size_kb as f64 / 1024.0; // 转换为MB
                }
            }
        }
    }
    0.0 // 如果无法获取，返回0
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🚀 时序数据库性能测试");
    println!("========================\n");

    // 创建临时目录
    let temp_dir = TempDir::new()?;
    println!("📁 测试数据目录: {:?}", temp_dir.path());

    // 创建数据库实例
    let db = TimeSeriesDB::new(temp_dir.path(), 10000)?; // 较大的内存表阈值
    println!("✅ 数据库创建成功\n");

    let mut results = Vec::new();

    // 测试1: 单点写入性能
    println!("🔥 测试1: 单点写入性能");
    println!("------------------------");
    
    let write_count = 10000;
    let start_timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let memory_before = get_memory_usage_mb();
    
    let start = Instant::now();
    for i in 0..write_count {
        let datapoint = generate_datapoint("perf_single", start_timestamp + i, 100.0);
        db.insert("perf_single".to_string(), datapoint).await?;
        
        if (i + 1) % 1000 == 0 {
            print!(".");
            if (i + 1) % 10000 == 0 {
                println!(" {} 完成", i + 1);
            }
        }
    }
    let duration = start.elapsed();
    let memory_after = get_memory_usage_mb();
    
    let result = PerformanceResult::new(
        "单点写入测试".to_string(),
        duration,
        write_count,
    ).with_memory(memory_after - memory_before);
    
    result.print();
    results.push(result);

    // 测试2: 批量数据生成和插入
    println!("📦 测试2: 多系列批量写入");
    println!("------------------------");
    
    let series_count = 50;
    let points_per_series = 1000;
    let total_batch_points = series_count * points_per_series;
    
    let start = Instant::now();
    for series_id in 0..series_count {
        let series_key = format!("perf_batch_{}", series_id);
        
        for point_id in 0..points_per_series {
            let timestamp = start_timestamp + 10000 + point_id; // 避免与之前的数据重叠
            let datapoint = generate_datapoint(&series_key, timestamp, 200.0 + series_id as f64);
            db.insert(series_key.clone(), datapoint).await?;
        }
        
        if (series_id + 1) % 10 == 0 {
            println!("  已完成 {} / {} 个系列", series_id + 1, series_count);
        }
    }
    let duration = start.elapsed();
    
    let result = PerformanceResult::new(
        "多系列批量写入".to_string(),
        duration,
        total_batch_points,
    );
    
    result.print();
    results.push(result);

    // 测试3: 并发写入性能
    println!("⚡ 测试3: 并发写入性能");
    println!("------------------------");
    
    let concurrent_tasks = 20;
    let points_per_task = 500;
    let total_concurrent_points = concurrent_tasks * points_per_task;
    
    let start = Instant::now();
    let mut join_set = JoinSet::new();
    let counter = Arc::new(AtomicU64::new(0));
    
    for task_id in 0..concurrent_tasks {
        let db_clone = db.clone();
        let counter_clone = Arc::clone(&counter);
        
        join_set.spawn(async move {
            for point_id in 0..points_per_task {
                let series_key = format!("perf_concurrent_{}_{}", task_id, point_id % 10);
                let timestamp = start_timestamp + 20000 + (task_id * points_per_task + point_id);
                let datapoint = generate_datapoint(&series_key, timestamp, 300.0 + task_id as f64);
                
                if let Err(e) = db_clone.insert(series_key, datapoint).await {
                    eprintln!("插入错误: {}", e);
                } else {
                    counter_clone.fetch_add(1, Ordering::Relaxed);
                }
            }
        });
    }
    
    // 等待所有任务完成
    while let Some(result) = join_set.join_next().await {
        result?;
    }

    println!("总并发数据点: {}", total_concurrent_points);
    
    let duration = start.elapsed();
    let successful_inserts = counter.load(Ordering::Relaxed);
    
    let result = PerformanceResult::new(
        "并发写入测试".to_string(),
        duration,
        successful_inserts,
    );
    
    result.print();
    results.push(result);

    // 测试4: 查询性能
    println!("🔍 测试4: 查询性能测试");
    println!("------------------------");
    
    let query_count = 1000;
    let start = Instant::now();
    
    for i in 0..query_count {
        let series_key = format!("perf_batch_{}", i % series_count);
        let _data = db.query_range(&series_key, None, None).await?;
        
        if (i + 1) % 100 == 0 {
            print!(".");
            if (i + 1) % 1000 == 0 {
                println!(" {} 完成", i + 1);
            }
        }
    }
    
    let duration = start.elapsed();
    
    let result = PerformanceResult::new(
        "全量查询测试".to_string(),
        duration,
        query_count,
    );
    
    result.print();
    results.push(result);

    // 测试5: 范围查询性能
    println!("📅 测试5: 范围查询性能");
    println!("------------------------");
    
    let range_query_count = 500;
    let range_size = 100; // 查询100个时间点的范围
    
    let start = Instant::now();
    for i in 0..range_query_count {
        let series_key = format!("perf_batch_{}", i % series_count);
        let range_start = start_timestamp + 10000 + (i % 500);
        let range_end = range_start + range_size;
        
        let _data = db.query_range(&series_key, Some(range_start), Some(range_end)).await?;
    }
    let duration = start.elapsed();
    
    let result = PerformanceResult::new(
        "范围查询测试".to_string(),
        duration,
        range_query_count,
    );
    
    result.print();
    results.push(result);

    // 测试6: 压缩性能
    println!("🗜️ 测试6: 数据压缩性能");
    println!("------------------------");
    
    let start = Instant::now();
    db.compact().await?;
    let duration = start.elapsed();
    
    let result = PerformanceResult::new(
        "数据压缩测试".to_string(),
        duration,
        1, // 只执行一次压缩操作
    );
    
    result.print();
    results.push(result);

    // 测试7: 更新操作性能
    println!("✏️ 测试7: 更新操作性能");
    println!("------------------------");
    
    let update_count = 1000;
    let start = Instant::now();
    
    for i in 0..update_count {
        let timestamp = start_timestamp + i;
        let new_value = 150.0 + (i as f64 * 0.1);
        let _updated = db.update("perf_single", timestamp, new_value).await?;
    }
    
    let duration = start.elapsed();
    
    let result = PerformanceResult::new(
        "数据更新测试".to_string(),
        duration,
        update_count,
    );
    
    result.print();
    results.push(result);

    // 测试8: 删除操作性能
    println!("🗑️ 测试8: 删除操作性能");
    println!("------------------------");
    
    let delete_count = 500;
    let start = Instant::now();
    
    for i in 0..delete_count {
        let timestamp = start_timestamp + i;
        let _deleted = db.delete("perf_single", Some(timestamp)).await?;
    }
    
    let duration = start.elapsed();
    
    let result = PerformanceResult::new(
        "数据删除测试".to_string(),
        duration,
        delete_count,
    );
    
    result.print();
    results.push(result);

    // 获取最终数据库统计
    println!("📊 最终数据库统计");
    println!("------------------");
    let final_stats = db.get_stats().await?;
    println!("内存表大小: {}", final_stats.memtable_size);
    println!("SSTable数量: {}", final_stats.sstable_count);
    println!("总系列数: {}", final_stats.total_series);
    
    let all_series = db.get_all_series().await?;
    let mut total_data_points = 0;
    for series_key in &all_series {
        let data = db.query_range(series_key, None, None).await?;
        total_data_points += data.len();
    }
    println!("总数据点数: {}", total_data_points);
    println!("最终内存使用: {:.2} MB", get_memory_usage_mb());
    println!();

    // 性能总结报告
    println!("📈 性能测试总结报告");
    println!("====================");
    println!("{:<20} {:<12} {:<12} {:<15} {:<15}", "测试类型", "操作数量", "耗时(ms)", "OPS", "平均延迟(ms)");
    println!("{}", "-".repeat(80));
    
    for result in &results {
        println!("{:<20} {:<12} {:<12} {:<15.2} {:<15.2}",
                 result.operation,
                 result.operations_count,
                 result.duration.as_millis(),
                 result.ops_per_second,
                 result.avg_latency_ms);
    }
    println!();

    // 性能评估
    println!("🎯 性能评估");
    println!("-----------");
    
    let write_performance = results.iter()
        .find(|r| r.operation.contains("单点写入"))
        .map(|r| r.ops_per_second)
        .unwrap_or(0.0);
    
    let query_performance = results.iter()
        .find(|r| r.operation.contains("全量查询"))
        .map(|r| r.ops_per_second)
        .unwrap_or(0.0);
    
    let concurrent_performance = results.iter()
        .find(|r| r.operation.contains("并发写入"))
        .map(|r| r.ops_per_second)
        .unwrap_or(0.0);

    println!("✅ 写入性能: {:.0} TPS {}", 
             write_performance,
             if write_performance > 1000.0 { "(优秀)" } else if write_performance > 500.0 { "(良好)" } else { "(需改进)" });
    
    println!("✅ 查询性能: {:.0} QPS {}", 
             query_performance,
             if query_performance > 100.0 { "(优秀)" } else if query_performance > 50.0 { "(良好)" } else { "(需改进)" });
    
    println!("✅ 并发性能: {:.0} TPS {}", 
             concurrent_performance,
             if concurrent_performance > 1500.0 { "(优秀)" } else if concurrent_performance > 1000.0 { "(良好)" } else { "(需改进)" });

    // 性能建议
    println!("\n💡 性能优化建议:");
    if write_performance < 1000.0 {
        println!("  - 考虑增加内存表大小以减少flush频率");
        println!("  - 使用SSD存储以提升写入性能");
    }
    if query_performance < 100.0 {
        println!("  - 考虑添加索引或优化查询策略");
        println!("  - 增加系统内存以提升缓存效果");
    }
    if concurrent_performance < 1500.0 {
        println!("  - 优化锁竞争，考虑分片策略");
        println!("  - 调整线程池大小");
    }

    println!("\n🎉 性能测试完成！");
    println!("测试数据将在程序结束时自动清理");

    Ok(())
}

