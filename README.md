###### 愿你的生命如花朵般盛开

<div align="center">
<h2>Ry TimeSeries Database</h2>
<br>
<strong>高性能时序数据库，基于LSM-Tree存储引擎</strong>
<br><br>
一个现代化的时序数据库解决方案，集成了Gorilla压缩、mmap零拷贝技术和异步HTTP API
<br><br>
<a href="#-快速开始">快速开始</a> -  
<a href="#-api文档">API文档</a> -  
<a href="#-性能测试">性能测试</a> -  
<a href="#-docker部署">Docker部署</a>
</div>

---

## ✨ 核心特性

- **🌲 LSM-Tree存储引擎** - 高效的写入性能和数据组织
- **🗜️ Gorilla压缩算法** - 专为时序数据设计，压缩率80-90%
- **⚡ mmap零拷贝技术** - 内存映射文件，减少数据拷贝开销
- **📡 异步HTTP API** - 基于Tokio和Axum的高并发架构
- **🔄 自动Compaction** - 定期数据压缩，保持存储效率

---

## 🚀 快速开始

### 环境要求

- Rust 1.70+
- 内存: 最少512MB，推荐2GB+
- 磁盘: 根据数据量确定，建议使用SSD


### 启动服务

```bash
# 克隆项目并进入目录
cd timeseries-db

# 启动服务（开发模式）
cargo run

# 启动服务（发布模式，更高性能）
cargo run --release
```


### 自定义配置启动

```bash
PORT=6364 \
DATA_DIR=./data \
MEMTABLE_THRESHOLD=5000 \
RUST_LOG=info \
cargo run --release
```

**环境变量说明：**


| 变量名 | 默认值 | 说明 |
| :-- | :-- | :-- |
| `PORT` | 6364 | HTTP服务端口 |
| `DATA_DIR` | ./tsdb_data | 数据存储目录 |
| `MEMTABLE_THRESHOLD` | 1000 | 内存表大小阈值 |
| `RUST_LOG` | info | 日志级别 |


---

## 📚 API文档

### 基础URL

```
http://localhost:6364
```


### 健康检查与统计

#### 健康检查

```bash
GET /health

curl http://localhost:6364/health
```


#### 数据库统计

```bash
GET /stats

curl http://localhost:6364/stats
```


### 数据点操作

#### 创建单个数据点

```bash
POST /api/v1/datapoints

curl -X POST http://localhost:6364/api/v1/datapoints \
  -H "Content-Type: application/json" \
  -d '{
    "series_key": "temperature_sensor_1",
    "timestamp": 1609459200,
    "value": 23.5,
    "tags": {
      "location": "room1",
      "sensor_type": "temperature"
    }
  }'
```


#### 批量创建数据点

```bash
POST /api/v1/datapoints/batch

curl -X POST http://localhost:6364/api/v1/datapoints/batch \
  -H "Content-Type: application/json" \
  -d '[
    {
      "series_key": "temperature_sensor_1",
      "timestamp": 1609459260,
      "value": 23.6,
      "tags": {"location": "room1"}
    },
    {
      "series_key": "humidity_sensor_1",
      "timestamp": 1609459200,
      "value": 65.2,
      "tags": {"location": "room1", "sensor_type": "humidity"}
    }
  ]'
```


#### 查询数据点

```bash
GET /api/v1/series/{series_key}/datapoints

# 查询所有数据点
curl "http://localhost:6364/api/v1/series/temperature_sensor_1/datapoints"

# 按时间范围查询
curl "http://localhost:6364/api/v1/series/temperature_sensor_1/datapoints?start_time=1609459200&end_time=1609459800"

# 带限制的查询
curl "http://localhost:6364/api/v1/series/temperature_sensor_1/datapoints?limit=100"
```

**查询参数：**

- `start_time`: 开始时间戳（可选）
- `end_time`: 结束时间戳（可选）
- `limit`: 返回数据点数量限制（可选）


#### 更新数据点

```bash
PUT /api/v1/series/{series_key}/datapoints/{timestamp}

curl -X PUT http://localhost:6364/api/v1/series/temperature_sensor_1/datapoints/1609459200 \
  -H "Content-Type: application/json" \
  -d '{"value": 25.0}'
```


#### 删除数据点

```bash
DELETE /api/v1/series/{series_key}/datapoints/{timestamp}

curl -X DELETE http://localhost:6364/api/v1/series/temperature_sensor_1/datapoints/1609459200
```


### 系列管理

#### 获取所有系列

```bash
GET /api/v1/series

curl http://localhost:6364/api/v1/series
```


#### 获取系列详细信息

```bash
GET /api/v1/series/{series_key}

curl http://localhost:6364/api/v1/series/temperature_sensor_1
```


#### 删除整个系列

```bash
DELETE /api/v1/series/{series_key}

curl -X DELETE http://localhost:6364/api/v1/series/temperature_sensor_1
```


### 数据库管理

#### 手动触发压缩

```bash
POST /api/v1/admin/compact

curl -X POST http://localhost:6364/api/v1/admin/compact \
  -H "Content-Type: application/json" \
  -d '{"force": true}'
```


---

## 🧪 性能测试

### 基础性能测试脚本

```bash
#!/bin/bash

echo "开始性能测试..."

# 批量插入测试
start_time=$(date +%s)

for i in {1..1000}; do
    timestamp=$((1609459200 + i * 60))
    value=$(echo "scale=2; 20 + ($i % 100) * 0.1" | bc)
    
    curl -s -X POST http://localhost:6364/api/v1/datapoints \
      -H "Content-Type: application/json" \
      -d "{
        \"series_key\": \"perf_test_$((i % 10))\",
        \"timestamp\": $timestamp,
        \"value\": $value,
        \"tags\": {\"test\": \"performance\"}
      }" > /dev/null
    
    if [ $((i % 100)) -eq 0 ]; then
        echo "已插入 $i 个数据点..."
    fi
done

end_time=$(date +%s)
duration=$((end_time - start_time))

echo "插入1000个数据点耗时: ${duration}秒"
echo "平均TPS: $(echo "scale=2; 1000 / $duration" | bc)"
```


### 性能指标

| 指标 | 数值 | 说明 |
| :-- | :-- | :-- |
| 写入TPS | 1,000+ | 内存表模式下的写入性能 |
| 查询延迟 | < 10ms | 毫秒级查询响应时间 |
| 压缩率 | 80-90% | 典型时序数据压缩效果 |
| 内存使用 | < 100MB | 基准负载下内存消耗 |
| 自动压缩 | 5分钟 | 自动compaction间隔 |


---

## 🐳 Docker部署

### Dockerfile

```dockerfile
FROM rust:1.70 as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/timeseries-db /usr/local/bin/timeseries-db

ENV PORT=6364
ENV DATA_DIR=/app/data
ENV RUST_LOG=info

EXPOSE 6364
VOLUME ["/app/data"]

HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
  CMD curl -f http://localhost:6364/health || exit 1

CMD ["timeseries-db"]
```


### 容器运行

```bash
# 构建镜像
docker build -t timeseries-db .

# 运行容器
docker run -d \
  --name tsdb \
  -p 6364:6364 \
  -v $(pwd)/data:/app/data \
  -e MEMTABLE_THRESHOLD=5000 \
  timeseries-db

# 查看日志
docker logs -f tsdb

# 健康检查
docker exec tsdb curl http://localhost:6364/health
```


---

## 🛠️ 开发指南

### 项目结构

```
timeseries-db/
├── Cargo.toml
├── README.md
├── src/
│   ├── main.rs              # 主程序入口
│   ├── lib.rs               # 库文件
│   ├── db/                  # 数据库核心
│   │   ├── mod.rs           # 模块定义
│   │   ├── engine.rs        # 存储引擎
│   │   ├── compression.rs   # 压缩算法
│   │   ├── sstable.rs       # SSTable管理
│   │   └── memtable.rs      # 内存表
│   └── api/                 # HTTP API
│       ├── mod.rs           # 模块定义
│       ├── handlers.rs      # 请求处理
│       └── models.rs        # 数据模型
├── docker/
│   ├── Dockerfile
│   └── docker-compose.yml
└── scripts/
    ├── benchmark.sh
    └── test.sh
```


### 本地开发

```bash
# 克隆项目
git clone https://github.com/yourusername/timeseries-db.git
cd timeseries-db

# 安装依赖
cargo build

# 运行测试
cargo test

# 启动开发服务
RUST_LOG=debug cargo run
```


---

## 📊 监控与运维

### 实时监控

```bash
# 持续监控数据库状态
watch -n 5 'curl -s http://localhost:6364/stats | jq .'

# 监控日志
tail -f logs/timeseries-db.log
```


### 数据备份

```bash
# 备份数据目录
tar -czf backup_$(date +%Y%m%d_%H%M%S).tar.gz ./tsdb_data

# 恢复数据
tar -xzf backup_20250603_174044.tar.gz
```


---

## 🚨 故障排除

### 常见问题

**服务启动失败**

```bash
# 检查端口占用
netstat -tulpn | grep 6364

# 检查数据目录权限
ls -la ./tsdb_data

# 查看详细错误日志
RUST_LOG=debug cargo run
```

**查询性能下降**

```bash
# 检查SSTable数量
curl http://localhost:6364/stats | jq '.data.sstable_count'

# 手动触发压缩
curl -X POST http://localhost:6364/api/v1/admin/compact \
  -d '{"force": true}'
```

**内存使用过高**

```bash
# 降低内存表阈值
MEMTABLE_THRESHOLD=500 cargo run --release

# 监控内存使用
ps aux | grep timeseries-db
```


---

## 🤝 贡献指南

1. Fork 项目
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add some amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 打开 Pull Request

## 📄 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

---

<div align="center">
<strong>如果这个项目对你有帮助，请给我们一个⭐！</strong>
<br><br>
Made with ❤️ by 老王
</div>
