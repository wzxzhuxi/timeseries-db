#!/bin/bash

set -e

# 🧪 时序数据库性能基准测试脚本

# 配置参数
BASE_URL="http://localhost:6364"
TOTAL_POINTS=10000
BATCH_SIZE=100
CONCURRENT_USERS=10
SERIES_COUNT=50

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

echo_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

echo_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

echo_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 检查服务是否可用
check_service() {
    echo_info "检查时序数据库服务状态..."
    if curl -f -s "${BASE_URL}/health" > /dev/null; then
        echo_success "服务运行正常"
    else
        echo_error "服务未运行或不可访问: ${BASE_URL}"
        exit 1
    fi
}

# 创建测试数据文件
create_test_data() {
    echo_info "生成测试数据..."
    
    cat > /tmp/single_datapoint.json << EOF
{
  "series_key": "benchmark_series",
  "timestamp": $(date +%s),
  "value": 42.0,
  "tags": {"test": "benchmark", "type": "single"}
}
EOF

    # 生成批量数据
    echo "[" > /tmp/batch_datapoints.json
    for i in $(seq 1 $BATCH_SIZE); do
        timestamp=$(($(date +%s) + i))
        value=$(echo "scale=2; 20 + ($i % 100) * 0.1" | bc)
        series_id=$((i % SERIES_COUNT))
        
        cat >> /tmp/batch_datapoints.json << EOF
{
  "series_key": "bench_series_${series_id}",
  "timestamp": ${timestamp},
  "value": ${value},
  "tags": {"test": "benchmark", "batch": "true", "series_id": "${series_id}"}
}
EOF
        if [ $i -lt $BATCH_SIZE ]; then
            echo "," >> /tmp/batch_datapoints.json
        fi
    done
    echo "]" >> /tmp/batch_datapoints.json
    
    echo_success "测试数据生成完成"
}

# 单点写入性能测试
test_single_write() {
    echo_info "开始单点写入性能测试..."
    echo_info "测试参数: ${TOTAL_POINTS} 个数据点"
    
    start_time=$(date +%s.%N)
    
    for i in $(seq 1 $TOTAL_POINTS); do
        timestamp=$(($(date +%s) + i))
        value=$(echo "scale=2; 20 + ($i % 100) * 0.1" | bc)
        series_id=$((i % SERIES_COUNT))
        
        curl -s -X POST "${BASE_URL}/api/v1/datapoints" \
            -H "Content-Type: application/json" \
            -d "{
                \"series_key\": \"single_test_${series_id}\",
                \"timestamp\": ${timestamp},
                \"value\": ${value},
                \"tags\": {\"test\": \"single_write\", \"point\": \"${i}\"}
            }" > /dev/null
        
        if [ $((i % 1000)) -eq 0 ]; then
            echo_info "已写入 ${i} 个数据点..."
        fi
    done
    
    end_time=$(date +%s.%N)
    duration=$(echo "$end_time - $start_time" | bc)
    tps=$(echo "scale=2; $TOTAL_POINTS / $duration" | bc)
    
    echo_success "单点写入测试完成"
    echo_success "总数据点: ${TOTAL_POINTS}"
    echo_success "总耗时: ${duration} 秒"
    echo_success "TPS: ${tps}"
}

# 批量写入性能测试
test_batch_write() {
    echo_info "开始批量写入性能测试..."
    batches=$((TOTAL_POINTS / BATCH_SIZE))
    echo_info "测试参数: ${batches} 批次, 每批 ${BATCH_SIZE} 个数据点"
    
    start_time=$(date +%s.%N)
    
    for batch in $(seq 1 $batches); do
        # 动态生成批量数据
        echo "[" > /tmp/current_batch.json
        for i in $(seq 1 $BATCH_SIZE); do
            timestamp=$(($(date +%s) + (batch - 1) * BATCH_SIZE + i))
            value=$(echo "scale=2; 20 + ($i % 100) * 0.1" | bc)
            series_id=$(((batch - 1) * BATCH_SIZE + i % SERIES_COUNT))
            
            cat >> /tmp/current_batch.json << EOF
{
  "series_key": "batch_test_${series_id}",
  "timestamp": ${timestamp},
  "value": ${value},
  "tags": {"test": "batch_write", "batch": "${batch}"}
}
EOF
            if [ $i -lt $BATCH_SIZE ]; then
                echo "," >> /tmp/current_batch.json
            fi
        done
        echo "]" >> /tmp/current_batch.json
        
        curl -s -X POST "${BASE_URL}/api/v1/datapoints/batch" \
            -H "Content-Type: application/json" \
            -d @/tmp/current_batch.json > /dev/null
        
        if [ $((batch % 10)) -eq 0 ]; then
            echo_info "已写入 ${batch} 批次..."
        fi
    done
    
    end_time=$(date +%s.%N)
    duration=$(echo "$end_time - $start_time" | bc)
    total_points=$((batches * BATCH_SIZE))
    tps=$(echo "scale=2; $total_points / $duration" | bc)
    
    echo_success "批量写入测试完成"
    echo_success "总批次: ${batches}"
    echo_success "总数据点: ${total_points}"
    echo_success "总耗时: ${duration} 秒"
    echo_success "TPS: ${tps}"
}

# 并发写入性能测试
test_concurrent_write() {
    echo_info "开始并发写入性能测试..."
    echo_info "测试参数: ${CONCURRENT_USERS} 个并发用户, 每用户 ${TOTAL_POINTS} 个请求"
    
    # 使用Apache Bench进行并发测试
    if command -v ab > /dev/null; then
        echo_info "使用Apache Bench进行并发测试..."
        ab -n $TOTAL_POINTS -c $CONCURRENT_USERS -T 'application/json' \
           -p /tmp/single_datapoint.json \
           "${BASE_URL}/api/v1/datapoints"
    else
        echo_warning "Apache Bench未安装，跳过并发测试"
        echo_info "安装方法: sudo apt-get install apache2-utils"
    fi
}

# 查询性能测试
test_query_performance() {
    echo_info "开始查询性能测试..."
    
    # 查询所有系列
    start_time=$(date +%s.%N)
    curl -s "${BASE_URL}/api/v1/series" > /dev/null
    end_time=$(date +%s.%N)
    list_duration=$(echo "$end_time - $start_time" | bc)
    
    echo_success "系列列表查询耗时: ${list_duration} 秒"
    
    # 查询单个系列的所有数据
    start_time=$(date +%s.%N)
    for i in $(seq 0 9); do
        curl -s "${BASE_URL}/api/v1/series/single_test_${i}/datapoints" > /dev/null
    done
    end_time=$(date +%s.%N)
    query_duration=$(echo "$end_time - $start_time" | bc)
    
    echo_success "10个系列数据查询耗时: ${query_duration} 秒"
    
    # 范围查询测试
    start_time=$(date +%s.%N)
    start_ts=$(($(date +%s) - 3600))  # 1小时前
    end_ts=$(date +%s)
    curl -s "${BASE_URL}/api/v1/series/single_test_0/datapoints?start_time=${start_ts}&end_time=${end_ts}" > /dev/null
    end_time=$(date +%s.%N)
    range_duration=$(echo "$end_time - $start_time" | bc)
    
    echo_success "时间范围查询耗时: ${range_duration} 秒"
}

# 压缩性能测试
test_compression() {
    echo_info "开始压缩性能测试..."
    
    # 获取压缩前统计
    before_stats=$(curl -s "${BASE_URL}/stats")
    
    # 手动触发压缩
    start_time=$(date +%s.%N)
    curl -s -X POST "${BASE_URL}/api/v1/admin/compact" \
        -H "Content-Type: application/json" \
        -d '{"force": true}' > /dev/null
    end_time=$(date +%s.%N)
    
    duration=$(echo "$end_time - $start_time" | bc)
    
    # 获取压缩后统计
    after_stats=$(curl -s "${BASE_URL}/stats")
    
    echo_success "压缩操作耗时: ${duration} 秒"
    echo_info "压缩前统计: ${before_stats}"
    echo_info "压缩后统计: ${after_stats}"
}

# 生成性能报告
generate_report() {
    echo_info "生成性能测试报告..."
    
    report_file="benchmark_report_$(date +%Y%m%d_%H%M%S).txt"
    
    cat > "$report_file" << EOF
时序数据库性能基准测试报告
================================

测试时间: $(date)
测试参数:
- 基础URL: ${BASE_URL}
- 总数据点数: ${TOTAL_POINTS}
- 批次大小: ${BATCH_SIZE}
- 并发用户数: ${CONCURRENT_USERS}
- 系列数量: ${SERIES_COUNT}

数据库统计信息:
$(curl -s "${BASE_URL}/stats")

测试完成时间: $(date)
EOF
    
    echo_success "性能报告已生成: ${report_file}"
}

# 清理测试数据
cleanup() {
    echo_info "清理临时文件..."
    rm -f /tmp/single_datapoint.json
    rm -f /tmp/batch_datapoints.json
    rm -f /tmp/current_batch.json
    echo_success "清理完成"
}

# 主函数
main() {
    echo_info "🚀 开始时序数据库性能基准测试"
    echo_info "======================================"
    
    check_service
    create_test_data
    
    echo_info "开始性能测试..."
    test_single_write
    echo ""
    test_batch_write
    echo ""
    test_concurrent_write
    echo ""
    test_query_performance
    echo ""
    test_compression
    
    generate_report
    cleanup
    
    echo_success "🎉 所有性能测试完成！"
}

# 脚本入口
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi

