//! 批量插入示例
//! 
//! 本示例演示了如何高效地进行批量数据插入：
//! - 模拟多个传感器的数据
//! - 批量生成和插入数据
//! - 性能测试和监控
//! - 数据压缩和存储优化

use std::collections::BTreeMap;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tempfile::TempDir;
use timeseries_db::{TimeSeriesDB, DataPoint};

/// 传感器配置
#[derive(Clone)]
struct SensorConfig {
    name: String,
    sensor_type: String,
    location: String,
    base_value: f64,
    variation: f64,
    interval_seconds: u64,
}

impl SensorConfig {
    fn new(name: &str, sensor_type: &str, location: &str, base_value: f64, variation: f64, interval_seconds: u64) -> Self {
        Self {
            name: name.to_string(),
            sensor_type: sensor_type.to_string(),
            location: location.to_string(),
            base_value,
            variation,
            interval_seconds,
        }
    }

    fn generate_tags(&self) -> BTreeMap<String, String> {
        let mut tags = BTreeMap::new();
        tags.insert("sensor_type".to_string(), self.sensor_type.clone());
        tags.insert("location".to_string(), self.location.clone());
        tags.insert("device_name".to_string(), self.name.clone());
        tags
    }

    fn generate_value(&self, time_offset: u64) -> f64 {
        // 模拟真实的传感器数据变化
        let time_factor = (time_offset as f64 / 3600.0) * std::f64::consts::PI * 2.0; // 24小时周期
        let noise = (time_offset as f64 * 0.1).sin() * 0.1; // 添加一些噪声
        
        self.base_value + 
        (time_factor.sin() * self.variation) + 
        (noise * self.variation * 0.3)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("📦 时序数据库批量插入示例");
    println!("==============================\n");

    // 创建临时目录
    let temp_dir = TempDir::new()?;
    println!("📁 数据目录: {:?}", temp_dir.path());

    // 创建数据库实例，使用较小的内存表阈值来演示flush行为
    let db = TimeSeriesDB::new(temp_dir.path(), 500)?;
    println!("✅ 数据库创建成功（内存表阈值: 500）\n");

    // 配置多个传感器
    let sensors = vec![
        SensorConfig::new("temp_001", "temperature", "server_room_a", 22.0, 5.0, 60),
        SensorConfig::new("temp_002", "temperature", "server_room_b", 24.0, 4.0, 60),
        SensorConfig::new("temp_003", "temperature", "server_room_c", 23.0, 3.5, 60),
        SensorConfig::new("humi_001", "humidity", "server_room_a", 45.0, 15.0, 120),
        SensorConfig::new("humi_002", "humidity", "server_room_b", 50.0, 12.0, 120),
        SensorConfig::new("pres_001", "pressure", "server_room_a", 1013.25, 50.0, 300),
        SensorConfig::new("cpu_001", "cpu_usage", "server_001", 45.0, 30.0, 30),
        SensorConfig::new("cpu_002", "cpu_usage", "server_002", 35.0, 25.0, 30),
        SensorConfig::new("mem_001", "memory_usage", "server_001", 65.0, 20.0, 30),
        SensorConfig::new("mem_002", "memory_usage", "server_002", 70.0, 18.0, 30),
    ];

    println!("🔧 配置了 {} 个传感器:", sensors.len());
    for sensor in &sensors {
        println!("  - {} ({}): {}，基础值: {:.1}",
                 sensor.name, sensor.sensor_type, sensor.location, sensor.base_value);
    }
    println!();

    // 批量插入参数
    let duration_hours = 2; // 生成2小时的数据
    let batch_size = 100; // 每批次插入的数据点数量
    
    println!("📊 批量插入参数:");
    println!("  时间跨度: {} 小时", duration_hours);
    println!("  批次大小: {} 个数据点", batch_size);
    println!();

    // 生成数据并批量插入
    let start_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs() - (duration_hours * 3600); // 从2小时前开始

    let mut total_points = 0;
    let mut batch_count = 0;
    let insert_start = Instant::now();

    println!("🚀 开始批量数据生成和插入...\n");

    // 为每个传感器生成数据
    for sensor in &sensors {
        println!("📡 处理传感器: {}", sensor.name);
        
        let mut sensor_points = 0;
        let sensor_start = Instant::now();
        
        // 计算该传感器在指定时间内的数据点数量
        let total_points_for_sensor = (duration_hours * 3600) / sensor.interval_seconds;
        
        for i in 0..total_points_for_sensor {
            let timestamp = start_timestamp + i * sensor.interval_seconds;
            let value = sensor.generate_value(i * sensor.interval_seconds);
            
            let datapoint = DataPoint {
                timestamp,
                value,
                tags: sensor.generate_tags(),
            };

            // 插入数据点
            db.insert(sensor.name.clone(), datapoint).await?;
            sensor_points += 1;
            total_points += 1;

            // 每批次报告进度
            if sensor_points % batch_size == 0 {
                batch_count += 1;
                let elapsed = insert_start.elapsed();
                let tps = total_points as f64 / elapsed.as_secs_f64();
                
                println!("  批次 {}: 已插入 {} 个数据点 (TPS: {:.1})", 
                         batch_count, total_points, tps);
            }
        }

        let sensor_elapsed = sensor_start.elapsed();
        println!("  ✅ {} 完成: {} 个数据点，耗时 {:?}\n", 
                 sensor.name, sensor_points, sensor_elapsed);
    }

    let total_elapsed = insert_start.elapsed();
    let overall_tps = total_points as f64 / total_elapsed.as_secs_f64();

    println!("🎉 批量插入完成！");
    println!("总结:");
    println!("  传感器数量: {}", sensors.len());
    println!("  总数据点数: {}", total_points);
    println!("  总耗时: {:?}", total_elapsed);
    println!("  平均TPS: {:.1}", overall_tps);
    println!();

    // 获取数据库统计信息
    println!("📊 数据库统计信息:");
    let stats = db.get_stats().await?;
    println!("  内存表大小: {}", stats.memtable_size);
    println!("  SSTable数量: {}", stats.sstable_count);
    println!("  总系列数: {}", stats.total_series);
    println!();

    // 验证数据插入
    println!("🔍 验证数据插入...");
    let all_series = db.get_all_series().await?;
    println!("系列验证:");
    
    for series_key in &all_series {
        let data = db.query_range(series_key, None, None).await?;
        println!("  - {}: {} 个数据点", series_key, data.len());
        
        if !data.is_empty() {
            let min_val = data.iter().map(|dp| dp.value).fold(f64::INFINITY, f64::min);
            let max_val = data.iter().map(|dp| dp.value).fold(f64::NEG_INFINITY, f64::max);
            let avg_val = data.iter().map(|dp| dp.value).sum::<f64>() / data.len() as f64;
            
            println!("    范围: {:.2} - {:.2}, 平均: {:.2}", min_val, max_val, avg_val);
        }
    }
    println!();

    // 时间范围查询测试
    println!("⏰ 时间范围查询测试...");
    let query_start = Instant::now();
    
    // 查询最近1小时的温度数据
    let recent_start = start_timestamp + (duration_hours - 1) * 3600;
    let recent_end = start_timestamp + duration_hours * 3600;
    
    for sensor in &sensors {
        if sensor.sensor_type == "temperature" {
            let recent_data = db.query_range(&sensor.name, Some(recent_start), Some(recent_end)).await?;
            println!("  {} 最近1小时: {} 个数据点", sensor.name, recent_data.len());
        }
    }
    
    let query_elapsed = query_start.elapsed();
    println!("  查询耗时: {:?}\n", query_elapsed);

    // 数据压缩测试
    println!("🔄 执行数据压缩...");
    let compact_start = Instant::now();
    db.compact().await?;
    let compact_elapsed = compact_start.elapsed();
    
    println!("✅ 压缩完成，耗时: {:?}", compact_elapsed);
    
    // 压缩后统计
    let stats_after = db.get_stats().await?;
    println!("压缩后统计:");
    println!("  内存表大小: {} (变化: {})", 
             stats_after.memtable_size, 
             stats_after.memtable_size as i32 - stats.memtable_size as i32);
    println!("  SSTable数量: {} (变化: {})", 
             stats_after.sstable_count, 
             stats_after.sstable_count as i32 - stats.sstable_count as i32);
    println!("  总系列数: {}", stats_after.total_series);
    println!();

    // 性能总结
    println!("📈 性能总结:");
    println!("  写入性能: {:.1} TPS", overall_tps);
    println!("  查询性能: 平均 {:.2} ms", query_elapsed.as_millis() as f64 / sensors.len() as f64);
    println!("  压缩性能: {:.2} 秒", compact_elapsed.as_secs_f64());
    println!();

    // 数据分布分析
    println!("📊 数据分布分析:");
    let mut type_counts = BTreeMap::new();
    let mut location_counts = BTreeMap::new();
    
    for series_key in &all_series {
        let data = db.query_range(series_key, None, None).await?;
        if let Some(first_point) = data.first() {
            let sensor_type = first_point.tags.get("sensor_type").unwrap_or(&"unknown".to_string()).clone();
            let location = first_point.tags.get("location").unwrap_or(&"unknown".to_string()).clone();
            
            *type_counts.entry(sensor_type).or_insert(0) += data.len();
            *location_counts.entry(location).or_insert(0) += data.len();
        }
    }
    
    println!("按传感器类型分布:");
    for (sensor_type, count) in type_counts {
        println!("  {}: {} 个数据点", sensor_type, count);
    }
    
    println!("按位置分布:");
    for (location, count) in location_counts {
        println!("  {}: {} 个数据点", location, count);
    }
    println!();


    // 批量删除测试 - 添加错误处理
println!("🗑️ 批量删除测试...");
let delete_start = Instant::now();

// 删除所有CPU使用率数据 - 逐个安全删除
let mut deleted_series = 0;
let cpu_series: Vec<String> = all_series.iter()
    .filter(|key| key.contains("cpu_"))
    .cloned()
    .collect();

for series_key in cpu_series {
    println!("  🗑️ 正在删除系列: {}", series_key);
    match db.delete(&series_key, None).await {
        Ok(deleted) => {
            if deleted {
                deleted_series += 1;
                println!("  ✅ 删除系列成功: {}", series_key);
            } else {
                println!("  ⚠️ 系列不存在或已删除: {}", series_key);
            }
        }
        Err(e) => {
            println!("  ❌ 删除系列失败: {} - {}", series_key, e);
        }
    }
    
    // 添加小延迟避免过快操作
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
}

let delete_elapsed = delete_start.elapsed();
println!("删除完成: {} 个系列，耗时: {:?}\n", deleted_series, delete_elapsed);




    // 最终验证
    println!("🔍 最终数据验证...");
    let final_series = db.get_all_series().await?;
    let final_stats = db.get_stats().await?;
    
    println!("最终状态:");
    println!("  剩余系列数: {}", final_series.len());
    println!("  内存表大小: {}", final_stats.memtable_size);
    println!("  SSTable数量: {}", final_stats.sstable_count);
    
    let mut final_total_points = 0;
    for series_key in &final_series {
        let data = db.query_range(series_key, None, None).await?;
        final_total_points += data.len();
    }
    println!("  总数据点数: {}", final_total_points);

    println!("\n🎉 批量插入示例演示完成！");
    println!("本示例演示了:");
    println!("  ✅ 多传感器数据模拟");
    println!("  ✅ 高效批量插入");
    println!("  ✅ 性能监控和测试");
    println!("  ✅ 数据压缩和优化");
    println!("  ✅ 批量查询和删除");

    Ok(())
}

