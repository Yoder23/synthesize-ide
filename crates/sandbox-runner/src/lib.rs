use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

#[derive(Debug)]
pub struct RunResult {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
}

pub async fn run_argv(argv: &[String], cwd: &str, timeout_seconds: u64) -> anyhow::Result<RunResult> {
    anyhow::ensure!(!argv.is_empty(), "argv cannot be empty");
    let mut cmd = Command::new(&argv[0]);
    cmd.args(&argv[1..])
        .current_dir(cwd)
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let child = cmd.output();
    match timeout(Duration::from_secs(timeout_seconds), child).await {
        Ok(output) => {
            let output = output?;
            Ok(RunResult {
                exit_code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                timed_out: false,
            })
        }
        Err(_) => Ok(RunResult { exit_code: None, stdout: String::new(), stderr: "process timed out".into(), timed_out: true })
    }
}
