pub mod battery;

use std::time::Duration;

use serde::Serialize;
use sysinfo::System;
use tauri::{AppHandle, Emitter};

#[derive(Serialize, Clone)]
pub struct SysSample {
    pub cpu: f32,
    pub mem_used: u64,
    pub mem_total: u64,
    pub battery: Option<battery::Battery>,
}

pub async fn monitor_loop(app: AppHandle) {
    let mut sys = System::new();
    let mut tick = tokio::time::interval(Duration::from_secs(1));
    loop {
        tick.tick().await;
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        let sample = SysSample {
            cpu: sys.global_cpu_usage(),
            mem_used: sys.used_memory(),
            mem_total: sys.total_memory(),
            battery: battery::read(),
        };
        let _ = app.emit("sysmon", sample);
    }
}
