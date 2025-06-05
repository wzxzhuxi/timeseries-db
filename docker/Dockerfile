# 🏗️ 多阶段构建
FROM rust:1.70-slim as builder

# 📦 安装构建依赖
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 📋 复制依赖文件
COPY Cargo.toml Cargo.lock ./

# 📁 复制源代码
COPY src ./src

# 🔨 构建发布版本
RUN cargo build --release

# 🚀 运行时镜像
FROM debian:bookworm-slim

# 📦 安装运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# 👤 创建非root用户
RUN useradd -r -s /bin/false tsdb

WORKDIR /app

# 📥 复制编译好的二进制文件
COPY --from=builder /app/target/release/timeseries-db /usr/local/bin/timeseries-db

# 🔒 设置权限
RUN chmod +x /usr/local/bin/timeseries-db

# 📁 创建数据目录
RUN mkdir -p /app/data /app/logs && \
    chown -R tsdb:tsdb /app

# ⚙️ 环境变量配置
ENV PORT=6364
ENV DATA_DIR=/app/data
ENV RUST_LOG=info
ENV MEMTABLE_THRESHOLD=1000

# 📡 暴露端口
EXPOSE 6364

# 💾 数据卷
VOLUME ["/app/data", "/app/logs"]

# 🏥 健康检查
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
  CMD curl -f http://localhost:6364/health || exit 1

# 👤 切换到非root用户
USER tsdb

# 🚀 启动命令
CMD ["timeseries-db"]

