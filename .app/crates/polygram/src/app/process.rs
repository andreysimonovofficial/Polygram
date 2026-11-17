use crate::app::PolygramApp;
use std::env;
use std::path::PathBuf;

impl PolygramApp {
    fn exe_dir() -> PathBuf {
        env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| env::current_dir().unwrap_or_default())
    }

    pub fn api_exe_path() -> PathBuf {
        Self::exe_dir().join("polygram-api.exe")
    }

    pub fn mcp_exe_path() -> PathBuf {
        Self::exe_dir().join("polygram-mcp.exe")
    }

    pub fn api_process_running(&mut self) -> bool {
        self.connection.api_process_running()
    }

    pub fn start_api_process(&mut self) {
        match self.connection.start_api_process() {
            Ok(msg) => {
                self.connection.add_connection_log(true, &msg);
            }
            Err(msg) => {
                self.connection.add_connection_log(false, &msg);
            }
        }
    }

    pub fn stop_api_process(&mut self) {
        if self.connection.stop_api_process() {
            self.connection
                .add_connection_log(true, "polygram-api остановлен");
        }
    }

    pub fn restart_api_process(&mut self) {
        self.stop_api_process();
        self.start_api_process();
    }

    pub fn mcp_process_running(&mut self) -> bool {
        self.connection.mcp_process_running()
    }

    pub fn start_mcp_process(&mut self) {
        match self.connection.start_mcp_process() {
            Ok(msg) => {
                self.connection.add_connection_log(true, &msg);
            }
            Err(msg) => {
                self.connection.add_connection_log(false, &msg);
            }
        }
    }

    pub fn stop_mcp_process(&mut self) {
        if self.connection.stop_mcp_process() {
            self.connection
                .add_connection_log(true, "polygram-mcp остановлен");
        }
    }

    pub fn restart_mcp_process(&mut self) {
        self.stop_mcp_process();
        self.start_mcp_process();
    }
}
