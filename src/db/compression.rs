use serde::{Serialize, Deserialize};

#[derive(Debug, Clone)]
pub struct GorillaBitWriter {
    buffer: Vec<u8>,
    bit_pos: usize,
}

impl GorillaBitWriter {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            bit_pos: 0,
        }
    }

    pub fn write_bits(&mut self, value: u64, num_bits: usize) {
        if num_bits == 0 || num_bits > 64 {
            return;
        }
        
        for i in (0..num_bits).rev() {
            let bit = (value >> i) & 1;
            
            // 使用 wrapping_div 和 wrapping_rem 避免溢出
            if self.bit_pos.wrapping_rem(8) == 0 {
                self.buffer.push(0);
            }
            
            let byte_index = self.bit_pos.wrapping_div(8);
            let bit_index = 7_usize.wrapping_sub(self.bit_pos.wrapping_rem(8));
            
            // 确保不越界
            if byte_index < self.buffer.len() && bit == 1 {
                self.buffer[byte_index] |= 1 << bit_index;
            }
            
            // 使用 wrapping_add 避免溢出
            self.bit_pos = self.bit_pos.wrapping_add(1);
        }
    }

    pub fn get_bytes(&self) -> &[u8] {
        &self.buffer
    }
}

#[derive(Debug, Clone)]
pub struct GorillaBitReader {
    buffer: Vec<u8>,
    bit_pos: usize,
}

impl GorillaBitReader {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            buffer: data,
            bit_pos: 0,
        }
    }

    pub fn read_bits(&mut self, num_bits: usize) -> Option<u64> {
        if num_bits == 0 || num_bits > 64 {
            return Some(0);
        }
        
        // 使用 wrapping_add 和 wrapping_mul 避免溢出
        let total_bits = self.buffer.len().wrapping_mul(8);
        if self.bit_pos.wrapping_add(num_bits) > total_bits {
            return None;
        }

        let mut result = 0u64;
        
        for _ in 0..num_bits {
            let byte_index = self.bit_pos.wrapping_div(8);
            let bit_index = 7_usize.wrapping_sub(self.bit_pos.wrapping_rem(8));
            
            if byte_index >= self.buffer.len() {
                return None;
            }
            
            let bit = (self.buffer[byte_index] >> bit_index) & 1;
            result = (result << 1) | (bit as u64);
            self.bit_pos = self.bit_pos.wrapping_add(1);
        }
        
        Some(result)
    }

    pub fn has_more_data(&self) -> bool {
        let total_bits = self.buffer.len().wrapping_mul(8);
        self.bit_pos < total_bits
    }
}

#[derive(Debug)]
pub struct GorillaCompressor {
    writer: GorillaBitWriter,
    prev_timestamp: Option<u64>,
    prev_delta: Option<i64>,
    prev_value: Option<f64>,
    count: u32, // 使用 u32 而不是 u8 避免溢出
}

impl GorillaCompressor {
    pub fn new() -> Self {
        Self {
            writer: GorillaBitWriter::new(),
            prev_timestamp: None,
            prev_delta: None,
            prev_value: None,
            count: 0,
        }
    }

    pub fn compress_datapoint(&mut self, timestamp: u64, value: f64) {
        if self.count == 0 {
            // 第一个数据点，直接存储
            self.writer.write_bits(timestamp, 64);
            self.writer.write_bits(value.to_bits(), 64);
            self.prev_timestamp = Some(timestamp);
            self.prev_value = Some(value);
            self.count = 1;
        } else {
            let prev_ts = self.prev_timestamp.unwrap();
            // 使用 wrapping_sub 避免溢出
            let delta = (timestamp as i64).wrapping_sub(prev_ts as i64);
            self.compress_timestamp(delta);
            self.compress_value(value);
            self.prev_timestamp = Some(timestamp);
            self.prev_value = Some(value);
            self.count = self.count.wrapping_add(1);
        }
    }

    fn compress_timestamp(&mut self, delta: i64) {
        match self.prev_delta {
            None => {
                // 第一个delta
                self.writer.write_bits(0b10, 2);
                // 确保delta在有效范围内
                let clamped_delta = delta.clamp(-8191, 8191);
                self.writer.write_bits(clamped_delta as u64, 14);
                self.prev_delta = Some(delta);
            }
            Some(prev_delta) => {
                // 使用 wrapping_sub 避免溢出
                let delta_of_delta = delta.wrapping_sub(prev_delta);
                
                if delta_of_delta == 0 {
                    self.writer.write_bits(0b0, 1);
                } else if (-63..=64).contains(&delta_of_delta) {
                    self.writer.write_bits(0b10, 2);
                    // 7位有符号整数编码，使用 wrapping_add
                    let encoded = if delta_of_delta < 0 {
                        (128_i64.wrapping_add(delta_of_delta)) as u64
                    } else {
                        delta_of_delta as u64
                    };
                    self.writer.write_bits(encoded, 7);
                } else {
                    self.writer.write_bits(0b11, 2);
                    // 12位有符号整数编码，使用 wrapping_add
                    let encoded = if delta_of_delta < 0 {
                        (4096_i64.wrapping_add(delta_of_delta)) as u64
                    } else {
                        delta_of_delta as u64
                    };
                    self.writer.write_bits(encoded, 12);
                }
                
                self.prev_delta = Some(delta);
            }
        }
    }

    fn compress_value(&mut self, value: f64) {
        let current_bits = value.to_bits();
        
        match self.prev_value {
            None => {
                self.writer.write_bits(current_bits, 64);
            }
            Some(prev_value) => {
                let prev_bits = prev_value.to_bits();
                let xor_result = current_bits ^ prev_bits;
                
                if xor_result == 0 {
                    self.writer.write_bits(0b0, 1);
                } else {
                    self.writer.write_bits(0b1, 1);
                    let leading_zeros = xor_result.leading_zeros() as usize;
                    let trailing_zeros = xor_result.trailing_zeros() as usize;
                    
                    // 使用 saturating_sub 避免溢出
                    let meaningful_bits = 64_usize.saturating_sub(leading_zeros).saturating_sub(trailing_zeros);
                    
                    if meaningful_bits > 0 && meaningful_bits <= 64 {
                        self.writer.write_bits(leading_zeros.min(63) as u64, 6);
                        self.writer.write_bits(meaningful_bits.min(64) as u64, 6);
                        let meaningful_value = xor_result >> trailing_zeros.min(63);
                        self.writer.write_bits(meaningful_value, meaningful_bits.min(64));
                    } else {
                        // 如果没有有意义的位，存储完整值
                        self.writer.write_bits(0, 6);
                        self.writer.write_bits(64, 6);
                        self.writer.write_bits(current_bits, 64);
                    }
                }
            }
        }
    }

    pub fn finish(mut self) -> Vec<u8> {
        // 在末尾添加结束标记
        self.writer.write_bits(0b11111111, 8);
        self.writer.get_bytes().to_vec()
    }
}

#[derive(Debug)]
pub struct GorillaDecompressor {
    reader: GorillaBitReader,
    prev_timestamp: Option<u64>,
    prev_delta: Option<i64>,
    prev_value: Option<f64>,
    finished: bool,
}

impl GorillaDecompressor {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            reader: GorillaBitReader::new(data),
            prev_timestamp: None,
            prev_delta: None,
            prev_value: None,
            finished: false,
        }
    }

    pub fn decompress_next(&mut self) -> Option<(u64, f64)> {
        if self.finished || !self.reader.has_more_data() {
            return None;
        }

        match self.prev_timestamp {
            None => {
                // 第一个数据点
                let timestamp = self.reader.read_bits(64)?;
                let value_bits = self.reader.read_bits(64)?;
                let value = f64::from_bits(value_bits);
                
                self.prev_timestamp = Some(timestamp);
                self.prev_value = Some(value);
                
                Some((timestamp, value))
            }
            Some(prev_ts) => {
                // 检查是否遇到结束标记
                if let Some(end_marker) = self.reader.read_bits(8) {
                    if end_marker == 0b11111111 {
                        self.finished = true;
                        return None;
                    }
                    // 回退8位
                    self.reader.bit_pos = self.reader.bit_pos.saturating_sub(8);
                }
                
                let timestamp = self.decompress_timestamp(prev_ts)?;
                let value = self.decompress_value()?;
                
                self.prev_timestamp = Some(timestamp);
                self.prev_value = Some(value);
                
                Some((timestamp, value))
            }
        }
    }

    fn decompress_timestamp(&mut self, prev_timestamp: u64) -> Option<u64> {
        match self.prev_delta {
            None => {
                let control_bits = self.reader.read_bits(2)?;
                if control_bits == 0b10 {
                    let delta = self.reader.read_bits(14)? as i64;
                    let signed_delta = if delta > 8191 { 
                        delta.wrapping_sub(16384) 
                    } else { 
                        delta 
                    };
                    self.prev_delta = Some(signed_delta);
                    // 使用 wrapping_add 避免溢出
                    Some((prev_timestamp as i64).wrapping_add(signed_delta) as u64)
                } else {
                    None
                }
            }
            Some(prev_delta) => {
                let control_bit = self.reader.read_bits(1)?;
                
                if control_bit == 0 {
                    // 使用 wrapping_add 避免溢出
                    Some((prev_timestamp as i64).wrapping_add(prev_delta) as u64)
                } else {
                    let second_bit = self.reader.read_bits(1)?;
                    
                    let delta_of_delta = if second_bit == 0 {
                        let value = self.reader.read_bits(7)? as i64;
                        if value > 63 { 
                            value.wrapping_sub(128) 
                        } else { 
                            value 
                        }
                    } else {
                        let value = self.reader.read_bits(12)? as i64;
                        if value > 2047 { 
                            value.wrapping_sub(4096) 
                        } else { 
                            value 
                        }
                    };
                    
                    // 使用 wrapping_add 避免溢出
                    let new_delta = prev_delta.wrapping_add(delta_of_delta);
                    self.prev_delta = Some(new_delta);
                    Some((prev_timestamp as i64).wrapping_add(new_delta) as u64)
                }
            }
        }
    }

    fn decompress_value(&mut self) -> Option<f64> {
        match self.prev_value {
            None => {
                let value_bits = self.reader.read_bits(64)?;
                let value = f64::from_bits(value_bits);
                Some(value)
            }
            Some(prev_value) => {
                let control_bit = self.reader.read_bits(1)?;
                
                if control_bit == 0 {
                    Some(prev_value)
                } else {
                    let leading_zeros = self.reader.read_bits(6)? as usize;
                    let meaningful_bits = self.reader.read_bits(6)? as usize;
                    
                    if meaningful_bits == 0 || meaningful_bits > 64 {
                        return Some(prev_value);
                    }
                    
                    let meaningful_value = self.reader.read_bits(meaningful_bits.min(64))?;
                    // 使用 saturating_sub 避免溢出
                    let trailing_zeros = 64_usize.saturating_sub(leading_zeros).saturating_sub(meaningful_bits);
                    
                    let xor_result = meaningful_value << trailing_zeros.min(63);
                    let prev_bits = prev_value.to_bits();
                    let current_bits = prev_bits ^ xor_result;
                    
                    Some(f64::from_bits(current_bits))
                }
            }
        }
    }

    pub fn decompress_all(mut self) -> Vec<(u64, f64)> {
        let mut results = Vec::new();
        
        while let Some(datapoint) = self.decompress_next() {
            results.push(datapoint);
        }
        
        results
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    pub timestamp: u64,
    pub value: f64,
    pub tags: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeriesData {
    pub series_key: String,
    pub compressed_data: Vec<u8>,
    pub tags: std::collections::BTreeMap<String, String>,
    pub min_timestamp: u64,
    pub max_timestamp: u64,
    pub count: usize,
}

// 为 GorillaBitWriter 添加 Default 实现
impl Default for GorillaBitWriter {
    fn default() -> Self {
        Self::new()
    }
}

// 为 GorillaCompressor 添加 Default 实现  
impl Default for GorillaCompressor {
    fn default() -> Self {
        Self::new()
    }
}

