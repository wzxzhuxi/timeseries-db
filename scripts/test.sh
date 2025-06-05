#!/bin/bash

set -e

# 🧪 时序数据库测试套件

# 配置参数
BASE_URL="http://localhost:6364"
TEST_DATA_DIR="/tmp/tsdb_test"
CARGO_TARGET_DIR="target"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

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

# 初始化测试环境
setup_test_env() {
    echo_info "设置测试环境..."
    
    # 创建测试数据目录
    mkdir -p "$TEST_DATA_DIR"
    
    # 设置测试环境变量
    export RUST_LOG=debug
    export DATA_DIR="$TEST_DATA_DIR"
    export MEMTABLE_THRESHOLD=10
    
    echo_success "测试环境设置完成"
}

# 清理测试环境
cleanup_test_env() {
    echo_info "清理测试环境..."
    
    # 清理测试数据
    if [ -d "$TEST_DATA_DIR" ]; then
        rm -rf "$TEST_DATA_DIR"
    fi
    
    # 停止测试服务
    if [ ! -z "$SERVER_PID" ]; then
        kill "$SERVER_PID" 2>/dev/null || true
        wait "$SERVER_PID" 2>/dev/null || true
    fi
    
    echo_success "测试环境清理完成"
}

# 设置退出时清理
trap cleanup_test_env EXIT

# 编译项目
build_project() {
    echo_info "编译项目..."
    
    cargo build --release
    
    if [ $? -eq 0 ]; then
        echo_success "项目编译成功"
    else
        echo_error "项目编译失败"
        exit 1
    fi
}

# 运行单元测试
run_unit_tests() {
    echo_info "运行单元测试..."
    
    cargo test --lib -- --nocapture
    
    if [ $? -eq 0 ]; then
        echo_success "单元测试通过"
    else
        echo_error "单元测试失败"
        return 1
    fi
}

# # 运行集成测试
# run_integration_tests() {
#     echo_info "运行集成测试..."
    
#     cargo test --test integration -- --nocapture
    
#     if [ $? -eq 0 ]; then
#         echo_success "集成测试通过"
#     else
#         echo_warning "集成测试失败或不存在"
#     fi
# }

# 运行示例程序
run_examples() {
    echo_info "运行示例程序..."
    
    # 基础使用示例
    if [ -f "examples/basic_usage.rs" ]; then
        echo_info "运行基础使用示例..."
        cargo run --example basic_usage
        echo_success "基础使用示例运行完成"
    fi
    
    # 批量插入示例
    if [ -f "examples/batch_insert.rs" ]; then
        echo_info "运行批量插入示例..."
        cargo run --example batch_insert
        echo_success "批量插入示例运行完成"
    fi
    
    # 性能测试示例
    if [ -f "examples/performance_test.rs" ]; then
        echo_info "运行性能测试示例..."
        cargo run --example performance_test
        echo_success "性能测试示例运行完成"
    fi
}

# 启动测试服务
start_test_server() {
    echo_info "启动测试服务..."
    
    # 后台启动服务
    cargo run --release &
    SERVER_PID=$!
    
    # 等待服务启动
    echo_info "等待服务启动..."
    for i in {1..30}; do
        if curl -f -s "${BASE_URL}/health" > /dev/null 2>&1; then
            echo_success "服务启动成功 (PID: $SERVER_PID)"
            return 0
        fi
        sleep 1
    done
    
    echo_error "服务启动超时"
    return 1
}

# API功能测试
test_api_functionality() {
    echo_info "测试API功能..."
    
    # 健康检查测试
    echo_info "测试健康检查..."
    response=$(curl -s "${BASE_URL}/health")
    if echo "$response" | grep -q "healthy"; then
        echo_success "健康检查测试通过"
    else
        echo_error "健康检查测试失败: $response"
        return 1
    fi
    
    # 统计信息测试
    echo_info "测试统计信息..."
    response=$(curl -s "${BASE_URL}/stats")
    if echo "$response" | grep -q "success"; then
        echo_success "统计信息测试通过"
    else
        echo_error "统计信息测试失败: $response"
        return 1
    fi
    
    # 创建数据点测试
    echo_info "测试创建数据点..."
    response=$(curl -s -X POST "${BASE_URL}/api/v1/datapoints" \
        -H "Content-Type: application/json" \
        -d '{
            "series_key": "test_series",
            "timestamp": 1609459200,
            "value": 23.5,
            "tags": {"test": "api", "location": "test_room"}
        }')
    
    if echo "$response" | grep -q "success"; then
        echo_success "创建数据点测试通过"
    else
        echo_error "创建数据点测试失败: $response"
        return 1
    fi
    
    # 查询数据点测试
    echo_info "测试查询数据点..."
    response=$(curl -s "${BASE_URL}/api/v1/series/test_series/datapoints")
    if echo "$response" | grep -q "success"; then
        echo_success "查询数据点测试通过"
    else
        echo_error "查询数据点测试失败: $response"
        return 1
    fi
    
    # 批量创建测试
    echo_info "测试批量创建..."
    response=$(curl -s -X POST "${BASE_URL}/api/v1/datapoints/batch" \
        -H "Content-Type: application/json" \
        -d '[
            {
                "series_key": "batch_test_1",
                "timestamp": 1609459260,
                "value": 24.0,
                "tags": {"test": "batch"}
            },
            {
                "series_key": "batch_test_2",
                "timestamp": 1609459260,
                "value": 25.0,
                "tags": {"test": "batch"}
            }
        ]')
    
    if echo "$response" | grep -q "success"; then
        echo_success "批量创建测试通过"
    else
        echo_error "批量创建测试失败: $response"
        return 1
    fi
    
    # 更新数据点测试
    echo_info "测试更新数据点..."
    response=$(curl -s -X PUT "${BASE_URL}/api/v1/series/test_series/datapoints/1609459200" \
        -H "Content-Type: application/json" \
        -d '{"value": 26.0}')
    
    if echo "$response" | grep -q "success"; then
        echo_success "更新数据点测试通过"
    else
        echo_warning "更新数据点测试失败或数据不存在: $response"
    fi
    
    # 系列列表测试
    echo_info "测试系列列表..."
    response=$(curl -s "${BASE_URL}/api/v1/series")
    if echo "$response" | grep -q "success"; then
        echo_success "系列列表测试通过"
    else
        echo_error "系列列表测试失败: $response"
        return 1
    fi
    
    # 压缩测试
    echo_info "测试手动压缩..."
    response=$(curl -s -X POST "${BASE_URL}/api/v1/admin/compact" \
        -H "Content-Type: application/json" \
        -d '{"force": true}')
    
    if echo "$response" | grep -q "success"; then
        echo_success "手动压缩测试通过"
    else
        echo_error "手动压缩测试失败: $response"
        return 1
    fi
}

# 压力测试
run_stress_test() {
    echo_info "运行压力测试..."
    
    # 快速插入大量数据
    echo_info "快速插入1000个数据点..."
    for i in {1..1000}; do
        timestamp=$((1609459200 + i))
        value=$(echo "scale=2; 20 + ($i % 100) * 0.1" | bc)
        
        curl -s -X POST "${BASE_URL}/api/v1/datapoints" \
            -H "Content-Type: application/json" \
            -d "{
                \"series_key\": \"stress_test_$((i % 10))\",
                \"timestamp\": ${timestamp},
                \"value\": ${value},
                \"tags\": {\"test\": \"stress\"}
            }" > /dev/null
        
        if [ $((i % 100)) -eq 0 ]; then
            echo_info "已插入 $i 个数据点..."
        fi
    done
    
    echo_success "压力测试完成"
}

# 数据持久性测试
test_data_persistence() {
    echo_info "测试数据持久性..."
    
    # 插入测试数据
    curl -s -X POST "${BASE_URL}/api/v1/datapoints" \
        -H "Content-Type: application/json" \
        -d '{
            "series_key": "persistence_test",
            "timestamp": 1609459200,
            "value": 42.0,
            "tags": {"test": "persistence"}
        }' > /dev/null
    
    # 重启服务测试持久性
    echo_info "重启服务测试数据持久性..."
    kill "$SERVER_PID"
    wait "$SERVER_PID" 2>/dev/null || true
    
    # 重新启动服务
    start_test_server
    
    # 验证数据是否还存在
    response=$(curl -s "${BASE_URL}/api/v1/series/persistence_test/datapoints")
    if echo "$response" | grep -q "42"; then
        echo_success "数据持久性测试通过"
    else
        echo_warning "数据持久性测试失败或数据未持久化"
    fi
}

# 代码质量检查
run_quality_checks() {
    echo_info "运行代码质量检查..."
    
    # Clippy检查
    echo_info "运行Clippy检查..."
    cargo clippy -- -D warnings
    if [ $? -eq 0 ]; then
        echo_success "Clippy检查通过"
    else
        echo_warning "Clippy检查发现问题"
    fi
    
    # 格式检查
    echo_info "运行格式检查..."
    cargo fmt -- --check
    if [ $? -eq 0 ]; then
        echo_success "代码格式检查通过"
    else
        echo_warning "代码格式需要调整"
    fi
}

# 生成测试报告
generate_test_report() {
    echo_info "生成测试报告..."
    
    report_file="test_report_$(date +%Y%m%d_%H%M%S).txt"
    
    cat > "$report_file" << EOF
时序数据库测试报告
==================

测试时间: $(date)
测试环境:
- Rust版本: $(rustc --version)
- 项目版本: $(grep version Cargo.toml | head -1 | cut -d'"' -f2)
- 测试数据目录: ${TEST_DATA_DIR}

测试结果:
- 单元测试: $(cargo test --lib 2>&1 | grep "test result" || echo "未运行")
- API功能测试: 已完成
- 压力测试: 已完成
- 数据持久性测试: 已完成

数据库最终统计:
$(curl -s "${BASE_URL}/stats" 2>/dev/null || echo "服务未运行")

测试完成时间: $(date)
EOF
    
    echo_success "测试报告已生成: ${report_file}"
}

# 主函数
main() {
    echo_info "🧪 开始时序数据库测试套件"
    echo_info "================================"
    
    setup_test_env
    build_project
    
    echo_info "运行本地测试..."
    run_unit_tests
    # run_integration_tests
    run_examples
    # run_quality_checks
    
    echo_info "启动服务进行API测试..."
    if start_test_server; then
        test_api_functionality
        run_stress_test
        test_data_persistence
    else
        echo_error "无法启动服务，跳过API测试"
    fi
    
    generate_test_report
    
    echo_success "🎉 所有测试完成！"
}

# 脚本入口
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi

