mod config;
mod conflict;
mod connection_pool;
mod error;
mod grpc_client;
mod monitoring;
mod network;
mod retry;
mod rules;
mod sync;
mod token;
mod transfer;
mod watcher;

use anyhow::Result;
use clap::{Parser, Subcommand};
use config::ClientConfig;
use conflict::{ConflictResolver, ResolutionStrategy};
use indicatif::{ProgressBar, ProgressStyle};
use monitoring::MonitoringManager;
use rules::RuleEngine;
use std::sync::Arc;
use sync::SyncEngine;
use token::TokenManager;
use tracing::{info, Level};
use transfer::TransferManager;
use uuid::Uuid;

/// Claude CLI 配置同步工具
#[derive(Parser, Debug)]
#[command(name = "claude-sync")]
#[command(author = "Claude Sync Team")]
#[command(version = "0.1.0")]
#[command(about = "Sync Claude CLI configuration across multiple devices", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// 初始化配置
    ConfigInit,

    /// 登录到服务器
    Login {
        /// 邮箱
        #[arg(short, long)]
        email: Option<String>,

        /// 密码
        #[arg(short, long)]
        password: Option<String>,

        /// 设备名称
        #[arg(short, long)]
        device_name: Option<String>,
    },

    /// 登出
    Logout,

    /// 开始同步
    Sync {
        /// 同步模式 (incremental/full/selective)
        #[arg(short, long, default_value = "incremental")]
        mode: String,

        /// 后台运行（守护进程）
        #[arg(short, long)]
        daemon: bool,

        /// 显示详细输出
        #[arg(short, long)]
        verbose: bool,
    },

    /// 查看设备列表
    ListDevices,

    /// 查看同步状态
    Status,

    /// 管理同步规则
    Rules {
        #[command(subcommand)]
        rule_command: RuleCommands,
    },

    /// 检查健康状态
    HealthCheck,

    /// 导出性能指标
    Metrics {
        /// 输出格式 (json/prometheus)
        #[arg(short, long, default_value = "json")]
        format: String,

        /// 输出文件路径（可选，默认输出到控制台）
        #[arg(short, long)]
        output: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum RuleCommands {
    /// 列出所有规则
    List,

    /// 添加新规则
    Add {
        /// 规则名称
        #[arg(short, long)]
        name: String,

        /// 规则类型 (include/exclude)
        #[arg(short, long)]
        rule_type: String,

        /// 文件模式
        #[arg(short, long)]
        pattern: String,

        /// 文件类型
        #[arg(short, long)]
        file_type: Option<String>,

        /// 优先级
        #[arg(short, long, default_value_t = 0)]
        priority: i32,
    },

    /// 删除规则
    Remove {
        /// 规则 ID
        rule_id: String,
    },

    /// 应用推荐规则
    Recommended,
}

#[tokio::main]
async fn main() {
    // 设置 panic hook 以便在崩溃时显示错误
    std::panic::set_hook(Box::new(|panic_info| {
        let backtrace = std::backtrace::Backtrace::capture();
        eprintln!("程序崩溃: {}", panic_info);
        eprintln!("堆栈信息:\n{}", backtrace);
        #[cfg(windows)]
        {
            println!("程序遇到错误，按任意键退出...");
            let _ = std::io::stdin().read_line(&mut String::new());
        }
    }));

    let result = run().await;

    if let Err(e) = result {
        eprintln!("错误: {}", e);
        let mut cause = e.source();
        while let Some(e) = cause {
            eprintln!("  原因: {}", e);
            cause = e.source();
        }

        #[cfg(windows)]
        {
            println!("\n按任意键退出...");
            let _ = std::io::stdin().read_line(&mut String::new());
        }
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();

    // 初始化日志
    let log_level = if matches!(cli.command, Commands::Sync { verbose: true, .. }) {
        Level::DEBUG
    } else {
        Level::INFO
    };

    // 使用 try_init 避免重复初始化
    let _ = tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)
        .try_init();

    info!("🚀 Claude Sync Client v0.1.0");

    match cli.command {
        Commands::ConfigInit => {
            handle_config_init().await?;
        }
        Commands::Login {
            email,
            password,
            device_name,
        } => {
            handle_login(email, password, device_name).await?;
        }
        Commands::Logout => {
            handle_logout().await?;
        }
        Commands::Sync {
            mode,
            daemon,
            verbose,
        } => {
            handle_sync(mode, daemon, verbose).await?;
        }
        Commands::ListDevices => {
            handle_list_devices().await?;
        }
        Commands::Status => {
            handle_status().await?;
        }
        Commands::Rules { rule_command } => {
            handle_rules(rule_command).await?;
        }
        Commands::HealthCheck => {
            handle_health_check().await?;
        }

        Commands::Metrics { format, output } => {
            handle_metrics(format, output).await?;
        }
    }

    Ok(())
}

/// 处理配置初始化
async fn handle_config_init() -> Result<()> {
    info!("初始化配置...");

    let config = ClientConfig::default();
    let config_path = ClientConfig::config_path()?;

    // 保存默认配置
    config.save(&config_path)?;

    // 初始化目录
    config.initialize()?;

    println!("✓ 配置已初始化: {:?}", config_path);
    println!("\n配置文件位置: {:?}", config_path);
    println!("Claude 目录: {:?}", config.sync.claude_dir);
    println!("\n💡 提示: 请编辑配置文件设置服务器地址");

    Ok(())
}

/// 处理登录
async fn handle_login(
    email: Option<String>,
    password: Option<String>,
    device_name: Option<String>,
) -> Result<()> {
    info!("登录到服务器...");

    // 加载配置
    let config = ClientConfig::load()?;
    config.validate()?;

    // 交互式输入
    let email = email.unwrap_or_else(|| {
        dialoguer::Input::new()
            .with_prompt("邮箱")
            .interact()
            .unwrap()
    });

    let password = password.unwrap_or_else(|| {
        dialoguer::Password::new()
            .with_prompt("密码")
            .interact()
            .unwrap()
    });

    let device_name = device_name.unwrap_or_else(|| {
        let hostname = gethostname::gethostname().to_string_lossy().to_string();
        format!("{}-{}", hostname, std::env::consts::OS)
    });

    // 创建 gRPC 客户端
    let client = grpc_client::GrpcClient::new(config.server.address.clone()).await?;

    // 调用登录 API
    let response = client
        .login(email.clone(), password, device_name, "desktop".to_string())
        .await?;

    // 保存 Token
    let token_manager = TokenManager::new(
        config.auth.token_dir,
        config.auth.encryption_key,
        "dummy_jwt_secret".to_string(), // TODO: 从配置获取
    );

    let tokens = token::TokenStorage {
        access_token: response.access_token.clone(),
        refresh_token: response.refresh_token.clone(),
        device_id: response.device_id.to_string(),
        user_id: response.user_id.to_string(),
        access_expires_at: chrono::Utc::now().timestamp() + 3600, // 1 小时
        refresh_expires_at: chrono::Utc::now().timestamp() + 30 * 24 * 3600, // 30 天
    };

    token_manager.save_tokens(tokens)?;

    println!("✓ 登录成功");
    println!("用户 ID: {}", response.user_id);
    println!("设备 ID: {}", response.device_id);
    println!("\n💡 提示: 运行 'claude-sync sync' 开始同步");

    Ok(())
}

/// 处理登出
async fn handle_logout() -> Result<()> {
    info!("登出...");

    let config = ClientConfig::load()?;
    let token_manager = TokenManager::new(
        config.auth.token_dir,
        config.auth.encryption_key,
        "dummy_jwt_secret".to_string(),
    );

    // 检查是否已登录
    if !token_manager.has_tokens() {
        println!("⚠️  未登录");
        return Ok(());
    }

    // TODO: 调用服务器登出 API

    // 删除本地 Token
    token_manager.delete_tokens()?;

    println!("✓ 已登出");

    Ok(())
}

/// 处理同步
async fn handle_sync(mode: String, daemon: bool, _verbose: bool) -> Result<()> {
    info!("开始同步 (模式: {})", mode);

    // 加载配置
    let config = Arc::new(ClientConfig::load()?);
    config.validate()?;

    // 检查登录状态
    let token_manager = TokenManager::new(
        config.auth.token_dir.clone(),
        config.auth.encryption_key.clone(),
        "dummy_jwt_secret".to_string(),
    );

    if !token_manager.has_tokens() {
        anyhow::bail!("未登录，请先运行 'claude-sync login'");
    }

    // 获取用户和设备 ID
    let user_id = Uuid::parse_str(&token_manager.get_user_id()?)?;
    let device_id = Uuid::parse_str(&token_manager.get_device_id()?)?;

    // 创建规则引擎
    let rule_engine = Arc::new(RuleEngine::from_rules(config.sync.rules.clone()));

    // 创建传输管理器
    let transfer_manager = Arc::new(TransferManager::new(
        config.performance.max_concurrent_uploads,
        config.performance.max_concurrent_downloads,
        config.performance.upload_retries,
        config.performance.download_retries,
        config.performance.retry_delay,
    ));

    // 创建冲突解决器
    let conflict_resolver = Arc::new(ConflictResolver::new(
        match config.conflict.default_strategy.as_str() {
            "keep_local" => ResolutionStrategy::KeepLocal,
            "keep_remote" => ResolutionStrategy::KeepRemote,
            "keep_newer" => ResolutionStrategy::KeepNewer,
            _ => ResolutionStrategy::Manual,
        },
        config.conflict.auto_merge_text,
        config.conflict.auto_merge_structured,
    ));

    // 创建同步引擎
    let sync_engine = SyncEngine::new(
        config.clone(),
        rule_engine,
        transfer_manager,
        conflict_resolver,
        user_id,
        device_id,
    );

    match mode.as_str() {
        "full" => {
            // 全量同步
            println!("🔄 开始全量同步...");
            let summary = sync_engine.run_full_sync().await?;

            println!("\n✓ 全量同步完成");
            println!("成功: {}", summary.synced_count);
            println!("失败: {}", summary.failed_count);
            println!("冲突: {}", summary.conflict_count);

            if !summary.conflicts.is_empty() {
                println!("\n冲突文件:");
                for path in &summary.conflicts {
                    println!("  - {:?}", path);
                }
            }

            if !summary.errors.is_empty() {
                println!("\n错误:");
                for (path, error) in &summary.errors {
                    println!("  - {:?}: {}", path, error);
                }
            }
        }
        "incremental" => {
            // 增量同步（实时监控）
            if daemon {
                println!("🔄 后台监控模式（按 Ctrl+C 停止）");
                // TODO: 启动文件监控和实时同步
                println!("⚠️  实时同步功能需要等待 protobuf 代码生成");
            } else {
                println!("⚠️  增量同步需要后台模式运行");
                println!("💡 使用: claude-sync sync --daemon");
            }
        }
        "selective" => {
            // 选择性同步
            println!("🔄 选择性同步...");
            // TODO: 实现选择性同步
            println!("⚠️  选择性同步功能开发中");
        }
        _ => {
            anyhow::bail!("无效的同步模式: {}", mode);
        }
    }

    Ok(())
}

/// 处理设备列表
async fn handle_list_devices() -> Result<()> {
    info!("获取设备列表...");

    // TODO: 实现 gRPC 调用
    println!("⚠️  此功能需要等待 protobuf 代码生成");

    Ok(())
}

/// 处理状态查询
async fn handle_status() -> Result<()> {
    info!("查询同步状态...");

    let config = ClientConfig::load()?;

    let token_manager = TokenManager::new(
        config.auth.token_dir,
        config.auth.encryption_key,
        "dummy_jwt_secret".to_string(),
    );

    if !token_manager.has_tokens() {
        println!("⚠️  未登录");
        return Ok(());
    }

    println!("✓ 已登录");
    println!("用户 ID: {}", token_manager.get_user_id()?);
    println!("设备 ID: {}", token_manager.get_device_id()?);

    // 检查 Token 过期时间
    if token_manager.is_access_expired()? {
        println!("⚠️  Access Token 已过期，需要刷新");
    } else if token_manager.needs_refresh(config.auth.refresh_before as i64)? {
        println!("⚠️  Access Token 即将过期");
    } else {
        println!("✓ Access Token 有效");
    }

    // TODO: 显示同步状态统计

    Ok(())
}

/// 处理规则命令
async fn handle_rules(command: RuleCommands) -> Result<()> {
    info!("管理同步规则...");

    let mut config = ClientConfig::load()?;

    match command {
        RuleCommands::List => {
            println!("同步规则:");
            println!(
                "{:<5} {:<20} {:<10} {:<30}",
                "优先级", "名称", "类型", "模式"
            );
            println!("{}", "-".repeat(70));

            for rule in &config.sync.rules {
                let rule_type_str = match rule.rule_type {
                    rules::RuleType::Include => "include",
                    rules::RuleType::Exclude => "exclude",
                };
                println!(
                    "{:<5} {:<20} {:<10} {:<30}",
                    rule.priority, rule.name, rule_type_str, rule.pattern
                );
            }

            if config.sync.rules.is_empty() {
                println!("(无规则)");
            }
        }
        RuleCommands::Add {
            name,
            rule_type,
            pattern,
            file_type,
            priority,
        } => {
            // 验证规则类型
            let rule_type_enum = match rule_type.as_str() {
                "include" => rules::RuleType::Include,
                "exclude" => rules::RuleType::Exclude,
                _ => anyhow::bail!("无效的规则类型: {}", rule_type),
            };

            let new_rule = rules::SyncRule {
                id: Uuid::new_v4().to_string(),
                name: name.clone(),
                rule_type: rule_type_enum,
                pattern,
                pattern_type: rules::PatternType::Glob, // 默认使用 Glob
                file_type,
                priority,
                enabled: true,
                description: None,
            };

            // 验证规则
            RuleEngine::validate_rule(&new_rule)?;

            // 添加到配置
            config.sync.rules.push(new_rule);

            // 保存配置
            let config_path = ClientConfig::config_path()?;
            config.save(&config_path)?;

            println!("✓ 规则已添加: {}", name);
        }
        RuleCommands::Remove { rule_id } => {
            let original_len = config.sync.rules.len();
            config.sync.rules.retain(|r| r.id != rule_id);

            if config.sync.rules.len() < original_len {
                // 保存配置
                let config_path = ClientConfig::config_path()?;
                config.save(&config_path)?;

                println!("✓ 规则已删除: {}", rule_id);
            } else {
                println!("⚠️  未找到规则: {}", rule_id);
            }
        }
        RuleCommands::Recommended => {
            let recommended = RuleEngine::recommended_rules();

            println!("添加推荐规则:");
            for rule in &recommended {
                println!("  - {} ({})", rule.name, rule.pattern);
            }

            config.sync.rules.extend(recommended);

            // 保存配置
            let config_path = ClientConfig::config_path()?;
            config.save(&config_path)?;

            println!("\n✓ 推荐规则已添加");
        }
    }

    Ok(())
}

/// 处理健康检查
async fn handle_health_check() -> Result<()> {
    info!("检查服务器健康状态...");

    let config = ClientConfig::load()?;

    // TODO: 调用健康检查 API
    println!("⚠️  此功能需要等待 protobuf 代码生成");
    println!("服务器地址: {}", config.server.address);

    Ok(())
}

/// 处理性能指标导出
async fn handle_metrics(format: String, output: Option<String>) -> Result<()> {
    info!("导出性能指标...");

    // 创建监控管理器（实际使用时应该从共享状态获取）
    let manager = MonitoringManager::new(1000, 1000);

    // 根据格式导出指标
    let content = match format.as_str() {
        "json" => {
            let json = manager.export_metrics_json().await?;
            json
        }
        "prometheus" => manager.export_metrics_prometheus().await,
        _ => {
            return Err(anyhow::anyhow!("不支持的格式: {}", format));
        }
    };

    // 输出到文件或控制台
    if let Some(output_path) = output {
        tokio::fs::write(&output_path, content).await?;
        println!("✓ 指标已导出到: {}", output_path);
    } else {
        println!("{}", content);
    }

    // 同时打印性能摘要
    manager.print_performance_summary().await;

    Ok(())
}

/// 创建进度条
fn create_progress_bar(len: u64) -> ProgressBar {
    let pb = indicatif::ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .expect("无效的进度条模板")
            .progress_chars("##-"),
    );
    pb
}
