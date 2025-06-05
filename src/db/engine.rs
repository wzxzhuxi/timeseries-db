use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use std::io::Result;

use super::{DataPoint, Memtable, SSTable, GorillaCompressor, SeriesData};

#[derive(Debug)]
pub struct TimeSeriesDB {
    memtable: Arc<RwLock<Memtable>>,
    sstables: Arc<Mutex<Vec<SSTable>>>,
    data_dir: PathBuf,
    memtable_threshold: usize,
}

impl TimeSeriesDB {
    pub fn new<P: AsRef<Path>>(data_dir: P, memtable_threshold: usize) -> Result<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&data_dir)?;

        let mut sstables = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&data_dir) {
            // for entry in entries {
            //     if let Ok(entry) = entry {
            //         let path = entry.path();
            //         if path.extension().and_then(|s| s.to_str()) == Some("data") {
            //             sstables.push(SSTable::new(path)?);
            //         }
            //     }
            // }
            for entry in entries.flatten() {
    let path = entry.path();
    if path.extension().and_then(|s| s.to_str()) == Some("data") {
        sstables.push(SSTable::new(path)?);
    }
}
        }

        Ok(Self {
            memtable: Arc::new(RwLock::new(Memtable::new(memtable_threshold))),
            sstables: Arc::new(Mutex::new(sstables)),
            data_dir,
            memtable_threshold,
        })
    }

    pub async fn insert(&self, series_key: String, datapoint: DataPoint) -> Result<()> {
        // 检查是否需要flush，在锁外进行
        let should_flush = {
            let mut memtable = self.memtable.write().unwrap();
            memtable.insert(series_key, datapoint);
            memtable.is_full()
        };

        if should_flush {
            self.flush_memtable().await?;
        }

        Ok(()) // 修复：添加 () 参数
    }

    pub async fn update(&self, series_key: &str, timestamp: u64, new_value: f64) -> Result<bool> {
        // 首先尝试在内存表中更新
        let updated_in_memtable = {
            let mut memtable = self.memtable.write().unwrap();
            memtable.update(series_key, timestamp, new_value)
        };

        if updated_in_memtable {
            return Ok(true);
        }

        // 在SSTable中查找并更新，避免跨await持有锁
        let mut sstables = self.sstables.lock().unwrap();
        for sstable in sstables.iter_mut() {
            if sstable.update_datapoint(series_key, timestamp, new_value)? {
                return Ok(true);
            }
        }
        drop(sstables);

        Ok(false)
    }

    pub async fn delete(&self, series_key: &str, timestamp: Option<u64>) -> Result<bool> {
        // 首先尝试在内存表中删除
        let deleted_from_memtable = {
            let mut memtable = self.memtable.write().unwrap();
            memtable.delete(series_key, timestamp)
        };

        // 在SSTable中删除
        let mut deleted_from_sstable = false;
        {
            let mut sstables = self.sstables.lock().unwrap();
            
            for sstable in sstables.iter_mut() {
                if sstable.delete_datapoint(series_key, timestamp)? {
                    deleted_from_sstable = true;
                }
            }

            // 清理空的SSTable文件
            let mut indices_to_remove = Vec::new();
            for (index, sstable) in sstables.iter_mut().enumerate() {
                match sstable.get_all_series_keys() {
                    Ok(keys) => {
                        if keys.is_empty() {
                            indices_to_remove.push(index);
                        }
                    }
                    Err(_) => {
                        // 保留无法读取的文件
                    }
                }
            }
            
            // 从后往前删除，避免索引错乱
            for &index in indices_to_remove.iter().rev() {
                sstables.remove(index);
            }
        }

        Ok(deleted_from_memtable || deleted_from_sstable)
    }

    pub async fn query_range(&self, series_key: &str, start_time: Option<u64>, end_time: Option<u64>) -> Result<Vec<DataPoint>> {
    let mut results = Vec::new();

    // 查询内存表
    {
        let memtable = self.memtable.read().unwrap();
        let memtable_results = memtable.query(series_key, start_time, end_time);
        println!("🔍 内存表查询: {} 个数据点", memtable_results.len());
        results.extend(memtable_results);
    }

    // 查询SSTable文件
    {
        let mut sstables = self.sstables.lock().unwrap();
        println!("🗄️ 检查 {} 个SSTable文件", sstables.len());
        
        for (i, sstable) in sstables.iter_mut().enumerate() {
            match sstable.query_series(series_key, start_time, end_time) {
                Ok(sstable_results) => {
                    println!("  SSTable {}: {} 个数据点", i, sstable_results.len());
                    results.extend(sstable_results);
                }
                Err(e) => {
                    println!("  SSTable {} 查询失败: {}", i, e);
                    continue;
                }
            }
        }
    }

    // 按时间戳排序并去重
    results.sort_by_key(|dp| dp.timestamp);
    let before_dedup = results.len();
    results.dedup_by_key(|dp| dp.timestamp);
    
    if before_dedup != results.len() {
        println!("🔄 去重: {} -> {} 个数据点", before_dedup, results.len());
    }
    
    println!("📊 最终查询结果: {} 个数据点", results.len());
    Ok(results)
    }


    pub async fn get_all_series(&self) -> Result<Vec<String>> {
        let mut series_keys = std::collections::HashSet::new();

        // 获取内存表中的系列
        {
            let memtable = self.memtable.read().unwrap();
            for key in memtable.get_data().keys() {
                series_keys.insert(key.clone());
            }
        }

        // 获取SSTable中的系列
        {
            let mut sstables = self.sstables.lock().unwrap();
            for sstable in sstables.iter_mut() {
                match sstable.get_all_series_keys() {
                    Ok(keys) => {
                        for key in keys {
                            series_keys.insert(key);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("读取SSTable系列键失败: {}", e);
                    }
                }
            }
        }

        Ok(series_keys.into_iter().collect())
    }

    async fn flush_memtable(&self) -> Result<()> {
        // 获取数据并清空内存表，确保锁不跨越await
        let data = {
            let mut memtable = self.memtable.write().unwrap();
            let data = memtable.get_data().clone();
            memtable.clear();
            data
        };

        if data.is_empty() {
            return Ok(()); // 修复：这里就是第196行，需要添加 () 参数
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let sstable_path = self.data_dir.join(format!("sstable_{}.data", timestamp));
        let mut sstable = SSTable::new(sstable_path)?;

        let mut series_data_list = Vec::new();
        
        for (series_key, datapoints) in data {
            if datapoints.is_empty() {
                continue;
            }

            let mut compressor = GorillaCompressor::new();
            let mut min_timestamp = u64::MAX;
            let mut max_timestamp = 0u64;
            let mut tags = BTreeMap::new();

            for datapoint in &datapoints {
                compressor.compress_datapoint(datapoint.timestamp, datapoint.value);
                min_timestamp = min_timestamp.min(datapoint.timestamp);
                max_timestamp = max_timestamp.max(datapoint.timestamp);
                
                if tags.is_empty() {
                    tags = datapoint.tags.clone();
                }
            }
            
            let compressed_data = compressor.finish();
            
            let series_data = SeriesData {
                series_key,
                compressed_data,
                tags,
                min_timestamp,
                max_timestamp,
                count: datapoints.len(),
            };

            series_data_list.push(series_data);
        }

        sstable.write_data(&series_data_list)?;
        
        // 添加新的SSTable，锁的作用域很小
        {
            let mut sstables = self.sstables.lock().unwrap();
            sstables.push(sstable);
        }

        tracing::info!("内存表已刷新到SSTable，包含 {} 个系列", series_data_list.len());

        Ok(()) // 修复：添加 () 参数
    }

    pub async fn compact(&self) -> Result<()> {
        tracing::info!("开始执行compaction操作");
        
        // 获取所有SSTable数据，避免长时间持有锁
        let all_series_data = {
            let mut sstables = self.sstables.lock().unwrap();
            
            if sstables.len() < 2 {
                tracing::info!("SSTable数量不足，跳过compaction");
                return Ok(()); // 修复：添加 () 参数
            }

            let mut all_series_data = BTreeMap::new();

            // 读取所有SSTable中的数据
            for sstable in sstables.iter_mut() {
                match sstable.get_all_series_keys() {
                    Ok(series_keys) => {
                        for series_key in series_keys {
                            match sstable.query_series(&series_key, None, None) {
                                Ok(datapoints) => {
                                    let entry = all_series_data.entry(series_key).or_insert_with(Vec::new);
                                    entry.extend(datapoints);
                                }
                                Err(e) => {
                                    tracing::warn!("读取系列数据失败: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("读取SSTable失败: {}", e);
                    }
                }
            }

            // 删除旧的SSTable文件
            for sstable in sstables.iter() {
                if let Err(e) = sstable.delete_file() {
                    tracing::warn!("删除旧SSTable文件失败: {}", e);
                }
            }
            sstables.clear();

            all_series_data
        };

        // 创建新的compacted SSTable
        if !all_series_data.is_empty() {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            
            let sstable_path = self.data_dir.join(format!("compacted_{}.data", timestamp));
            let mut new_sstable = SSTable::new(sstable_path)?;

            let mut series_data_list = Vec::new();

            for (series_key, mut datapoints) in all_series_data {
                // 按时间戳排序并去重
                datapoints.sort_by_key(|dp| dp.timestamp);
                datapoints.dedup_by_key(|dp| dp.timestamp);

                if datapoints.is_empty() {
                    continue;
                }

                let mut compressor = GorillaCompressor::new();
                let mut min_timestamp = u64::MAX;
                let mut max_timestamp = 0u64;
                let mut tags = BTreeMap::new();

                for datapoint in &datapoints {
                    compressor.compress_datapoint(datapoint.timestamp, datapoint.value);
                    min_timestamp = min_timestamp.min(datapoint.timestamp);
                    max_timestamp = max_timestamp.max(datapoint.timestamp);
                    
                    if tags.is_empty() {
                        tags = datapoint.tags.clone();
                    }
                }

                let compressed_data = compressor.finish();
                
                let series_data = SeriesData {
                    series_key,
                    compressed_data,
                    tags,
                    min_timestamp,
                    max_timestamp,
                    count: datapoints.len(),
                };

                series_data_list.push(series_data);
            }

            new_sstable.write_data(&series_data_list)?;
            
            {
                let mut sstables = self.sstables.lock().unwrap();
                sstables.push(new_sstable);
            }

            tracing::info!("Compaction完成，合并了 {} 个系列", series_data_list.len());
        }

        Ok(()) // 修复：添加 () 参数
    }

    pub async fn get_stats(&self) -> Result<DatabaseStats> {
        let memtable_size = {
            let memtable = self.memtable.read().unwrap();
            memtable.get_data().len()
        };

        let sstable_count = {
            let sstables = self.sstables.lock().unwrap();
            sstables.len()
        };

        let all_series = self.get_all_series().await?;
        let total_series = all_series.len();

        Ok(DatabaseStats {
            memtable_size,
            sstable_count,
            total_series,
        })
    }
}

#[derive(Debug, serde::Serialize)]
pub struct DatabaseStats {
    pub memtable_size: usize,
    pub sstable_count: usize,
    pub total_series: usize,
}

impl Clone for TimeSeriesDB {
    fn clone(&self) -> Self {
        Self {
            memtable: Arc::clone(&self.memtable),
            sstables: Arc::clone(&self.sstables),
            data_dir: self.data_dir.clone(),
            memtable_threshold: self.memtable_threshold,
        }
    }
}

