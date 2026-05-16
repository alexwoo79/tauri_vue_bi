use once_cell::sync::OnceCell;
use serde::Serialize;
use std::env;
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::time::timeout;
use uuid::Uuid;

static PYTHON_AGENT: OnceCell<Mutex<PythonAgentRuntime>> = OnceCell::new();

#[derive(Default)]
struct PythonAgentRuntime {
    child: Option<Child>,
    port: u16,
    base_url: String,
    auth_token: String,
    python_bin: String,
    app_dir: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PythonAgentStatus {
    pub running: bool,
    pub port: u16,
    pub base_url: String,
    pub auth_token: String,
    pub python_bin: String,
    pub app_dir: String,
    pub pid: Option<u32>,
}

fn pick_random_local_port() -> Result<u16, String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| format!("分配随机端口失败: {e}"))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("读取随机端口失败: {e}"))?
        .port();
    drop(listener);
    Ok(port)
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
    eprintln!("[Python Agent Debug] Resolving Python interpreter for app_dir: {}", app_dir);
    
    if let Ok(bin) = env::var("PYTHON_BIN").or_else(|_| env::var("PYTHON_EXECUTABLE")) {
        eprintln!("[Python Agent Debug] Using env-var Python: {}", bin);
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

    eprintln!("[Python Agent Debug] Checking venv interpreter: {}", venv_python.display());
    if venv_python.exists() {
        eprintln!("[Python Agent Debug] venv interpreter found!");
        let bin = venv_python.to_string_lossy().to_string();
        return PythonLaunchSpec {
            program: bin.clone(),
            args: vec![],
            display: bin,
        };
    } else {
        eprintln!("[Python Agent Debug] venv interpreter NOT found at {}", venv_python.display());
    }

    eprintln!("[Python Agent Debug] Checking if 'uv' command available...");
    if command_exists("uv") {
        eprintln!("[Python Agent Debug] Using 'uv' as fallback");
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
    } else {
        eprintln!("[Python Agent Debug] 'uv' command NOT available");
    }

    let fallback = if cfg!(windows) {
        "python".to_string()
    } else {
        "python3".to_string()
    };

    eprintln!("[Python Agent Debug] Using system fallback: {}", fallback);
    PythonLaunchSpec {
        program: fallback.clone(),
        args: vec![],
        display: fallback,
    }
}

fn resolve_app_dir(app_handle: Option<&tauri::AppHandle>) -> String {
    eprintln!("[Python Agent Debug] Resolving app_dir...");
    
    if let Ok(from_env) = env::var("PY_AGENT_DIR") {
        let env_path = PathBuf::from(&from_env);
        eprintln!("[Python Agent Debug] Found PY_AGENT_DIR env var: {}", from_env);
        if env_path.join("app.py").exists() {
            eprintln!("[Python Agent Debug] Using PY_AGENT_DIR: {}", from_env);
            return env_path.to_string_lossy().to_string();
        } else {
            eprintln!("[Python Agent Debug] PY_AGENT_DIR exists but no app.py found at: {}", env_path.join("app.py").display());
        }
    }

    if let Some(app) = app_handle {
        if let Ok(resource_dir) = app.path().resource_dir() {
            let bundled = resource_dir.join("Data-Analysis-Agent");
            eprintln!("[Python Agent Debug] Checking bundled resource dir: {}", bundled.display());
            if bundled.join("app.py").exists() {
                eprintln!("[Python Agent Debug] Using bundled resource dir: {}", bundled.display());
                return bundled.to_string_lossy().to_string();
            } else {
                eprintln!("[Python Agent Debug] Bundled resource dir exists but no app.py: {}", bundled.join("app.py").display());
            }
        } else {
            eprintln!("[Python Agent Debug] Failed to get resource_dir from app handle");
        }
    } else {
        eprintln!("[Python Agent Debug] No app handle provided");
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fallback = manifest_dir
        .parent()
        .map(|p| p.join("Data-Analysis-Agent"))
        .unwrap_or_else(|| manifest_dir.join("Data-Analysis-Agent"));
    eprintln!("[Python Agent Debug] Using fallback dev path: {}", fallback.display());
    fallback.to_string_lossy().to_string()
}

fn status_from_runtime(rt: &PythonAgentRuntime, pid: Option<u32>) -> PythonAgentStatus {
    PythonAgentStatus {
        running: pid.is_some(),
        port: rt.port,
        base_url: rt.base_url.clone(),
        auth_token: rt.auth_token.clone(),
        python_bin: rt.python_bin.clone(),
        app_dir: rt.app_dir.clone(),
        pid,
    }
}

#[tauri::command]
pub async fn start_python_agent(
    app_handle: tauri::AppHandle,
    port: Option<u16>,
) -> Result<PythonAgentStatus, String> {
    let port = match port {
        Some(p) => p,
        None => pick_random_local_port()?,
    };
    let base_url = format!("http://127.0.0.1:{port}");
    let auth_token = Uuid::new_v4().to_string();

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

        let app_dir = resolve_app_dir(Some(&app_handle));
        let launch_spec = resolve_python_launch_spec(&app_dir);
        let app_py = PathBuf::from(&app_dir).join("app.py");

        eprintln!("[Python Agent Debug] app_dir: {}", app_dir);
        eprintln!("[Python Agent Debug] launch_spec.program: {}", launch_spec.program);
        eprintln!("[Python Agent Debug] app_py: {}", app_py.display());

        if !app_py.exists() {
            let err_msg = format!(
                "找不到 Python 启动脚本: {}\n[Debug] app_dir={}, launch_spec={}",
                app_py.display(),
                app_dir,
                launch_spec.display
            );
            eprintln!("[Python Agent Error] {}", err_msg);
            return Err(err_msg);
        }

        let mut cmd = Command::new(&launch_spec.program);
        cmd.current_dir(&app_dir)
            .env("PORT", port.to_string())
            .env("AGENT_PORT", port.to_string())
            .env("AGENT_HOST", "127.0.0.1")
            .env("AGENT_TOKEN", &auth_token)
            .env("PYTHONUNBUFFERED", "1")
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit());

        if !launch_spec.args.is_empty() {
            cmd.args(&launch_spec.args);
        }
        cmd.arg(app_py.as_os_str());

        eprintln!("[Python Agent Debug] Spawning: {} (in {})", launch_spec.display, app_dir);
        let child = cmd.spawn().map_err(|e| {
            let err_msg = format!(
                "启动 Python Agent 失败: {e}\n[Debug] program={}, app_py={}, cwd={}",
                launch_spec.display,
                app_py.display(),
                app_dir
            );
            eprintln!("[Python Agent Error] {}", err_msg);
            err_msg
        })?;

        rt.port = port;
        rt.base_url = base_url.clone();
        rt.auth_token = auth_token;
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
        rt.port = 0;
        rt.base_url = String::new();
    }

    if rt.app_dir.is_empty() {
        rt.app_dir = resolve_app_dir(None);
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
    let healthy = match reqwest::Client::new()
        .get(url)
        .bearer_auth(&status.auth_token)
        .send()
        .await
    {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    };

    Ok(PythonAgentStatus {
        running: healthy,
        ..status
    })
}