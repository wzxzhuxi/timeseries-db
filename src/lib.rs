//! 时序数据库库

pub mod db;
pub mod api;

pub use db::*;
pub use api::*;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::collections::BTreeMap;

    #[tokio::test]
    async fn test_full_crud_operations() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let db = TimeSeriesDB::new(temp_dir.path(), 1000)?; // 增大阈值

        let mut tags = BTreeMap::new();
        tags.insert("sensor".to_string(), "temperature".to_string());
        tags.insert("location".to_string(), "room1".to_string());

        // 测试插入
        println!("🔧 插入10个数据点...");
        for i in 0..10 {
            let dp = DataPoint {
                timestamp: 1609459200 + i * 60,
                value: 20.0 + (i as f64) * 0.5,
                tags: tags.clone(),
            };
            db.insert("temp_sensor_1".to_string(), dp).await?;
        }

        // 验证插入 - 查询所有数据
        let all_results = db.query_range("temp_sensor_1", None, None).await?;
        println!("📊 查询到 {} 个数据点", all_results.len());
        assert_eq!(all_results.len(), 10, "应该有10个数据点");

        // 测试更新
        println!("✏️ 测试更新操作...");
        let updated = db.update("temp_sensor_1", 1609459200, 25.0).await?;
        println!("更新结果: {}", updated);

        // 验证更新 - 查询所有数据并检查第一个点的值
        let updated_results = db.query_range("temp_sensor_1", None, None).await?;
        let first_point = updated_results.iter().find(|dp| dp.timestamp == 1609459200);
        assert!(first_point.is_some(), "应该找到时间戳为1609459200的数据点");
        if let Some(point) = first_point {
            assert_eq!(point.value, 25.0, "更新后的值应该是25.0");
        }

        // 测试删除
        println!("🗑️ 测试删除操作...");
        let deleted = db.delete("temp_sensor_1", Some(1609459200)).await?;
        println!("删除结果: {}", deleted);

        // 验证删除
        let after_delete = db.query_range("temp_sensor_1", None, None).await?;
        let deleted_point = after_delete.iter().find(|dp| dp.timestamp == 1609459200);
        assert!(deleted_point.is_none(), "删除的数据点不应该存在");
        assert_eq!(after_delete.len(), 9, "删除后应该剩余9个数据点");

        // 测试系列列表
        let series = db.get_all_series().await?;
        assert!(series.contains(&"temp_sensor_1".to_string()));

        // 测试删除整个系列
        let series_deleted = db.delete("temp_sensor_1", None).await?;
        assert!(series_deleted);

        Ok(())
    }

    #[test]
    fn test_gorilla_compression() {
        let mut compressor = GorillaCompressor::new();
        
        compressor.compress_datapoint(1609459200, 23.5);
        compressor.compress_datapoint(1609459260, 23.6);
        compressor.compress_datapoint(1609459320, 23.4);
        compressor.compress_datapoint(1609459380, 23.5);
        
        let compressed = compressor.finish();
        println!("原始数据大小: 32 bytes (4个数据点 * 8字节)");
        println!("压缩后大小: {} bytes", compressed.len());
        
        let decompressor = GorillaDecompressor::new(compressed);
        let decompressed = decompressor.decompress_all();
        
        assert_eq!(decompressed.len(), 4);
        assert_eq!(decompressed[0], (1609459200, 23.5));
        assert_eq!(decompressed[1], (1609459260, 23.6));
        assert_eq!(decompressed[2], (1609459320, 23.4));
        assert_eq!(decompressed[3], (1609459380, 23.5));
    }

    #[tokio::test]
    async fn test_compaction() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let db = TimeSeriesDB::new(temp_dir.path(), 15)?; // 稍大一些的阈值

        println!("🔧 插入20个数据点...");
        
        // 插入20个数据点
        for i in 0..20 {
            let dp = DataPoint {
                timestamp: 1609459200 + i * 60,
                value: 20.0 + (i as f64),
                tags: BTreeMap::new(),
            };
            db.insert("test_series".to_string(), dp).await?;
        }

        // 确保数据插入成功
        println!("📊 验证插入...");
        let before_compact = db.query_range("test_series", None, None).await?;
        println!("插入验证: {} 个数据点", before_compact.len());
        assert_eq!(before_compact.len(), 20, "插入后应该有20个数据点");

        // 手动触发compaction
        println!("🔄 执行compaction...");
        db.compact().await?;

        // 验证compaction后数据完整性
        println!("🔍 验证compaction结果...");
        let results = db.query_range("test_series", None, None).await?;
        println!("Compaction后: {} 个数据点", results.len());
        
        // 显示具体数据点用于调试
        for (i, dp) in results.iter().enumerate() {
            println!("  数据点{}: timestamp={}, value={}", i+1, dp.timestamp, dp.value);
        }

        assert_eq!(results.len(), 20, "Compaction后应该保留所有20个数据点");

        Ok(())
    }
}

