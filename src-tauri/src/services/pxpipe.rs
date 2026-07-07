//! PxPipe bridge lifecycle management.

use crate::database::Database;
use serde::{Deserialize, Serialize};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PxpipeConfig {
    pub enabled: bool,
    pub command: String,
    pub args: Vec<String>,
    pub listen_host: String,
    pub listen_port: u16,
    pub models: String,
    pub log_path: Option<String>,
}

impl Default for PxpipeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            command: "npx".to_string(),
            args: vec!["pxpipe-proxy".to_string()],
            listen_host: "127.0.0.1".to_string(),
            listen_port: 47821,
            models: "claude-fable-5,gpt-5.6".to_string(),
            log_path: None,
        }
    }
}

impl PxpipeConfig {
    fn normalized(mut self) -> Self {
        self.command = self.command.trim().to_string();
        if self.command.is_empty() {
            self.command = "npx".to_string();
        }

        self.args = self
            .args
            .into_iter()
            .map(|arg| arg.trim().to_string())
            .filter(|arg| !arg.is_empty())
            .collect();
        if self.args.is_empty() {
            self.args = vec!["pxpipe-proxy".to_string()];
        }

        self.listen_host = self.listen_host.trim().to_string();
        if self.listen_host.is_empty() {
            self.listen_host = "127.0.0.1".to_string();
        }

        self.models = self.models.trim().to_string();
        if self.models.is_empty() {
            self.models = "claude-fable-5,gpt-5.6".to_string();
        }

        self.log_path = self
            .log_path
            .map(|path| path.trim().to_string())
            .filter(|path| !path.is_empty());

        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PxpipeStatus {
    pub running: bool,
    pub pid: Option<u32>,
    pub listen_url: String,
    pub dashboard_url: String,
    pub upstream_url: String,
    pub last_error: Option<String>,
}

struct PxpipeRuntime {
    child: Option<Child>,
    last_error: Option<String>,
}

#[derive(Clone)]
pub struct PxpipeService {
    db: Arc<Database>,
    runtime: Arc<Mutex<PxpipeRuntime>>,
}

impl PxpipeService {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            runtime: Arc::new(Mutex::new(PxpipeRuntime {
                child: None,
                last_error: None,
            })),
        }
    }

    pub fn get_config(&self) -> Result<PxpipeConfig, String> {
        self.db
            .get_pxpipe_config()
            .map(PxpipeConfig::normalized)
            .map_err(|e| e.to_string())
    }

    pub fn update_config(&self, config: PxpipeConfig) -> Result<(), String> {
        let config = config.normalized();
        Self::validate_config(&config)?;
        self.db
            .set_pxpipe_config(&config)
            .map_err(|e| e.to_string())
    }

    pub async fn get_status(&self) -> Result<PxpipeStatus, String> {
        let config = self.get_config()?;
        let upstream_url = self.build_upstream_url_for_status().await;
        let (running, pid, last_error) = self.reap_exited_child()?;

        Ok(Self::build_status(
            &config,
            upstream_url,
            running,
            pid,
            last_error,
        ))
    }

    pub async fn start(&self) -> Result<PxpipeStatus, String> {
        let mut config = self.get_config()?;
        Self::validate_config(&config)?;
        let upstream_url = self.build_upstream_url().await?;

        {
            let (running, _, _) = self.reap_exited_child()?;
            if running {
                if !config.enabled {
                    config.enabled = true;
                    self.db
                        .set_pxpipe_config(&config)
                        .map_err(|e| e.to_string())?;
                }
                let runtime = self.lock_runtime()?;
                let pid = runtime.child.as_ref().map(Child::id);
                return Ok(Self::build_status(
                    &config,
                    upstream_url,
                    true,
                    pid,
                    runtime.last_error.clone(),
                ));
            }
        }

        let mut command = Command::new(&config.command);
        command
            .args(&config.args)
            .env("ANTHROPIC_UPSTREAM", &upstream_url)
            .env("PXPIPE_UPSTREAM", &upstream_url)
            .env("PORT", config.listen_port.to_string())
            .env("HOST", &config.listen_host)
            .env("PXPIPE_MODELS", &config.models)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        if let Some(log_path) = config
            .log_path
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            command.env("PXPIPE_LOG", log_path);
        }

        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(e) => {
                let message = format!("启动 PxPipe 失败: {e}");
                self.lock_runtime()?.last_error = Some(message.clone());
                return Err(message);
            }
        };
        let pid = Some(child.id());
        config.enabled = true;
        if let Err(e) = self.db.set_pxpipe_config(&config) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(e.to_string());
        }

        {
            let mut runtime = self.lock_runtime()?;
            runtime.child = Some(child);
            runtime.last_error = None;
        }

        log::info!(
            "PxPipe bridge started: {} -> {}",
            Self::listen_url(&config),
            upstream_url
        );

        Ok(Self::build_status(&config, upstream_url, true, pid, None))
    }

    pub async fn stop(&self) -> Result<PxpipeStatus, String> {
        let mut config = self.get_config()?;

        let last_error = {
            let mut runtime = self.lock_runtime()?;
            if let Some(mut child) = runtime.child.take() {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        runtime.last_error = Some(format!("PxPipe exited before stop: {status}"));
                    }
                    Ok(None) => {
                        child.kill().map_err(|e| format!("停止 PxPipe 失败: {e}"))?;
                        let _ = child.wait();
                        runtime.last_error = None;
                    }
                    Err(e) => {
                        runtime.last_error = Some(format!("检查 PxPipe 状态失败: {e}"));
                    }
                }
            }
            runtime.last_error.clone()
        };
        config.enabled = false;
        self.db
            .set_pxpipe_config(&config)
            .map_err(|e| e.to_string())?;
        let upstream_url = self.build_upstream_url_for_status().await;

        Ok(Self::build_status(
            &config,
            upstream_url,
            false,
            None,
            last_error,
        ))
    }

    async fn build_upstream_url(&self) -> Result<String, String> {
        let proxy_config = self
            .db
            .get_proxy_config()
            .await
            .map_err(|e| format!("获取代理配置失败: {e}"))?;

        let connect_host = match proxy_config.listen_address.as_str() {
            "0.0.0.0" => "127.0.0.1".to_string(),
            "::" => "::1".to_string(),
            _ => proxy_config.listen_address.clone(),
        };
        let connect_host = if connect_host.contains(':') && !connect_host.starts_with('[') {
            format!("[{connect_host}]")
        } else {
            connect_host
        };

        if proxy_config.listen_port == 0 {
            return Err("CC Switch 代理监听端口为 0，无法为 PxPipe 生成上游地址".to_string());
        }

        Ok(format!(
            "http://{}:{}",
            connect_host, proxy_config.listen_port
        ))
    }

    async fn build_upstream_url_for_status(&self) -> String {
        self.build_upstream_url()
            .await
            .unwrap_or_else(|e| format!("unavailable: {e}"))
    }

    fn validate_config(config: &PxpipeConfig) -> Result<(), String> {
        if config.command.trim().is_empty() {
            return Err("PxPipe 命令不能为空".to_string());
        }
        if config.listen_host.trim().is_empty() {
            return Err("PxPipe 监听地址不能为空".to_string());
        }
        if config.listen_port == 0 {
            return Err("PxPipe 监听端口不能为 0".to_string());
        }
        Ok(())
    }

    fn reap_exited_child(&self) -> Result<(bool, Option<u32>, Option<String>), String> {
        let mut runtime = self.lock_runtime()?;
        if let Some(child) = runtime.child.as_mut() {
            match child.try_wait() {
                Ok(Some(status)) => {
                    runtime.child = None;
                    runtime.last_error = Some(format!("PxPipe 已退出: {status}"));
                }
                Ok(None) => {
                    return Ok((true, Some(child.id()), runtime.last_error.clone()));
                }
                Err(e) => {
                    runtime.last_error = Some(format!("检查 PxPipe 状态失败: {e}"));
                }
            }
        }
        Ok((false, None, runtime.last_error.clone()))
    }

    fn lock_runtime(&self) -> Result<std::sync::MutexGuard<'_, PxpipeRuntime>, String> {
        self.runtime
            .lock()
            .map_err(|e| format!("PxPipe 状态锁获取失败: {e}"))
    }

    fn build_status(
        config: &PxpipeConfig,
        upstream_url: String,
        running: bool,
        pid: Option<u32>,
        last_error: Option<String>,
    ) -> PxpipeStatus {
        let listen_url = Self::listen_url(config);
        PxpipeStatus {
            running,
            pid,
            dashboard_url: format!("{}/", listen_url.trim_end_matches('/')),
            listen_url,
            upstream_url,
            last_error,
        }
    }

    fn listen_url(config: &PxpipeConfig) -> String {
        let connect_host = match config.listen_host.as_str() {
            "0.0.0.0" => "127.0.0.1",
            "::" => "::1",
            _ => config.listen_host.as_str(),
        };
        let host = if connect_host.contains(':') && !connect_host.starts_with('[') {
            format!("[{}]", connect_host)
        } else {
            connect_host.to_string()
        };
        format!("http://{}:{}", host, config.listen_port)
    }
}
