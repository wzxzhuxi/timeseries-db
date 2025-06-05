//! 基础使用示例
//! 
//! 本示例演示了时序数据库的基本操作：
//! - 创建数据库实例
//! - 插入数据点
//! - 查询数据
//! - 更新和删除操作

use std::collections::BTreeMap;
use tempfile::TempDir;
use timeseries_db::{TimeSeriesDB, DataPoint};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🚀 时序数据库基础使用示例");
    println!("==============================\n");

    // 创建临时目录用于存储数据
    let temp_dir = TempDir::new()?;
    println!("📁 数据目录: {:?}", temp_dir.path());

    // 创建数据库实例
    let db = TimeSeriesDB::new(temp_dir.path(), 100)?;
    println!("✅ 数据库创建成功\n");

    // 示例1: 创建传感器标签
    let mut sensor_tags = BTreeMap::new();
    sensor_tags.insert("sensor_type".to_string(), "temperature".to_string());
    sensor_tags.insert("location".to_string(), "server_room_1".to_string());
    sensor_tags.insert("device_id".to_string(), "temp_001".to_string());

    // 示例2: 插入单个数据点
    println!("📊 插入单个数据点...");
    let datapoint1 = DataPoint {
        timestamp: 1609459200, // 2021-01-01 00:00:00 UTC
        value: 23.5,
        tags: sensor_tags.clone(),
    };

    db.insert("temperature_sensor_1".to_string(), datapoint1).await?;
    println!("✅ 数据点插入成功: temperature_sensor_1 = 23.5°C\n");

    // 示例3: 插入多个数据点（模拟1小时的数据，每分钟一个点）
    println!("📈 插入1小时的温度数据（每分钟一个点）...");
    for i in 1..=60 {
        let timestamp = 1609459200 + i * 60; // 每分钟增加60秒
        let value = 23.5 + (i as f64) * 0.1 + ((i % 10) as f64 - 5.0) * 0.05; // 模拟温度变化

        let datapoint = DataPoint {
            timestamp,
            value,
            tags: sensor_tags.clone(),
        };

        db.insert("temperature_sensor_1".to_string(), datapoint).await?;

        if i % 10 == 0 {
            println!("  已插入 {} 个数据点...", i + 1);
        }
    }
    println!("✅ 共插入 61 个数据点\n");

    // 示例4: 创建湿度传感器数据
    println!("💧 创建湿度传感器数据...");
    let mut humidity_tags = BTreeMap::new();
    humidity_tags.insert("sensor_type".to_string(), "humidity".to_string());
    humidity_tags.insert("location".to_string(), "server_room_1".to_string());
    humidity_tags.insert("device_id".to_string(), "humi_001".to_string());

    for i in 0..=30 { // 每2分钟一个湿度数据点
        let timestamp = 1609459200 + i * 120;
        let value = 45.0 + (i as f64) * 0.3 + ((i % 5) as f64 - 2.0) * 0.8;

        let datapoint = DataPoint {
            timestamp,
            value,
            tags: humidity_tags.clone(),
        };

        db.insert("humidity_sensor_1".to_string(), datapoint).await?;
    }
    println!("✅ 湿度数据插入完成（31个数据点）\n");

    // 示例5: 查询所有系列
    println!("📋 查询所有数据系列...");
    let all_series = db.get_all_series().await?;
    println!("系列数量: {}", all_series.len());
    for series in &all_series {
        println!("  - {}", series);
    }
    println!();

    // 示例6: 查询温度传感器的所有数据
    println!("🔍 查询温度传感器的所有数据...");
    let temp_data = db.query_range("temperature_sensor_1", None, None).await?;
    println!("温度数据点数量: {}", temp_data.len());
    
    // 显示前5个和后5个数据点
    println!("前5个数据点:");
    for (i, dp) in temp_data.iter().take(5).enumerate() {
        println!("  {}. 时间戳: {}, 值: {:.2}°C", i + 1, dp.timestamp, dp.value);
    }
    
    if temp_data.len() > 10 {
        println!("  ...");
        println!("后5个数据点:");
        for (i, dp) in temp_data.iter().rev().take(5).enumerate() {
            let index = temp_data.len() - i;
            println!("  {}. 时间戳: {}, 值: {:.2}°C", index, dp.timestamp, dp.value);
        }
    }
    println!();

    // 示例7: 时间范围查询
    println!("⏰ 查询前30分钟的温度数据...");
    let start_time = 1609459200;
    let end_time = 1609459200 + 30 * 60; // 30分钟后
    
    let range_data = db.query_range("temperature_sensor_1", Some(start_time), Some(end_time)).await?;
    println!("前30分钟数据点数量: {}", range_data.len());
    
    if !range_data.is_empty() {
        let min_temp = range_data.iter().map(|dp| dp.value).fold(f64::INFINITY, f64::min);
        let max_temp = range_data.iter().map(|dp| dp.value).fold(f64::NEG_INFINITY, f64::max);
        let avg_temp = range_data.iter().map(|dp| dp.value).sum::<f64>() / range_data.len() as f64;
        
        println!("  最低温度: {:.2}°C", min_temp);
        println!("  最高温度: {:.2}°C", max_temp);
        println!("  平均温度: {:.2}°C", avg_temp);
    }
    println!();

    // 示例8: 数据更新操作
    println!("✏️ 更新第一个数据点的值...");
    let first_timestamp = 1609459200;
    let updated = db.update("temperature_sensor_1", first_timestamp, 25.0).await?;
    
    if updated {
        println!("✅ 数据点更新成功");
        
        // 验证更新
        let updated_data = db.query_range("temperature_sensor_1", Some(first_timestamp), Some(first_timestamp)).await?;
        if let Some(dp) = updated_data.first() {
            println!("  更新后的值: {:.1}°C", dp.value);
        }
    } else {
        println!("❌ 数据点更新失败");
    }
    println!();

    // 示例9: 数据删除操作
    println!("🗑️ 删除最后一个温度数据点...");
    let last_timestamp = 1609459200 + 60 * 60; // 1小时后的时间戳
    let deleted = db.delete("temperature_sensor_1", Some(last_timestamp)).await?;
    
    if deleted {
        println!("✅ 数据点删除成功");
        
        // 验证删除
        let remaining_data = db.query_range("temperature_sensor_1", None, None).await?;
        println!("  剩余数据点数量: {}", remaining_data.len());
    } else {
        println!("❌ 数据点删除失败（可能不存在）");
    }
    println!();

    // 示例10: 数据库统计信息
    println!("📊 获取数据库统计信息...");
    let stats = db.get_stats().await?;
    println!("数据库统计:");
    println!("  内存表大小: {}", stats.memtable_size);
    println!("  SSTable数量: {}", stats.sstable_count);
    println!("  总系列数: {}", stats.total_series);
    println!();

    // 示例11: 手动触发压缩
    println!("🔄 手动触发数据压缩...");
    db.compact().await?;
    println!("✅ 数据压缩完成\n");

    // 示例12: 压缩后的统计信息
    println!("📊 压缩后的数据库统计信息...");
    let stats_after = db.get_stats().await?;
    println!("压缩后统计:");
    println!("  内存表大小: {}", stats_after.memtable_size);
    println!("  SSTable数量: {}", stats_after.sstable_count);
    println!("  总系列数: {}", stats_after.total_series);
    println!();

    // 示例13: 批量删除操作
    println!("🗑️ 删除整个湿度传感器系列...");
    let series_deleted = db.delete("humidity_sensor_1", None).await?;
    
    if series_deleted {
        println!("✅ 湿度传感器系列删除成功");
        
        // 验证删除
        let remaining_series = db.get_all_series().await?;
        println!("  剩余系列数量: {}", remaining_series.len());
        for series in &remaining_series {
            println!("    - {}", series);
        }
    } else {
        println!("❌ 系列删除失败");
    }
    println!();

    // 示例14: 最终数据验证
    println!("🔍 最终数据验证...");
    let final_temp_data = db.query_range("temperature_sensor_1", None, None).await?;
    let final_series = db.get_all_series().await?;
    
    println!("最终状态:");
    println!("  系列数量: {}", final_series.len());
    println!("  温度数据点数量: {}", final_temp_data.len());
    
    if !final_temp_data.is_empty() {
        let min_temp = final_temp_data.iter().map(|dp| dp.value).fold(f64::INFINITY, f64::min);
        let max_temp = final_temp_data.iter().map(|dp| dp.value).fold(f64::NEG_INFINITY, f64::max);
        println!("  温度范围: {:.2}°C - {:.2}°C", min_temp, max_temp);
    }

    println!("\n🎉 基础使用示例演示完成！");
    println!("数据将在程序结束时自动清理（使用了临时目录）");

    Ok(())
}

