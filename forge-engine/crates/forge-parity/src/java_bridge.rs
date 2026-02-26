//! Java subprocess bridge for cross-engine parity testing.
//!
//! Launches the Java `forge-harness` JAR as a subprocess and reads JSONL
//! `StateSnapshot` output from its stdout for comparison with Rust snapshots.

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::protocol::StateSnapshot;

/// Configuration for a Java bridge subprocess.
pub struct JavaBridgeConfig {
    /// Path to the Java Forge harness JAR file.
    pub jar_path: PathBuf,
    /// RNG seed for reproducibility.
    pub seed: u64,
    /// Maximum number of turns.
    pub max_turns: u32,
    /// Deck 1 preset ID.
    pub deck1: String,
    /// Deck 2 preset ID.
    pub deck2: String,
    /// Path to the forge-gui/ assets directory (optional, auto-detected from JAR path).
    pub forge_home: Option<String>,
}

/// Java bridge that manages a subprocess running the Java Forge engine.
pub struct JavaBridge {
    pub config: JavaBridgeConfig,
}

impl JavaBridge {
    /// Create a new bridge from configuration.
    pub fn new(config: JavaBridgeConfig) -> Self {
        Self { config }
    }

    /// Run the Java engine and collect snapshots from stdout.
    ///
    /// Launches `java -jar <path> --deck1 ... --deck2 ... --seed ... --max-turns ...`
    /// and reads JSONL output (one `StateSnapshot` per line) from stdout.
    pub fn run(&self) -> Result<Vec<StateSnapshot>, JavaBridgeError> {
        let jar = &self.config.jar_path;

        if !jar.exists() {
            return Err(JavaBridgeError::SpawnError(format!(
                "JAR file not found: {}",
                jar.display()
            )));
        }

        eprintln!("[parity] Launching Java harness: {}", jar.display());
        eprintln!(
            "[parity]   args: --deck1 {} --deck2 {} --seed {} --max-turns {}",
            self.config.deck1, self.config.deck2, self.config.seed, self.config.max_turns
        );

        // Resolve java binary: JAVA_HOME/bin/java if set, otherwise "java"
        let java_bin = std::env::var("JAVA_HOME")
            .ok()
            .map(|home| {
                let bin = PathBuf::from(&home).join("bin").join("java");
                eprintln!("[parity]   using JAVA_HOME: {}", home);
                bin.to_string_lossy().to_string()
            })
            .unwrap_or_else(|| "java".to_string());

        let mut cmd = Command::new(&java_bin);
        cmd.arg("-jar")
            .arg(jar)
            .arg("--deck1")
            .arg(&self.config.deck1)
            .arg("--deck2")
            .arg(&self.config.deck2)
            .arg("--seed")
            .arg(self.config.seed.to_string())
            .arg("--max-turns")
            .arg(self.config.max_turns.to_string());

        // Add --forge-home if specified, otherwise auto-detect from JAR path
        if let Some(ref home) = self.config.forge_home {
            cmd.arg("--forge-home").arg(home);
        } else if let Some(jar_parent) = jar.parent() {
            // Auto-detect: JAR is typically at forge/forge-harness/target/
            // so forge-gui/ is at forge/forge-gui/
            let forge_gui = jar_parent.join("..").join("..").join("forge-gui");
            if forge_gui.join("res").join("cardsfolder").exists() {
                let forge_gui_str = format!("{}/", forge_gui.display());
                eprintln!("[parity]   auto-detected forge-home: {}", forge_gui_str);
                cmd.arg("--forge-home").arg(forge_gui_str);
            }
        }

        let mut child = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| JavaBridgeError::SpawnError(format!("Failed to spawn java: {}", e)))?;

        // Read stderr in a background thread for diagnostics
        let stderr = child.stderr.take();
        let stderr_handle = std::thread::spawn(move || {
            if let Some(stderr) = stderr {
                let reader = BufReader::new(stderr);
                for line in reader.lines().flatten() {
                    eprintln!("[java] {}", line);
                }
            }
        });

        // Read stdout line by line, parse each as a StateSnapshot
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| JavaBridgeError::ProtocolError("No stdout from Java process".into()))?;

        let reader = BufReader::new(stdout);
        let mut snapshots = Vec::new();

        for line_result in reader.lines() {
            let line = line_result.map_err(|e| {
                JavaBridgeError::ProtocolError(format!("Failed to read stdout: {}", e))
            })?;

            let line = line.trim().to_string();
            if line.is_empty() {
                continue;
            }

            match serde_json::from_str::<StateSnapshot>(&line) {
                Ok(snapshot) => {
                    eprintln!(
                        "[parity] Java snapshot: turn={} phase={} game_over={}",
                        snapshot.turn, snapshot.phase, snapshot.game_over
                    );
                    snapshots.push(snapshot);
                }
                Err(e) => {
                    eprintln!(
                        "[parity] Warning: failed to parse Java output as snapshot: {}",
                        e
                    );
                    eprintln!("[parity]   line: {}", line);
                    // Continue reading — might be a diagnostic line that leaked to stdout
                }
            }
        }

        // Wait for process to finish
        let _ = stderr_handle.join();
        let status = child
            .wait()
            .map_err(|e| JavaBridgeError::SpawnError(format!("Failed to wait for java: {}", e)))?;

        if !status.success() {
            let code = status.code().unwrap_or(-1);
            eprintln!("[parity] Java process exited with code {}", code);
            return Err(JavaBridgeError::ProcessError(code));
        }

        eprintln!(
            "[parity] Java harness completed: {} snapshot(s)",
            snapshots.len()
        );
        Ok(snapshots)
    }
}

/// Errors from the Java bridge.
#[derive(Debug)]
pub enum JavaBridgeError {
    /// Failed to start the Java subprocess.
    SpawnError(String),
    /// Communication error (timeout, malformed JSON, etc.).
    ProtocolError(String),
    /// Java process exited with non-zero status.
    ProcessError(i32),
}

impl std::fmt::Display for JavaBridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JavaBridgeError::SpawnError(msg) => write!(f, "Failed to spawn Java: {}", msg),
            JavaBridgeError::ProtocolError(msg) => write!(f, "Protocol error: {}", msg),
            JavaBridgeError::ProcessError(code) => {
                write!(f, "Java process exited with code {}", code)
            }
        }
    }
}

impl std::error::Error for JavaBridgeError {}
