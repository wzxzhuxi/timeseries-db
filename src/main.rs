use axum::{
    routing::{get, post, put, delete},
    Router,
    middleware::from_fn,
};
use std::net::SocketAddr;
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use timeseries_db::{
    TimeSeriesDB,
    api::handlers::{
        create_datapoint, create_datapoints_batch, query_datapoints, 
        update_datapoint, delete_datapoint, delete_series, list_series,
        health_check, db_stats, get_series_info, trigger_compaction
    }
};

// 中间件：请求日志
async fn logging_middleware(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let start = std::time::Instant::now();
    
    let response = next.run(req).await;
    
    let duration = start.elapsed();
    tracing::info!(
        "HTTP {} {} - {} - {:?}",
        method,
        uri,
        response.status(),
        duration
    );
    
    response
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化增强的日志系统
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "timeseries_db=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    // 打印启动横幅
    print_banner();
    
    // 初始化数据库
    let memtable_threshold = std::env::var("MEMTABLE_THRESHOLD")
        .unwrap_or_else(|_| "1000".to_string())
        .parse()
        .unwrap_or(1000);
    
    let data_dir = std::env::var("DATA_DIR")
        .unwrap_or_else(|_| "./tsdb_data".to_string());
    
    tracing::info!("初始化数据库，数据目录: {}, 内存表阈值: {}", data_dir, memtable_threshold);
    let db = TimeSeriesDB::new(&data_dir, memtable_threshold)?;
    
    // 启动定期compaction任务
    let db_for_compaction = db.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5分钟
        loop {
            interval.tick().await;
            if let Err(e) = db_for_compaction.compact().await {
                tracing::error!("定期compaction失败: {}", e);
            } else {
                tracing::info!("定期compaction执行完成");
            }
        }
    });
    
    // 构建路由
    let app = Router::new()
        // 健康检查和统计
        .route("/health", get(health_check))
        .route("/stats", get(db_stats))
        
        // 数据点CRUD操作
        .route("/api/v1/datapoints", post(create_datapoint))
        .route("/api/v1/datapoints/batch", post(create_datapoints_batch))
        .route("/api/v1/series/:series_key/datapoints", get(query_datapoints))
        .route("/api/v1/series/:series_key/datapoints/:timestamp", put(update_datapoint))
        .route("/api/v1/series/:series_key/datapoints/:timestamp", delete(delete_datapoint))
        
        // 系列管理
        .route("/api/v1/series", get(list_series))
        .route("/api/v1/series/:series_key", get(get_series_info))
        .route("/api/v1/series/:series_key", delete(delete_series))
        
        // 数据库管理
        .route("/api/v1/admin/compact", post(trigger_compaction))
        
        // 添加中间件
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(TimeoutLayer::new(Duration::from_secs(30)))
                .layer(CorsLayer::permissive())
                .layer(from_fn(logging_middleware))
        )
        .with_state(db);

    // 获取监听地址
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "6364".to_string())
        .parse::<u16>()
        .unwrap_or(6364);
    
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    
    // 打印API信息
    print_api_info(port);
    
    // 启动服务器
    tracing::info!("🚀 时序数据库服务启动完成，监听地址: http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}

fn print_banner() {
    println!(r#"
╔══════════════════════════════════════════════════════════════╗
║                      时序数据库服务                         ║
║                   Time Series Database                       ║
╠══════════════════════════════════════════════════════════════╣
║ 存储引擎: LSM-Tree                                          ║
║ 压缩算法: Gorilla                                           ║
║ 零拷贝技术: mmap                                            ║
║ 异步框架: Tokio + Axum                                      ║
╚══════════════════════════════════════════════════════════════╝
"#);
}

fn print_api_info(port: u16) {
    tracing::info!("📚 API接口列表:");
    tracing::info!("┌─────────────────────────────────────────────────────────────────────────────────┐");
    tracing::info!("│  健康检查与统计                                                                │");
    tracing::info!("│  GET  /health                                    - 健康检查                   │");
    tracing::info!("│  GET  /stats                                     - 数据库统计                 │");
    tracing::info!("├─────────────────────────────────────────────────────────────────────────────────┤");
    tracing::info!("│  数据点操作                                                                    │");
    tracing::info!("│  POST /api/v1/datapoints                         - 创建数据点                 │");
    tracing::info!("│  POST /api/v1/datapoints/batch                   - 批量创建数据点             │");
    tracing::info!("│  GET  /api/v1/series/{{series_key}}/datapoints     - 查询数据点                 │");
    tracing::info!("│  PUT  /api/v1/series/{{series_key}}/datapoints/{{ts}} - 更新数据点                 │");
    tracing::info!("│  DEL  /api/v1/series/{{series_key}}/datapoints/{{ts}} - 删除数据点                 │");
    tracing::info!("├─────────────────────────────────────────────────────────────────────────────────┤");
    tracing::info!("│  系列管理                                                                      │");
    tracing::info!("│  GET  /api/v1/series                             - 获取系列列表               │");
    tracing::info!("│  GET  /api/v1/series/{{series_key}}               - 获取系列信息               │");
    tracing::info!("│  DEL  /api/v1/series/{{series_key}}               - 删除整个系列               │");
    tracing::info!("├─────────────────────────────────────────────────────────────────────────────────┤");
    tracing::info!("│  数据库管理                                                                    │");
    tracing::info!("│  POST /api/v1/admin/compact                      - 手动触发compaction         │");
    tracing::info!("└─────────────────────────────────────────────────────────────────────────────────┘");
    tracing::info!("🌐 服务地址: http://localhost:{}", port);
    tracing::info!("🔧 环境变量:");
    tracing::info!("   PORT              - 服务端口 (默认: 6364)");
    tracing::info!("   DATA_DIR          - 数据目录 (默认: ./tsdb_data)");
    tracing::info!("   MEMTABLE_THRESHOLD - 内存表阈值 (默认: 1000)");
    tracing::info!("   RUST_LOG          - 日志级别 (默认: timeseries_db=info)");
}

