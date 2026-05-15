use once_cell::sync::OnceCell;
use serde::Serialize;
use std::env;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::time::timeout;

static PYTHON_AGENT: OnceCell<Mutex<PythonAgentRuntime>> = OnceCell::new();

#[derive(Default)]
struct PythonAgentRuntime {
    child: Option<Child>,
    port: u16,
    base_url: String,
    python_bin: String,
    app_dir: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PythonAgentStatus {
    pub running: bool,
    pub port: u16,
    pub base_url: String,
    pub python_bin: String,
    pub app_dir: String,
    pub pid: Option<u32>,
}

fn runtime() -> &'static Mutex<PythonAgentRuntime> {
    PYTHON_AGENT.get_or_init(|| Mutex::new(PythonAgentRuntime::default()))
}

#[derive(Debug, Clone)]
struct PythonLaunchSpec {
    program: String,
    args: Vec<String>,
    display: String,
}

fn command_exists(cmd: &str) -> bool {
    let checker = if cfg!(windows) { "where" } else { "which" };
    std::process::Command::new(checker)
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn resolve_python_launch_spec(app_dir: &str) -> PythonLaunchSpec {
    if let Ok(bin) = env::var("PYTHON_BIN").or_else(|_| env::var("PYTHON_EXECUTABLE")) {
        return PythonLaunchSpec {
            program: bin.clone(),
            args: vec![],
            display: bin,
        };
    }

    let venv_python = if cfg!(windows) {
        PathBuf::from(app_dir).join(".venv").join("Scripts").join("python.exe")
    } else {
        PathBuf::from(app_dir).join(".venv").join("bin").join("python")
    };

    if venv_python.exists() {
        let bin = venv_python.to_string_lossy().to_string();
        return PythonLaunchSpec {
            program: bin.clone(),
            args: vec![],
            display: bin,
        };
    }

    if command_exists("uv") {
        return PythonLaunchSpec {
            program: "uv".to_string(),
            args: vec![
                "run".to_string(),
                "--project".to_string(),
                app_dir.to_string(),
                "python".to_string(),
            ],
            display: format!("uv run --project {app_dir} python"),
        };
    }

    let fallback = if cfg!(windows) {
        "python".to_string()
    } else {
        "python3".to_string()
    };

    PythonLaunchSpec {
        program: fallback.clone(),
        args: vec![],
        display: fallback,
    }
}

fn resolve_app_dir() -> String {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .map(|p| p.join("Data-Analysis-Agent"))
        .unwrap_or_else(|| manifest_dir.join("Data-Analysis-Agent"))
        .to_string_lossy()
        .to_string()
}

fn status_from_runtime(rt: &PythonAgentRuntime, pid: Option<u32>) -> PythonAgentStatus {
    PythonAgentStatus {
        running: pid.is_some(),
        port: rt.port,
        base_url: rt.base_url.clone(),
        python_bin: rt.python_bin.clone(),
        app_dir: rt.app_dir.clone(),
        pid,
    }
}

#[tauri::command]
pub async fn start_python_agent(port: Option<u16>) -> Result<PythonAgentStatus, String> {
    let port = port.unwrap_or(5001);
    let base_url = format!("http://127.0.0.1:{port}");

    {
        let mut rt = runtime().lock().map_err(|e| e.to_string())?;

        let running_pid = if let Some(child) = rt.child.as_mut() {
            match child.try_wait() {
                Ok(None) => child.id(),
                Ok(Some(_)) | Err(_) => None,
            }
        } else {
            None
        };

        if let Some(pid) = running_pid {
            return Ok(status_from_runtime(&rt, Some(pid)));
        }

        if let Some(child) = rt.child.as_mut() {
            if matches!(child.try_wait(), Ok(Some(_)) | Err(_)) {
                rt.child = None;
            }
        }

        let app_dir = resolve_app_dir();
        let launch_spec = resolve_python_launch_spec(&app_dir);
        let app_py = PathBuf::from(&app_dir).join("app.py");

        if !app_py.exists() {
            return Err(format!("找不到 Python 启动脚本: {}", app_py.display()));
        }

        let mut cmd = Command::new(&launch_spec.program);
        cmd.current_dir(&app_dir)
            .env("PORT", port.to_string())
            .env("AGENT_PORT", port.to_string())
            .env("PYTHONUNBUFFERED", "1")
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit());

        if !launch_spec.args.is_empty() {
            cmd.args(&launch_spec.args);
        }
        cmd.arg(app_py.as_os_str());

        let child = cmd.spawn().map_err(|e| {
            format!(
                "启动 Python Agent 失败: {e}; python={}, app_py={}",
                launch_spec.display,
                app_py.display()
            )
        })?;

        rt.port = port;
        rt.base_url = base_url.clone();
        rt.python_bin = launch_spec.display;
        rt.app_dir = app_dir;
        rt.child = Some(child);
    }

    // 等待服务短暂启动，尽快给前端一个可用地址。
    let _ = timeout(Duration::from_secs(2), async {
        for _ in 0..20u32 {
            if python_agent_health().await.ok().map(|s| s.running).unwrap_or(false) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await;

    python_agent_status().await
}

#[tauri::command]
pub async fn stop_python_agent() -> Result<PythonAgentStatus, String> {
    let mut child = {
        let mut rt = runtime().lock().map_err(|e| e.to_string())?;
        rt.child.take()
    };

    if let Some(mut process) = child.take() {
        let _ = process.kill().await;
        let _ = process.wait().await;
    }

    python_agent_status().await
}

#[tauri::command]
pub async fn python_agent_status() -> Result<PythonAgentStatus, String> {
    let mut rt = runtime().lock().map_err(|e| e.to_string())?;
    let pid = if let Some(child) = rt.child.as_mut() {
        match child.try_wait() {
            Ok(None) => child.id(),
            Ok(Some(_)) | Err(_) => {
                rt.child = None;
                None
            }
        }
    } else {
        None
    };

    if rt.base_url.is_empty() {
        rt.port = 5001;
        rt.base_url = format!("http://127.0.0.1:{}", rt.port);
    }

    Ok(status_from_runtime(&rt, pid))
}

#[tauri::command]
pub async fn python_agent_health() -> Result<PythonAgentStatus, String> {
    let status = python_agent_status().await?;

    if !status.running {
        return Ok(status);
    }

    let url = format!("{}/api/models", status.base_url);
    let healthy = match reqwest::get(url).await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    };

    Ok(PythonAgentStatus {
        running: healthy,
        ..status
    })
}