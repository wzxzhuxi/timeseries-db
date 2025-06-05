use std::fs::{File, OpenOptions};
use std::io::{Result, Write};
use std::path::PathBuf;
use memmap2::Mmap;

use super::{DataPoint, GorillaDecompressor, GorillaCompressor, SeriesData};

#[derive(Debug)]
pub struct SSTable {
    file_path: PathBuf,
    mmap: Option<Mmap>,
}

impl SSTable {
    pub fn new(file_path: PathBuf) -> Result<Self> {
        Ok(Self {
            file_path,
            mmap: None,
        })
    }

    pub fn write_data(&mut self, series_data: &[SeriesData]) -> Result<()> {
        // 清除现有的内存映射
        self.mmap = None;
        
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.file_path)?;
        
        let serialized = bincode::serialize(series_data)
            // .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            .map_err(std::io::Error::other)?;
        
        file.write_all(&serialized)?;
        file.sync_all()?;
        
        // 确保文件被完全写入并关闭
        drop(file);
        
        Ok(())
    }

    pub fn read_with_mmap(&mut self) -> Result<&[u8]> {
        // 如果已有映射，先检查文件是否仍然有效
        if self.mmap.is_some() && !self.file_path.exists() {
        self.mmap = None;
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "SSTable file was deleted"
    ));
}

        
        if self.mmap.is_none() {
            // 检查文件是否存在且不为空
            let metadata = std::fs::metadata(&self.file_path)?;
            if metadata.len() == 0 {
                return Ok(&[]);
            }
            
            let file = File::open(&self.file_path)?;
            
            // 安全地创建内存映射
            let mmap = unsafe { 
                match Mmap::map(&file) {
                    Ok(mmap) => mmap,
                    Err(e) => {
                        tracing::error!("Failed to create mmap for {:?}: {}", self.file_path, e);
                        return Err(e);
                    }
                }
            };
            
            self.mmap = Some(mmap);
        }
        
        Ok(self.mmap.as_ref().unwrap())
    }

    pub fn delete_file(&self) -> Result<()> {
        // 在删除文件前清除内存映射
        if self.file_path.exists() {
            std::fs::remove_file(&self.file_path)?;
        }
        Ok(())
    }

    // 安全的删除数据点方法
    pub fn delete_datapoint(&mut self, series_key: &str, timestamp: Option<u64>) -> Result<bool> {
        // 首先释放内存映射
        self.mmap = None;
        
        // 检查文件是否存在
        if !self.file_path.exists() {
            return Ok(false);
        }
        
        let data = std::fs::read(&self.file_path)?;
        if data.is_empty() {
            return Ok(false);
        }
        
        let mut series_list: Vec<SeriesData> = bincode::deserialize(&data)
            // .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            .map_err(std::io::Error::other)?;

        let mut deleted = false;

        match timestamp {
            Some(ts) => {
                for series in series_list.iter_mut() {
                    if series.series_key == series_key {
                        let decompressor = GorillaDecompressor::new(series.compressed_data.clone());
                        let mut decompressed_points = decompressor.decompress_all();
                        let original_len = decompressed_points.len();

                        decompressed_points.retain(|(timestamp, _)| *timestamp != ts);

                        if decompressed_points.len() < original_len {
                            deleted = true;
                            
                            if decompressed_points.is_empty() {
                                series_list.retain(|s| s.series_key != series_key);
                            } else {
                                let mut compressor = GorillaCompressor::new();
                                for (timestamp, value) in decompressed_points {
                                    compressor.compress_datapoint(timestamp, value);
                                }
                                series.compressed_data = compressor.finish();
                                series.count -= 1;
                            }
                            break;
                        }
                    }
                }
            }
            None => {
                let original_len = series_list.len();
                series_list.retain(|s| s.series_key != series_key);
                deleted = series_list.len() < original_len;
            }
        }

        if deleted {
            if series_list.is_empty() {
                // 安全地删除空文件
                self.delete_file()?;
            } else {
                // 重写文件
                self.write_data(&series_list)?;
            }
        }

        Ok(deleted)
    }

    // 其他方法保持不变，但添加错误处理...
    pub fn query_series(&mut self, series_key: &str, start_time: Option<u64>, end_time: Option<u64>) -> Result<Vec<DataPoint>> {
        let data = match self.read_with_mmap() {
            Ok(data) => data,
            Err(e) => {
                tracing::warn!("Failed to read SSTable {:?}: {}", self.file_path, e);
                return Ok(Vec::new());
            }
        };
        
        if data.is_empty() {
            return Ok(Vec::new());
        }
        
        let series_list: Vec<SeriesData> = match bincode::deserialize(data) {
            Ok(list) => list,
            Err(e) => {
                tracing::error!("Failed to deserialize SSTable data: {}", e);
                return Ok(Vec::new());
            }
        };

        let mut results = Vec::new();

        for series in series_list {
            if series.series_key == series_key {
                if let Some(start) = start_time {
                    if series.max_timestamp < start {
                        continue;
                    }
                }
                if let Some(end) = end_time {
                    if series.min_timestamp > end {
                        continue;
                    }
                }

                let decompressor = GorillaDecompressor::new(series.compressed_data);
                let decompressed_points = decompressor.decompress_all();

                for (timestamp, value) in decompressed_points {
                    if let Some(start) = start_time {
                        if timestamp < start {
                            continue;
                        }
                    }
                    if let Some(end) = end_time {
                        if timestamp > end {
                            continue;
                        }
                    }

                    results.push(DataPoint {
                        timestamp,
                        value,
                        tags: series.tags.clone(),
                    });
                }
            }
        }

        Ok(results)
    }

    // 安全的系列键获取方法
    pub fn get_all_series_keys(&mut self) -> Result<Vec<String>> {
        let data = match self.read_with_mmap() {
            Ok(data) => data,
            Err(_) => return Ok(Vec::new()),
        };
        
        if data.is_empty() {
            return Ok(Vec::new());
        }
        
        match bincode::deserialize::<Vec<SeriesData>>(data) {
            Ok(series_list) => {
                Ok(series_list.into_iter().map(|s| s.series_key).collect())
            }
            Err(e) => {
                tracing::warn!("Failed to deserialize series keys: {}", e);
                Ok(Vec::new())
            }
        }
    }

    pub fn update_datapoint(&mut self, series_key: &str, timestamp: u64, new_value: f64) -> Result<bool> {
        // 释放内存映射
        self.mmap = None;
        
        let data = std::fs::read(&self.file_path)?;
        let mut series_list: Vec<SeriesData> = bincode::deserialize(&data)
            // .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            .map_err(std::io::Error::other)?;

        let mut updated = false;

        for series in series_list.iter_mut() {
            if series.series_key == series_key {
                let decompressor = GorillaDecompressor::new(series.compressed_data.clone());
                let mut decompressed_points = decompressor.decompress_all();

                for (ts, value) in decompressed_points.iter_mut() {
                    if *ts == timestamp {
                        *value = new_value;
                        updated = true;
                        break;
                    }
                }

                if updated {
                    let mut compressor = GorillaCompressor::new();
                    for (ts, val) in decompressed_points {
                        compressor.compress_datapoint(ts, val);
                    }
                    series.compressed_data = compressor.finish();
                    break;
                }
            }
        }

        if updated {
            self.write_data(&series_list)?;
        }

        Ok(updated)
    }
}

// 确保Drop时清理资源
impl Drop for SSTable {
    fn drop(&mut self) {
        self.mmap = None;
    }
}

