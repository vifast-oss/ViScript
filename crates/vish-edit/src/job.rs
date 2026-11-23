// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-304b** — job control POSIX (`&`, `jobs`, `fg`, `bg`, `kill`).
//!
//! Phần parse/state dùng được mọi nền; spawn/wait/kill chỉ bật trên Unix (`cfg(unix)`).
//! Windows biên dịch được và trả lỗi rõ khi yêu cầu spawn.

#[cfg(unix)]
use std::process::Command;

#[cfg(unix)]
use std::process::Child;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobCommand {
    Spawn(String),
    List,
    Fg(Option<u32>),
    Bg(Option<u32>),
    Kill(Option<u32>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    Running,
    Done,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobInfo {
    pub id: u32,
    pub cmd: String,
    pub state: JobState,
}

#[derive(Debug)]
struct Job {
    id: u32,
    cmd: String,
    #[cfg(unix)]
    child: Option<Child>,
    done: bool,
}

#[derive(Debug, Default)]
pub struct JobControl {
    next: u32,
    jobs: Vec<Job>,
}

impl JobControl {
    pub fn new() -> Self {
        Self {
            next: 1,
            jobs: Vec::new(),
        }
    }

    pub fn jobs(&mut self) -> Vec<JobInfo> {
        self.cap_nhat();
        self.jobs
            .iter()
            .filter(|j| !j.done)
            .map(|j| JobInfo {
                id: j.id,
                cmd: j.cmd.clone(),
                state: if j.done {
                    JobState::Done
                } else {
                    JobState::Running
                },
            })
            .collect()
    }

    /// Xử lý một dòng job control; trả các dòng phản hồi cho ViShell.
    pub fn handle(&mut self, line: &str) -> Result<Vec<String>, String> {
        let Some(cmd) = parse_command(line) else {
            return Err(format!("lệnh job không hợp lệ: `{line}`"));
        };
        match cmd {
            JobCommand::Spawn(c) => self.spawn(&c),
            JobCommand::List => Ok(self
                .jobs()
                .into_iter()
                .map(|j| {
                    format!(
                        "[{}] {} {}",
                        j.id,
                        if j.state == JobState::Running {
                            "running"
                        } else {
                            "done"
                        },
                        j.cmd
                    )
                })
                .collect()),
            JobCommand::Fg(id) => self.fg(id),
            JobCommand::Bg(id) => self.bg(id),
            JobCommand::Kill(id) => self.kill(id),
        }
    }

    fn spawn(&mut self, cmd: &str) -> Result<Vec<String>, String> {
        #[cfg(unix)]
        {
            let child = Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .spawn()
                .map_err(|e| format!("không spawn được `{cmd}`: {e}"))?;
            let id = self.next;
            self.next += 1;
            self.jobs.push(Job {
                id,
                cmd: cmd.to_string(),
                child: Some(child),
                done: false,
            });
            Ok(vec![format!("[{id}] {cmd} &")])
        }
        #[cfg(not(unix))]
        {
            let _ = cmd;
            Err("job control spawn chỉ hỗ trợ POSIX (Unix/macOS)".to_string())
        }
    }

    fn chon(&mut self, id: Option<u32>) -> Result<usize, String> {
        self.cap_nhat();
        let idx = match id {
            Some(id) => self.jobs.iter().position(|j| j.id == id),
            None => self.jobs.iter().rposition(|j| !j.done),
        };
        idx.ok_or_else(|| "không có job phù hợp".to_string())
    }

    fn fg(&mut self, id: Option<u32>) -> Result<Vec<String>, String> {
        let idx = self.chon(id)?;
        #[cfg(unix)]
        {
            if let Some(mut c) = self.jobs[idx].child.take() {
                let _ = c.wait();
            }
        }
        let j = self.jobs.remove(idx);
        Ok(vec![format!("[{}] {} (fg)", j.id, j.cmd)])
    }

    fn bg(&mut self, id: Option<u32>) -> Result<Vec<String>, String> {
        let idx = self.chon(id)?;
        let j = &self.jobs[idx];
        Ok(vec![format!("[{}] {} & (bg)", j.id, j.cmd)])
    }

    fn kill(&mut self, id: Option<u32>) -> Result<Vec<String>, String> {
        let idx = self.chon(id)?;
        #[cfg(unix)]
        {
            if let Some(mut c) = self.jobs[idx].child.take() {
                let _ = c.kill();
                let _ = c.wait();
            }
        }
        let j = self.jobs.remove(idx);
        Ok(vec![format!("[{}] {} (kill)", j.id, j.cmd)])
    }

    fn cap_nhat(&mut self) {
        #[cfg(unix)]
        for j in &mut self.jobs {
            if !j.done {
                if let Some(c) = j.child.as_mut() {
                    if matches!(c.try_wait(), Ok(Some(_))) {
                        j.done = true;
                    }
                }
            }
        }
    }
}

/// Parse cú pháp job control; `None` nếu không phải lệnh job.
pub fn parse_command(line: &str) -> Option<JobCommand> {
    let s = line.trim();
    if s.is_empty() {
        return None;
    }
    if let Some(cmd) = s.strip_suffix('&') {
        let cmd = cmd.trim();
        return (!cmd.is_empty()).then(|| JobCommand::Spawn(cmd.to_string()));
    }
    let mut it = s.split_whitespace();
    let ten = it.next()?;
    match ten {
        "jobs" => Some(JobCommand::List),
        "fg" => Some(JobCommand::Fg(it.next().and_then(|x| x.parse().ok()))),
        "bg" => Some(JobCommand::Bg(it.next().and_then(|x| x.parse().ok()))),
        "kill" => {
            let args: Vec<&str> = it.collect();
            let id = args
                .iter()
                .rev()
                .find_map(|x| x.trim_start_matches('%').parse::<u32>().ok());
            Some(JobCommand::Kill(id))
        }
        _ => None,
    }
}
