use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use tokio::process::Command;

use crate::types::opencode::SessionInfo;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenCodeMode {
    Tui,
    Serve,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DiscoveredInstance {
    pub pid: u32,
    pub port: Option<u16>,
    pub working_dir: PathBuf,
    pub mode: OpenCodeMode,
}

#[allow(dead_code)]
pub struct Discovery;

#[allow(dead_code)]
impl Discovery {
    pub async fn discover_all() -> Result<Vec<DiscoveredInstance>> {
        let ps_output = Self::run_ps_command().await?;
        Self::parse_ps_output_and_discover(&ps_output).await
    }

    pub async fn discover_by_path(path: &Path) -> Result<Option<DiscoveredInstance>> {
        let instances = Self::discover_all().await?;
        let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

        Ok(instances.into_iter().find(|instance| {
            let instance_canonical = instance
                .working_dir
                .canonicalize()
                .unwrap_or_else(|_| instance.working_dir.clone());
            instance_canonical == canonical_path
        }))
    }

    pub async fn get_session_info(port: u16) -> Result<Option<SessionInfo>> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()?;

        let url = format!("http://localhost:{}/sessions", port);

        match client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<Vec<SessionInfo>>().await {
                        Ok(sessions) => Ok(sessions.into_iter().next()),
                        Err(_) => Ok(None),
                    }
                } else {
                    Ok(None)
                }
            }
            Err(_) => Ok(None),
        }
    }

    async fn run_ps_command() -> Result<String> {
        let output = Command::new("ps")
            .args(["aux"])
            .output()
            .await
            .map_err(|e| anyhow!("Failed to run ps command: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(stdout)
    }

    async fn parse_ps_output_and_discover(ps_output: &str) -> Result<Vec<DiscoveredInstance>> {
        let mut instances = Vec::new();

        for line in ps_output.lines() {
            if let Some(parsed) = Self::parse_ps_line(line) {
                let working_dir = match Self::get_working_directory(parsed.pid).await {
                    Ok(Some(dir)) => dir,
                    Ok(None) => parsed.project_path.unwrap_or_else(|| PathBuf::from("/")),
                    Err(_) => parsed.project_path.unwrap_or_else(|| PathBuf::from("/")),
                };

                let port = match Self::get_listening_port(parsed.pid).await {
                    Ok(Some(p)) => Some(p),
                    _ => parsed.port,
                };

                instances.push(DiscoveredInstance {
                    pid: parsed.pid,
                    port,
                    working_dir,
                    mode: parsed.mode,
                });
            }
        }

        Ok(instances)
    }

    fn parse_ps_line(line: &str) -> Option<ParsedProcess> {
        if line.contains("USER") && line.contains("PID") {
            return None;
        }

        let lower_line = line.to_lowercase();
        if !lower_line.contains("opencode") {
            return None;
        }

        if lower_line.contains("grep") {
            return None;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 11 {
            return None;
        }

        let pid = parts.get(1)?.parse::<u32>().ok()?;

        // ps aux: command starts at column 10 (columns 0-9 are USER, PID, %CPU, %MEM, VSZ, RSS, TTY, STAT, START, TIME)
        let command_parts: Vec<&str> = parts.iter().skip(10).copied().collect();
        let command_line = command_parts.join(" ");

        let mode = if command_line.contains("serve") {
            OpenCodeMode::Serve
        } else {
            OpenCodeMode::Tui
        };

        let port = Self::extract_port_from_args(&command_parts);
        let project_path = Self::extract_project_from_args(&command_parts);

        Some(ParsedProcess {
            pid,
            mode,
            port,
            project_path,
        })
    }

    fn extract_port_from_args(args: &[&str]) -> Option<u16> {
        for (i, arg) in args.iter().enumerate() {
            if *arg == "--port" || *arg == "-p" {
                return args.get(i + 1).and_then(|p| p.parse().ok());
            }
            if arg.starts_with("--port=") {
                return arg.strip_prefix("--port=").and_then(|p| p.parse().ok());
            }
        }
        None
    }

    fn extract_project_from_args(args: &[&str]) -> Option<PathBuf> {
        for (i, arg) in args.iter().enumerate() {
            if *arg == "--project" {
                return args.get(i + 1).map(PathBuf::from);
            }
            if arg.starts_with("--project=") {
                return arg.strip_prefix("--project=").map(PathBuf::from);
            }
        }
        None
    }

    async fn get_listening_port(pid: u32) -> Result<Option<u16>> {
        let output = Command::new("lsof")
            .args(["-p", &pid.to_string(), "-a", "-i", "-sTCP:LISTEN"])
            .output()
            .await
            .map_err(|e| anyhow!("Failed to run lsof: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(Self::parse_lsof_port_output(&stdout))
    }

    // lsof output: NAME column format is "*:PORT (LISTEN)" or "localhost:PORT (LISTEN)"
    fn parse_lsof_port_output(output: &str) -> Option<u16> {
        for line in output.lines() {
            if line.contains("LISTEN") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                for part in parts.iter().rev() {
                    if part.contains(':') && !part.starts_with('(') {
                        if let Some(port_str) = part.split(':').next_back() {
                            if let Ok(port) = port_str.parse::<u16>() {
                                return Some(port);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    async fn get_working_directory(pid: u32) -> Result<Option<PathBuf>> {
        let output = Command::new("lsof")
            .args(["-p", &pid.to_string(), "-a", "-d", "cwd"])
            .output()
            .await
            .map_err(|e| anyhow!("Failed to run lsof: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(Self::parse_lsof_cwd_output(&stdout))
    }

    // lsof cwd output: last column is the path
    fn parse_lsof_cwd_output(output: &str) -> Option<PathBuf> {
        for line in output.lines() {
            if line.contains("cwd") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(path) = parts.last() {
                    return Some(PathBuf::from(path));
                }
            }
        }
        None
    }
}

struct ParsedProcess {
    pid: u32,
    mode: OpenCodeMode,
    port: Option<u16>,
    project_path: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ps_output() {
        let parsed = Discovery::parse_ps_line(
            "user     12345  0.0  0.1 123456 7890 pts/0    Sl   10:00   0:01 opencode serve --port 4100 --project /home/user/project",
        );

        assert!(parsed.is_some());
        let process = parsed.unwrap();
        assert_eq!(process.pid, 12345);
        assert_eq!(process.mode, OpenCodeMode::Serve);
        assert_eq!(process.port, Some(4100));
        assert_eq!(
            process.project_path,
            Some(PathBuf::from("/home/user/project"))
        );
    }

    #[test]
    fn test_parse_lsof_port_output() {
        let lsof_output = r#"COMMAND   PID USER   FD   TYPE DEVICE SIZE/OFF NODE NAME
opencode 12345 user   10u  IPv4  0x123  0t0  TCP *:4100 (LISTEN)"#;

        let port = Discovery::parse_lsof_port_output(lsof_output);
        assert_eq!(port, Some(4100));
    }

    #[test]
    fn test_parse_lsof_cwd_output() {
        let lsof_output = r#"COMMAND   PID USER   FD   TYPE DEVICE SIZE/OFF NODE NAME
opencode 12345 user  cwd    DIR  1,4    1024 12345 /home/user/project"#;

        let cwd = Discovery::parse_lsof_cwd_output(lsof_output);
        assert_eq!(cwd, Some(PathBuf::from("/home/user/project")));
    }

    #[test]
    fn test_detect_tui_mode() {
        let parsed = Discovery::parse_ps_line(
            "user     12346  0.0  0.1 123456 7890 pts/1    Sl   10:01   0:00 opencode",
        );

        assert!(parsed.is_some());
        let process = parsed.unwrap();
        assert_eq!(process.mode, OpenCodeMode::Tui);
        assert_eq!(process.port, None);
    }

    #[test]
    fn test_detect_serve_mode() {
        let parsed = Discovery::parse_ps_line(
            "user     12345  0.0  0.1 123456 7890 pts/0    Sl   10:00   0:01 opencode serve --port 4100",
        );

        assert!(parsed.is_some());
        let process = parsed.unwrap();
        assert_eq!(process.mode, OpenCodeMode::Serve);
        assert_eq!(process.port, Some(4100));
    }

    #[test]
    fn test_skip_grep_processes() {
        let parsed = Discovery::parse_ps_line(
            "user     99999  0.0  0.0   5000 1000 pts/2    S+   10:02   0:00 grep opencode",
        );

        assert!(parsed.is_none());
    }

    #[test]
    fn test_skip_header_line() {
        let parsed = Discovery::parse_ps_line(
            "USER       PID  %CPU %MEM    VSZ   RSS TTY      STAT START   TIME COMMAND",
        );

        assert!(parsed.is_none());
    }

    #[test]
    fn test_extract_port_equals_syntax() {
        let args = vec!["opencode", "serve", "--port=4200"];
        let port = Discovery::extract_port_from_args(&args);
        assert_eq!(port, Some(4200));
    }

    #[test]
    fn test_extract_project_from_args() {
        let args = vec![
            "opencode",
            "serve",
            "--project",
            "/path/to/project",
            "--port",
            "4100",
        ];
        let project = Discovery::extract_project_from_args(&args);
        assert_eq!(project, Some(PathBuf::from("/path/to/project")));
    }

    #[tokio::test]
    async fn test_discover_all_returns_empty_when_none() {
        let ps_output = r#"USER       PID  %CPU %MEM    VSZ   RSS TTY      STAT START   TIME COMMAND
user     12345  0.0  0.1 123456 7890 pts/0    Sl   10:00   0:01 vim file.txt
user     12346  0.0  0.1 123456 7890 pts/1    Sl   10:01   0:00 bash"#;

        let result = Discovery::parse_ps_output_and_discover(ps_output).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_invalid_ps_output() {
        let parsed = Discovery::parse_ps_line("invalid line without enough columns");
        assert!(parsed.is_none());
    }

    #[tokio::test]
    async fn test_multiple_processes() {
        let line1 = "user     12345  0.0  0.1 123456 7890 pts/0    Sl   10:00   0:01 opencode serve --port 4100 --project /proj1";
        let line2 = "user     12346  0.0  0.1 123456 7890 pts/1    Sl   10:01   0:00 opencode serve --port 4101 --project /proj2";

        let parsed1 = Discovery::parse_ps_line(line1);
        let parsed2 = Discovery::parse_ps_line(line2);

        assert!(parsed1.is_some());
        assert!(parsed2.is_some());

        let p1 = parsed1.unwrap();
        let p2 = parsed2.unwrap();

        assert_eq!(p1.pid, 12345);
        assert_eq!(p1.port, Some(4100));
        assert_eq!(p2.pid, 12346);
        assert_eq!(p2.port, Some(4101));
    }

    #[test]
    fn test_parse_lsof_port_output_localhost() {
        let lsof_output = r#"COMMAND   PID USER   FD   TYPE DEVICE SIZE/OFF NODE NAME
opencode 12345 user   10u  IPv4  0x123  0t0  TCP localhost:4200 (LISTEN)"#;

        let port = Discovery::parse_lsof_port_output(lsof_output);
        assert_eq!(port, Some(4200));
    }

    #[test]
    fn test_invalid_lsof_port_output() {
        let lsof_output = r#"COMMAND   PID USER   FD   TYPE DEVICE SIZE/OFF NODE NAME
opencode 12345 user   10u  IPv4  0x123  0t0  TCP *:notaport (LISTEN)"#;

        let port = Discovery::parse_lsof_port_output(lsof_output);
        assert!(port.is_none());
    }

    #[test]
    fn test_empty_lsof_output() {
        let port = Discovery::parse_lsof_port_output("");
        assert!(port.is_none());

        let cwd = Discovery::parse_lsof_cwd_output("");
        assert!(cwd.is_none());
    }

    #[tokio::test]
    async fn test_get_session_info_returns_none_on_error() {
        let result = Discovery::get_session_info(59999).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_discovered_instance_construction() {
        let instance = DiscoveredInstance {
            pid: 12345,
            port: Some(4100),
            working_dir: PathBuf::from("/home/user/project"),
            mode: OpenCodeMode::Serve,
        };

        assert_eq!(instance.pid, 12345);
        assert_eq!(instance.port, Some(4100));
        assert_eq!(instance.working_dir, PathBuf::from("/home/user/project"));
        assert_eq!(instance.mode, OpenCodeMode::Serve);
    }

    #[test]
    fn test_opencode_mode_equality() {
        assert_eq!(OpenCodeMode::Tui, OpenCodeMode::Tui);
        assert_eq!(OpenCodeMode::Serve, OpenCodeMode::Serve);
        assert_ne!(OpenCodeMode::Tui, OpenCodeMode::Serve);
    }

    #[test]
    fn test_discovered_instance_clone() {
        let instance = DiscoveredInstance {
            pid: 12345,
            port: Some(4100),
            working_dir: PathBuf::from("/home/user/project"),
            mode: OpenCodeMode::Serve,
        };

        let cloned = instance.clone();
        assert_eq!(cloned.pid, instance.pid);
        assert_eq!(cloned.port, instance.port);
        assert_eq!(cloned.working_dir, instance.working_dir);
        assert_eq!(cloned.mode, instance.mode);
    }

    #[test]
    fn test_extract_port_short_flag() {
        let args = vec!["opencode", "serve", "-p", "4300"];
        let port = Discovery::extract_port_from_args(&args);
        assert_eq!(port, Some(4300));
    }
}
