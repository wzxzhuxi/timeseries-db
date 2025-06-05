# API接口文档

## 概述

时序数据库提供完整的RESTful API，支持时序数据的增删改查操作。所有接口均使用JSON格式进行数据交换。

### 基础信息

- **Base URL**: `http://localhost:6364`
- **Content-Type**: `application/json`
- **字符编码**: UTF-8
- **API版本**: v1

## API接口列表

### 健康检查与统计

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | `/health` | 服务健康检查 |
| GET | `/stats` | 数据库统计信息 |

### 数据点操作

| 方法 | 路径 | 描述 |
|------|------|------|
| POST | `/api/v1/datapoints` | 创建单个数据点 |
| POST | `/api/v1/datapoints/batch` | 批量创建数据点 |
| GET | `/api/v1/series/{series_key}/datapoints` | 查询数据点 |
| PUT | `/api/v1/series/{series_key}/datapoints/{timestamp}` | 更新数据点 |
| DELETE | `/api/v1/series/{series_key}/datapoints/{timestamp}` | 删除数据点 |

### 系列管理

| 方法 | 路径 | 描述 |
|------|------|------|
| GET | `/api/v1/series` | 获取所有系列 |
| GET | `/api/v1/series/{series_key}` | 获取系列详细信息 |
| DELETE | `/api/v1/series/{series_key}` | 删除整个系列 |

### 数据库管理

| 方法 | 路径 | 描述 |
|------|------|------|
| POST | `/api/v1/admin/compact` | 手动触发压缩 |

## 详细接口说明

### 健康检查

**接口**: `GET /health`

**描述**: 检查服务运行状态

**请求参数**: 无

**响应示例**:

```

{
"status": "healthy",
"service": "时序数据库",
"version": "1.0.0",
"timestamp": 1609459200,
"features": [
"LSM-Tree存储引擎",
"Gorilla压缩算法",
"mmap零拷贝技术",
"异步HTTP API"
]
}

```

### 数据库统计

**接口**: `GET /stats`

**描述**: 获取数据库运行统计信息

**请求参数**: 无

**响应示例**:
```

{
"success": true,
"message": "操作成功",
"data": {
"storage_engine": "LSM-Tree",
"compression": "Gorilla",
"memory_mapping": "mmap零拷贝",
"status": "运行中",
"memtable_size": 245,
"sstable_count": 3,
"total_series": 15,
"timestamp": 1609459200
},
"timestamp": 1609459200
}

```

### 创建数据点

**接口**: `POST /api/v1/datapoints`

**描述**: 创建单个数据点

**请求体**:
```

{
"series_key": "temperature_sensor_1",
"timestamp": 1609459200,
"value": 23.5,
"tags": {
"location": "room1",
"sensor_type": "temperature"
}
}

```

**字段说明**:
- `series_key` (string, 必需): 时间序列唯一标识
- `timestamp` (integer, 必需): Unix时间戳（秒）
- `value` (number, 必需): 数值
- `tags` (object, 可选): 标签键值对

**响应示例**:
```

{
"success": true,
"message": "数据点已添加到系列: temperature_sensor_1 (时间戳: 1609459200)",
"data": null,
"timestamp": 1609459200
}

```

### 批量创建数据点

**接口**: `POST /api/v1/datapoints/batch`

**描述**: 批量创建多个数据点

**请求体**:
```

[
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
]

```

**响应示例**:
```

{
"success": true,
"message": "批量创建完成: 成功 2 个，失败 0 个",
"data": null,
"timestamp": 1609459200
}

```

### 查询数据点

**接口**: `GET /api/v1/series/{series_key}/datapoints`

**描述**: 查询指定系列的数据点

**路径参数**:
- `series_key` (string): 时间序列标识

**查询参数**:
- `start_time` (integer, 可选): 开始时间戳
- `end_time` (integer, 可选): 结束时间戳
- `limit` (integer, 可选): 返回数据点数量限制

**响应示例**:
```

{
"success": true,
"message": "操作成功",
"data": [
{
"timestamp": 1609459200,
"value": 23.5,
"tags": {
"location": "room1",
"sensor_type": "temperature"
}
},
{
"timestamp": 1609459260,
"value": 23.6,
"tags": {
"location": "room1"
}
}
],
"timestamp": 1609459200
}

```

### 更新数据点

**接口**: `PUT /api/v1/series/{series_key}/datapoints/{timestamp}`

**描述**: 更新指定时间戳的数据点值

**路径参数**:
- `series_key` (string): 时间序列标识
- `timestamp` (integer): 时间戳

**请求体**:
```

{
"value": 25.0
}

```

**响应示例**:
```

{
"success": true,
"message": "数据点已更新: temperature_sensor_1 at 1609459200 -> 25.0",
"data": null,
"timestamp": 1609459200
}

```

### 删除数据点

**接口**: `DELETE /api/v1/series/{series_key}/datapoints/{timestamp}`

**描述**: 删除指定时间戳的数据点

**路径参数**:
- `series_key` (string): 时间序列标识
- `timestamp` (integer): 时间戳

**响应示例**:
```

{
"success": true,
"message": "数据点已删除: temperature_sensor_1 at 1609459200",
"data": null,
"timestamp": 1609459200
}

```

### 获取所有系列

**接口**: `GET /api/v1/series`

**描述**: 获取所有时间序列列表

**响应示例**:
```

{
"success": true,
"message": "操作成功",
"data": {
"series": [
"temperature_sensor_1",
"humidity_sensor_1",
"pressure_sensor_1"
],
"count": 3
},
"timestamp": 1609459200
}

```

### 获取系列详细信息

**接口**: `GET /api/v1/series/{series_key}`

**描述**: 获取指定系列的详细统计信息

**路径参数**:
- `series_key` (string): 时间序列标识

**响应示例**:
```

{
"success": true,
"message": "操作成功",
"data": {
"series_key": "temperature_sensor_1",
"count": 1440,
"min_timestamp": 1609459200,
"max_timestamp": 1609545600,
"min_value": 18.5,
"max_value": 28.9,
"tags": {
"location": "room1",
"sensor_type": "temperature"
}
},
"timestamp": 1609459200
}

```

### 删除系列

**接口**: `DELETE /api/v1/series/{series_key}`

**描述**: 删除整个时间序列及其所有数据点

**路径参数**:
- `series_key` (string): 时间序列标识

**响应示例**:
```

{
"success": true,
"message": "系列已删除: temperature_sensor_1",
"data": null,
"timestamp": 1609459200
}

```

### 手动触发压缩

**接口**: `POST /api/v1/admin/compact`

**描述**: 手动触发数据库压缩操作

**请求体**:
```

{
"force": true
}

```

**响应示例**:
```

{
"success": true,
"message": "Compaction执行完成",
"data": null,
"timestamp": 1609459200
}

```

## 错误处理

### 标准错误响应格式

```

{
"success": false,
"message": "错误描述信息",
"data": null,
"timestamp": 1609459200
}

```

### 常见错误码

| HTTP状态码 | 描述 |
|------------|------|
| 200 | 请求成功 |
| 400 | 请求参数错误 |
| 404 | 资源不存在 |
| 500 | 服务器内部错误 |

## 使用限制

- 单次批量操作最大支持1000个数据点
- 系列键长度限制: 1-255字符
- 标签键值长度限制: 1-100字符
- 单个系列最大标签数量: 20个

## 客户端示例

### Python示例

```

import requests
import time

class TimeSeriesClient:
def __init__(self, base_url="http://localhost:6364"):
self.base_url = base_url

    def health_check(self):
        response = requests.get(f"{self.base_url}/health")
        return response.json()
    
    def insert(self, series_key, timestamp, value, tags=None):
        data = {
            "series_key": series_key,
            "timestamp": timestamp,
            "value": value,
            "tags": tags or {}
        }
        response = requests.post(f"{self.base_url}/api/v1/datapoints", json=data)
        return response.json()
    
    def query(self, series_key, start_time=None, end_time=None, limit=None):
        params = {}
        if start_time:
            params["start_time"] = start_time
        if end_time:
            params["end_time"] = end_time
        if limit:
            params["limit"] = limit
        
        response = requests.get(
            f"{self.base_url}/api/v1/series/{series_key}/datapoints",
            params=params
        )
        return response.json()

# 使用示例

client = TimeSeriesClient()
print(client.health_check())

# 插入数据

result = client.insert(
"temperature_01",
int(time.time()),
23.5,
{"location": "office", "sensor": "DS18B20"}
)
print(result)

# 查询数据

data = client.query("temperature_01")
print(data)

```

### JavaScript示例

```

class TimeSeriesClient {
constructor(baseUrl = "http://localhost:6364") {
this.baseUrl = baseUrl;
}

    async healthCheck() {
        const response = await fetch(`${this.baseUrl}/health`);
        return await response.json();
    }
    
    async insert(seriesKey, timestamp, value, tags = {}) {
        const data = {
            series_key: seriesKey,
            timestamp: timestamp,
            value: value,
            tags: tags
        };
        
        const response = await fetch(`${this.baseUrl}/api/v1/datapoints`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify(data)
        });
        
        return await response.json();
    }
    
    async query(seriesKey, options = {}) {
        const params = new URLSearchParams();
        if (options.startTime) params.append('start_time', options.startTime);
        if (options.endTime) params.append('end_time', options.endTime);
        if (options.limit) params.append('limit', options.limit);
    
        const response = await fetch(
            `${this.baseUrl}/api/v1/series/${seriesKey}/datapoints?${params}`
        );
        return await response.json();
    }
    }

// 使用示例
const client = new TimeSeriesClient();

(async () => {
const health = await client.healthCheck();
console.log(health);

    const result = await client.insert(
        "temperature_01",
        Math.floor(Date.now() / 1000),
        23.5,
        { location: "office", sensor: "DS18B20" }
    );
    console.log(result);
    
    const data = await client.query("temperature_01");
    console.log(data);
    })();

```

### cURL示例

```


# 健康检查

curl http://localhost:6364/health

# 创建数据点

curl -X POST http://localhost:6364/api/v1/datapoints \
-H "Content-Type: application/json" \
-d '{
"series_key": "temperature_sensor_1",
"timestamp": 1609459200,
"value": 23.5,
"tags": {"location": "room1"}
}'

# 查询数据点

curl "http://localhost:6364/api/v1/series/temperature_sensor_1/datapoints?start_time=1609459200\&end_time=1609459800"

# 更新数据点

curl -X PUT http://localhost:6364/api/v1/series/temperature_sensor_1/datapoints/1609459200 \
-H "Content-Type: application/json" \
-d '{"value": 25.0}'

# 删除数据点

curl -X DELETE http://localhost:6364/api/v1/series/temperature_sensor_1/datapoints/1609459200

```

## 性能建议

1. **批量操作**: 优先使用批量插入接口
2. **时间范围**: 查询时指定合理的时间范围
3. **分页查询**: 大数据集使用limit参数分页
4. **连接复用**: 客户端应复用HTTP连接
5. **压缩传输**: 启用HTTP压缩减少传输量

## API版本管理

- 当前版本: v1
- 版本前缀: `/api/v1`
- 向后兼容: 新版本保持向后兼容
- 废弃通知: 废弃功能提前3个月通知
