//! External command execution helper with environment and I/O controls.
//! （提供外部指令執行的幫手，支援環境變數與 I/O 控制。）
//!
//! The executor wraps `std::process::Command` to deliver a higher-level API
//! suitable for the RustNotePad “Run” feature. It supports deterministic
//! configuration objects that can be serialised, optional stdin piping, and
//! working-directory overrides.
//! 本模組封裝 `std::process::Command`，提供 RustNotePad「執行」功能使用的高階 API。
//! 支援可序列化的設定資料、可選的標準輸入串流，以及更換工作目錄。

use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use thiserror::Error;

/// Errors that may surface while preparing or executing a command.
/// （準備或執行指令時有可能發生的錯誤。）
#[derive(Debug, Error)]
pub enum RunError {
    #[error("failed to spawn process: {0}")]
    Spawn(std::io::Error),
    #[error("process stdin not available")]
    StdinUnavailable,
    #[error("failed to write to stdin: {0}")]
    Stdin(std::io::Error),
    #[error("failed to read process output: {0}")]
    Output(std::io::Error),
    #[error("failed to poll process status: {0}")]
    Poll(std::io::Error),
    #[error("process timed out after {0:?}")]
    TimedOut(Duration),
    #[error("failed to terminate process: {0}")]
    Kill(std::io::Error),
}

/// Captures the desired stdin payload for a command.
/// （描述指令標準輸入要送出的資料內容。）
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StdinPayload {
    Text(String),
    Binary(Vec<u8>),
}

impl StdinPayload {
    fn as_bytes(&self) -> Cow<'_, [u8]> {
        match self {
            StdinPayload::Text(text) => Cow::Borrowed(text.as_bytes()),
            StdinPayload::Binary(bytes) => Cow::Borrowed(bytes),
        }
    }
}

/// Serializable command specification.
/// （可序列化的指令設定資料結構。）
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunSpec {
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub working_dir: Option<PathBuf>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub clear_env: bool,
    #[serde(default)]
    pub stdin: Option<StdinPayload>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default = "default_kill_on_timeout")]
    pub kill_on_timeout: bool,
}

fn default_kill_on_timeout() -> bool {
    true
}

impl RunSpec {
    /// Creates a new command pointing at the given program.
    /// （以指定的程式建立指令設定。）
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            working_dir: None,
            env: BTreeMap::new(),
            clear_env: false,
            stdin: None,
            timeout_ms: None,
            kill_on_timeout: true,
        }
    }

    /// Appends an argument to the command.
    /// （為指令加入一個參數。）
    pub fn push_arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Adds multiple arguments at once.
    /// （一次加入多個參數。）
    pub fn with_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Registers an environment variable override.
    /// （設定環境變數覆寫值。）
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Enables environment clearing before applying overrides.
    /// （先清除既有環境變數，再套用覆寫值。）
    pub fn clear_env(mut self) -> Self {
        self.clear_env = true;
        self
    }

    /// Sets the working directory.
    /// （設定指令執行的工作目錄。）
    pub fn with_working_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.working_dir = Some(path.into());
        self
    }

    /// Supplies stdin payload.
    /// （設定標準輸入要送出的資料。）
    pub fn with_stdin(mut self, payload: StdinPayload) -> Self {
        self.stdin = Some(payload);
        self
    }

    /// Applies a timeout to the command execution.
    /// （設定指令執行的逾時限制。）
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        let millis = timeout.as_millis().clamp(1, u128::from(u64::MAX)) as u64;
        self.timeout_ms = Some(millis);
        self
    }

    /// Controls whether the process is killed after a timeout.
    /// （決定逾時後是否強制終止進程。）
    pub fn with_kill_on_timeout(mut self, kill: bool) -> Self {
        self.kill_on_timeout = kill;
        self
    }
}

/// Result information produced by a command execution.
/// （指令執行完成後的結果資訊。）
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunResult {
    pub exit_code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub duration_ms: u128,
    pub timed_out: bool,
}

impl RunResult {
    /// Indicates whether the command exited successfully (code `0`).
    /// （判斷指令是否以 0 代表成功結束。）
    pub fn success(&self) -> bool {
        !self.timed_out && matches!(self.exit_code, Some(0))
    }
}

/// Executes commands according to the provided specification.
/// （依照設定執行指令的主要元件。）
pub struct RunExecutor;

impl RunExecutor {
    /// Runs the provided command and captures output.
    /// （執行指定指令並擷取輸出。）
    pub fn execute(spec: &RunSpec) -> Result<RunResult, RunError> {
        let mut command = Command::new(&spec.program);
        command.args(&spec.args);
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        if spec.stdin.is_some() {
            command.stdin(Stdio::piped());
        } else {
            command.stdin(Stdio::null());
        }

        if spec.clear_env {
            command.env_clear();
        }

        for (key, value) in &spec.env {
            command.env(key, value);
        }

        if let Some(dir) = &spec.working_dir {
            command.current_dir(dir);
        }

        let start = Instant::now();
        let mut child = command.spawn().map_err(RunError::Spawn)?;

        if let Some(payload) = &spec.stdin {
            let mut stdin = child.stdin.take().ok_or(RunError::StdinUnavailable)?;
            let bytes = payload.as_bytes();
            stdin.write_all(bytes.as_ref()).map_err(RunError::Stdin)?;
            stdin.flush().map_err(RunError::Stdin)?;
        }

        let timeout_duration = spec.timeout_ms.map(Duration::from_millis);
        let mut timed_out = false;
        let output = match timeout_duration {
            Some(timeout) => loop {
                if let Some(_) = child.try_wait().map_err(RunError::Poll)? {
                    break child.wait_with_output().map_err(RunError::Output)?;
                }
                if start.elapsed() >= timeout {
                    if spec.kill_on_timeout {
                        child.kill().map_err(RunError::Kill)?;
                        timed_out = true;
                        break child.wait_with_output().map_err(RunError::Output)?;
                    } else {
                        return Err(RunError::TimedOut(timeout));
                    }
                }
                thread::sleep(Duration::from_millis(15));
            },
            None => child.wait_with_output().map_err(RunError::Output)?,
        };
        let duration = start.elapsed();

        Ok(RunResult {
            exit_code: output.status.code(),
            stdout: output.stdout,
            stderr: output.stderr,
            duration_ms: duration.as_millis(),
            timed_out,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str;
    use std::time::Duration;
    use tempfile::tempdir;

    fn require_utf8(bytes: &[u8]) -> &str {
        str::from_utf8(bytes).expect("output should be valid UTF-8 / 輸出需為有效 UTF-8")
    }

    #[test]
    fn execute_command_with_environment() {
        let spec = RunSpec::new("bash")
            .with_args(["-lc", "printf '%s' \"$RUN_TEST_MESSAGE\""])
            .with_env("RUN_TEST_MESSAGE", "hello-macro");

        let result = RunExecutor::execute(&spec).expect("command should execute / 指令應成功執行");
        assert!(result.success(), "exit code should be zero / 結束碼應為 0");
        assert_eq!(require_utf8(&result.stdout), "hello-macro");
        assert!(
            require_utf8(&result.stderr).is_empty(),
            "stderr should be empty / 錯誤輸出應為空"
        );
        assert!(
            !result.timed_out,
            "command should not time out / 指令不應逾時"
        );
    }

    #[test]
    fn execute_with_custom_working_directory() {
        let temp = tempdir().expect("tempdir should work / 臨時目錄應可建立");
        let spec = RunSpec::new("bash")
            .with_args(["-lc", "pwd"])
            .with_working_dir(temp.path());

        let result = RunExecutor::execute(&spec).expect("command should execute / 指令應成功執行");
        assert!(result.success());
        let output = require_utf8(&result.stdout).trim_end();
        assert_eq!(
            output,
            temp.path()
                .to_str()
                .expect("path convertible to str / 路徑需可轉為字串")
        );
        assert!(
            !result.timed_out,
            "command should not time out / 指令不應逾時"
        );
    }

    #[test]
    fn execute_with_stdin_payload() {
        let spec = RunSpec::new("bash")
            .with_args(["-lc", "read line; printf '%s' \"${line^^}\""])
            .with_stdin(StdinPayload::Text("rustnotepad".into()));

        let result = RunExecutor::execute(&spec).expect("command should execute / 指令應成功執行");
        assert!(result.success());
        assert_eq!(require_utf8(&result.stdout), "RUSTNOTEPAD");
        assert!(
            !result.timed_out,
            "command should not time out / 指令不應逾時"
        );
    }

    #[test]
    fn clear_environment_removes_existing_values() {
        let spec = RunSpec::new("bash")
            .with_args(["-lc", "printf '%s' \"${HOME:-missing}\""])
            .clear_env();

        let result = RunExecutor::execute(&spec).expect("command should execute / 指令應成功執行");
        assert!(result.success());
        assert_eq!(require_utf8(&result.stdout), "missing");
        assert!(
            !result.timed_out,
            "command should not time out / 指令不應逾時"
        );
    }

    #[test]
    fn enforce_timeout_and_kill() {
        let spec = RunSpec::new("bash")
            .with_args(["-lc", "sleep 1 && echo done"])
            .with_timeout(Duration::from_millis(100));

        let result =
            RunExecutor::execute(&spec).expect("command should report timeout / 指令應回報逾時");
        assert!(
            result.timed_out,
            "result should indicate timeout / 結果需標示逾時"
        );
        assert!(
            !result.success(),
            "timed out command should not be success / 逾時指令不應視為成功"
        );
    }

    #[test]
    fn timeout_without_kill_returns_error() {
        let spec = RunSpec::new("bash")
            .with_args(["-lc", "sleep 1"])
            .with_timeout(Duration::from_millis(100))
            .with_kill_on_timeout(false);

        let err = RunExecutor::execute(&spec).unwrap_err();
        assert!(
            matches!(err, RunError::TimedOut(_)),
            "expected timed out error / 預期得到逾時錯誤"
        );
    }

    #[cfg(windows)]
    #[test]
    fn execute_cmd_parity() {
        let spec = RunSpec::new("cmd")
            .with_args(["/C", "echo hello-rustnotepad"])
            .with_timeout(Duration::from_secs(2));

        let result =
            RunExecutor::execute(&spec).expect("cmd command should execute / cmd 指令應成功執行");
        assert!(result.success());
        assert!(
            String::from_utf8_lossy(&result.stdout)
                .trim()
                .ends_with("hello-rustnotepad"),
            "stdout should contain greeting / 標準輸出需含問候字串"
        );
    }

    #[cfg(windows)]
    #[test]
    fn execute_powershell_parity() {
        let spec = RunSpec::new("powershell")
            .with_args(["-NoProfile", "-Command", "Write-Output 'hello-rnp'"])
            .with_timeout(Duration::from_secs(2));

        let result = RunExecutor::execute(&spec)
            .expect("PowerShell command should execute / PowerShell 指令應成功執行");
        assert!(result.success());
        assert!(
            String::from_utf8_lossy(&result.stdout)
                .trim()
                .ends_with("hello-rnp"),
            "stdout should contain marker / 標準輸出需含標記"
        );
    }
}
