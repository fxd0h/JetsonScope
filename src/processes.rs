use serde::Serialize;
use sysinfo::{System, Uid};

#[derive(Debug, Clone, Serialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory_kb: u64,
    pub user: Option<String>,
    pub threads: Option<usize>,
}

pub struct ProcessMonitor {
    system: System,
}

impl Default for ProcessMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessMonitor {
    pub fn new() -> Self {
        Self {
            system: System::new_all(),
        }
    }

    pub fn refresh(&mut self) {
        self.system.refresh_all(); // full refresh to keep CPU/mem accurate
    }

    pub fn top_processes(&mut self, limit: usize, sort_by_mem: bool) -> Vec<ProcessInfo> {
        self.refresh();
        let mut processes: Vec<ProcessInfo> = self
            .system
            .processes()
            .iter()
            .map(|(pid, process)| ProcessInfo {
                pid: pid.as_u32(),
                name: process.name().to_string_lossy().to_string(),
                cpu_usage: process.cpu_usage(),
                memory_kb: process.memory() / 1024,
                user: process.user_id().map(|uid: &Uid| uid.to_string()),
                threads: process.tasks().map(|t| t.len()),
            })
            .collect();

        if sort_by_mem {
            processes.sort_by(|a, b| b.memory_kb.cmp(&a.memory_kb));
        } else {
            processes.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap());
        }
        
        processes.truncate(limit);
        processes
    }

    #[allow(dead_code)]
    pub fn top_by_cpu(&mut self, limit: usize) -> Vec<ProcessInfo> {
        self.refresh();
        let mut processes: Vec<ProcessInfo> = self
            .system
            .processes()
            .iter()
            .map(|(pid, process)| ProcessInfo {
                pid: pid.as_u32(),
                name: process.name().to_string_lossy().to_string(),
                cpu_usage: process.cpu_usage(),
                memory_kb: process.memory() / 1024,
                user: process.user_id().map(|uid: &Uid| uid.to_string()),
                threads: process.tasks().map(|t| t.len()),
            })
            .collect();

        processes.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap());
        processes.truncate(limit);
        processes
    }

    #[allow(dead_code)]
    pub fn top_by_memory(&mut self, limit: usize) -> Vec<ProcessInfo> {
        self.refresh();
        let mut processes: Vec<ProcessInfo> = self
            .system
            .processes()
            .iter()
            .map(|(pid, process)| ProcessInfo {
                pid: pid.as_u32(),
                name: process.name().to_string_lossy().to_string(),
                cpu_usage: process.cpu_usage(),
                memory_kb: process.memory() / 1024,
                user: process.user_id().map(|uid: &Uid| uid.to_string()),
                threads: process.tasks().map(|t| t.len()),
            })
            .collect();

        processes.sort_by(|a, b| b.memory_kb.cmp(&a.memory_kb));
        processes.truncate(limit);
        processes
    }
}
