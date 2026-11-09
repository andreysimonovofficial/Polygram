use polygram_core::Published;
use reqwest::blocking::Client;
use std::collections::VecDeque;
use std::sync::mpsc;
use std::sync::Arc;

pub struct ConnectionLogEntry {
    pub time: String,
    pub message: String,
    pub ok: bool,
}

pub enum ConnectionResult {
    Connected { rev: u64 },
    Failed { error: String },
}

pub struct ConnectionState {
    pub api_client: Client,
    pub api_base: String,
    pub api_connected: bool,
    pub snapshot: Option<Arc<Published>>,
    pub last_rev: u64,
    pub snapshot_rx: mpsc::Receiver<Arc<Published>>,
    pub api_error: Option<String>,
    pub connection_log: VecDeque<ConnectionLogEntry>,
    pub reconnect_tick: u32,
    pub api_process: Option<std::process::Child>,
    pub mcp_process: Option<std::process::Child>,
    pub warmup_frames: u32,
}

impl ConnectionState {
    pub fn new(api_client: Client, api_base: String, snapshot_rx: mpsc::Receiver<Arc<Published>>) -> Self {
        Self {
            api_client,
            api_base,
            api_connected: false,
            snapshot: None,
            last_rev: 0,
            snapshot_rx,
            api_error: None,
            connection_log: VecDeque::new(),
            reconnect_tick: 150,
            api_process: None,
            mcp_process: None,
            warmup_frames: 120,
        }
    }

    pub fn add_connection_log(&mut self, ok: bool, message: &str) {
        let time = chrono::Local::now().format("%H:%M:%S").to_string();
        self.connection_log.push_back(ConnectionLogEntry {
            time: time.clone(),
            message: message.to_owned(),
            ok,
        });
        if self.connection_log.len() > 50 {
            self.connection_log.pop_front();
        }
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("C:\\Dev\\Polygram\\target\\debug\\gui_connection.log")
            .and_then(|mut f| {
                use std::io::Write;
                writeln!(f, "[{}] {} {}", time, if ok { "OK" } else { "ERR" }, message)
            });
    }

    pub fn try_connect_once(&mut self) -> ConnectionResult {
        let api_base = self.api_base.clone();
        let client = self.api_client.clone();

        let result = (|| -> Result<Arc<Published>, Box<dyn std::error::Error + Send + Sync>> {
            let resp = client.get(format!("{api_base}/api/snapshot")).send()?;
            if !resp.status().is_success() {
                return Err(format!("HTTP {}", resp.status()).into());
            }
            let published: Published = resp.json()?;
            Ok(Arc::new(published))
        })();

        match result {
            Ok(published) => {
                let new_rev = published.rev;
                self.last_rev = new_rev;
                self.snapshot = Some(published);
                self.api_connected = true;
                self.api_error = None;
                ConnectionResult::Connected { rev: new_rev }
            }
            Err(e) => {
                let err_str = e.to_string();
                self.api_connected = false;
                self.api_error = Some(err_str.clone());
                ConnectionResult::Failed { error: err_str }
            }
        }
    }

    pub fn api_process_running(&mut self) -> bool {
        match &mut self.api_process {
            Some(child) => match child.try_wait() {
                Ok(None) => true,
                Ok(Some(_)) => {
                    self.api_process = None;
                    false
                }
                Err(_) => {
                    self.api_process = None;
                    false
                }
            },
            None => false,
        }
    }

    pub fn start_api_process(&mut self) -> Result<String, String> {
        if self.api_process_running() {
            return Err("уже запущен".to_owned());
        }
        match std::process::Command::new("polygram-api.exe").spawn() {
            Ok(child) => {
                self.api_process = Some(child);
                self.api_error = None;
                Ok("polygram-api запущен".to_owned())
            }
            Err(e) => {
                let msg = format!("Не удалось запустить polygram-api: {e}");
                self.api_error = Some(msg.clone());
                Err(msg)
            }
        }
    }

    pub fn stop_api_process(&mut self) -> bool {
        if let Some(mut child) = self.api_process.take() {
            let _ = child.kill();
            true
        } else {
            false
        }
    }

    pub fn restart_api_process(&mut self) -> Result<String, String> {
        self.stop_api_process();
        self.start_api_process()
    }

    pub fn mcp_process_running(&mut self) -> bool {
        // Дочерний процесс, запущенный из GUI.
        if let Some(child) = &mut self.mcp_process {
            match child.try_wait() {
                Ok(None) => return true,
                Ok(Some(_)) => {
                    self.mcp_process = None;
                }
                Err(_) => {
                    self.mcp_process = None;
                }
            }
        }
        // Любой polygram-mcp.exe, запущенный извне (например, вручную или другим агентом).
        match std::process::Command::new("tasklist")
            .args(["/FI", "IMAGENAME eq polygram-mcp.exe", "/FO", "CSV", "/NH"])
            .output()
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout.lines().any(|line| line.to_lowercase().contains("polygram-mcp.exe"))
            }
            Err(_) => false,
        }
    }

    pub fn start_mcp_process(&mut self) -> Result<String, String> {
        if self.mcp_process_running() {
            return Err("уже запущен".to_owned());
        }
        match std::process::Command::new("polygram-mcp.exe").spawn() {
            Ok(child) => {
                self.mcp_process = Some(child);
                Ok("polygram-mcp запущен".to_owned())
            }
            Err(e) => {
                let msg = format!("Не удалось запустить polygram-mcp: {e}");
                self.api_error = Some(msg.clone());
                Err(msg)
            }
        }
    }

    pub fn stop_mcp_process(&mut self) -> bool {
        if let Some(mut child) = self.mcp_process.take() {
            let _ = child.kill();
            true
        } else {
            false
        }
    }

    pub fn restart_mcp_process(&mut self) -> Result<String, String> {
        self.stop_mcp_process();
        self.start_mcp_process()
    }
}
