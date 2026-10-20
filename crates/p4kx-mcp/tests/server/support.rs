use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use p4kx_testkit::{ArchiveBuilder, TestEntry, write_build_manifest};
use serde_json::{Value, json};

pub(crate) type TestResult<T = ()> = Result<T, Box<dyn Error>>;

/// The members of the JSON object `json` as written, in order, each with its raw JSON.
pub(crate) fn members(json: &str) -> TestResult<Vec<(String, String)>> {
    struct Ordered(Vec<(String, String)>);
    impl<'de> serde::Deserialize<'de> for Ordered {
        fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            struct Visit;
            impl<'de> serde::de::Visitor<'de> for Visit {
                type Value = Ordered;
                fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.write_str("a JSON object")
                }
                fn visit_map<A: serde::de::MapAccess<'de>>(
                    self,
                    mut map: A,
                ) -> Result<Ordered, A::Error> {
                    let mut out = Vec::new();
                    while let Some((key, value)) =
                        map.next_entry::<String, Box<serde_json::value::RawValue>>()?
                    {
                        out.push((key, value.get().to_owned()));
                    }
                    Ok(Ordered(out))
                }
            }
            deserializer.deserialize_map(Visit)
        }
    }
    Ok(serde_json::from_str::<Ordered>(json)?.0)
}

/// The keys of the JSON object `json` as written, in order.
pub(crate) fn key_order(json: &str) -> TestResult<Vec<String>> {
    Ok(members(json)?.into_iter().map(|(key, _)| key).collect())
}

/// The raw JSON of the member `key` of the JSON object `json`.
pub(crate) fn member(json: &str, key: &str) -> TestResult<String> {
    members(json)?
        .into_iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value)
        .ok_or_else(|| format!("no member {key}").into())
}

const TIMEOUT: Duration = Duration::from_secs(30);

/// A temporary install: a default Library folder with a `LIVE` Channel, an empty launcher folder,
/// a config folder and a working folder for the server.
pub(crate) struct Install {
    root: tempfile::TempDir,
}

impl Install {
    /// Creates the install with `entries` in the `LIVE` Archive.
    pub(crate) fn new(entries: Vec<TestEntry>) -> TestResult<Self> {
        let install = Self {
            root: tempfile::tempdir()?,
        };
        let channel = install.default_library().join("LIVE");
        std::fs::create_dir_all(&channel)?;
        std::fs::create_dir_all(install.launcher().join("logs"))?;
        std::fs::create_dir_all(install.config())?;
        std::fs::create_dir_all(install.work())?;
        write_build_manifest(
            &channel,
            "sc-alpha-4.3.1",
            "4.3.1-live.10000001",
            "10000001",
            "9001",
        )?;
        entries
            .into_iter()
            .fold(ArchiveBuilder::new(), ArchiveBuilder::entry)
            .write_to(install.archive())?;
        Ok(install)
    }

    /// Creates the install with a small fixed set of Entries.
    pub(crate) fn standard() -> TestResult<Self> {
        Self::new(vec![
            TestEntry::stored("Data/Libs/Config/a.xml", b"<a/>".to_vec()),
            TestEntry::deflated("Data/Libs/Config/b.xml", b"<b/>".repeat(20)),
            TestEntry::zstd("Data/Libs/Other/c.txt", b"c".repeat(50)).encrypted(),
            TestEntry::stored("Data/Objects/ship.cgf", b"cgf".to_vec()),
            TestEntry::stored("Data/Game2.dcb", b"dcb".to_vec()),
            TestEntry::stored("readme.txt", b"hello".to_vec()),
        ])
    }

    pub(crate) fn path(&self) -> &Path {
        self.root.path()
    }

    pub(crate) fn default_library(&self) -> PathBuf {
        self.path().join("library")
    }

    pub(crate) fn launcher(&self) -> PathBuf {
        self.path().join("rsilauncher")
    }

    pub(crate) fn config(&self) -> PathBuf {
        self.path().join("config")
    }

    pub(crate) fn work(&self) -> PathBuf {
        self.path().join("work")
    }

    pub(crate) fn archive(&self) -> PathBuf {
        self.default_library().join("LIVE").join("Data.p4k")
    }

    /// Returns the command starting the server with `args` in this install.
    pub(crate) fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_p4kx-mcp"));
        command
            .args(args)
            .env("P4KX_CONFIG_DIR", self.config())
            .env("P4KX_LAUNCHER_DIR", self.launcher())
            .env("P4KX_DEFAULT_LIBRARY", self.default_library())
            .env("P4KX_ARCHIVE_KEY", p4kx_testkit::test_key().to_hex())
            .current_dir(self.work());
        command
    }

    /// Starts the server with `args`, initialised and ready for requests.
    pub(crate) fn serve(&self, args: &[&str]) -> TestResult<Server> {
        self.serve_command(self.command(args))
    }

    /// Starts the server with `args` and no Archive key in its environment.
    pub(crate) fn serve_without_key(&self, args: &[&str]) -> TestResult<Server> {
        let mut command = self.command(args);
        command.env_remove("P4KX_ARCHIVE_KEY");
        self.serve_command(command)
    }

    fn serve_command(&self, mut command: Command) -> TestResult<Server> {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let stdin = child.stdin.take().ok_or("no stdin")?;
        let stdout = child.stdout.take().ok_or("no stdout")?;
        let (sender, lines) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if sender.send(line).is_err() {
                    break;
                }
            }
        });
        let mut server = Server {
            child,
            stdin,
            lines,
            next_id: 0,
        };
        let init = server.request(
            "initialize",
            &json!({
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": { "name": "p4kx-mcp-test", "version": "0.0.0" }
            }),
        )?;
        assert!(init.get("result").is_some(), "initialize succeeds: {init}");
        server.send(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))?;
        Ok(server)
    }
}

/// A running `p4kx-mcp`; dropping it ends the process.
pub(crate) struct Server {
    child: Child,
    stdin: ChildStdin,
    lines: Receiver<String>,
    next_id: u64,
}

impl Server {
    fn send(&mut self, message: &Value) -> TestResult {
        writeln!(self.stdin, "{message}")?;
        self.stdin.flush()?;
        Ok(())
    }

    /// Sends a request and returns the whole response message.
    pub(crate) fn request(&mut self, method: &str, params: &Value) -> TestResult<Value> {
        self.next_id += 1;
        let id = self.next_id;
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))?;
        loop {
            let line = self.lines.recv_timeout(TIMEOUT)?;
            let message: Value = serde_json::from_str(&line)?;
            if message.get("id").and_then(Value::as_u64) == Some(id) {
                return Ok(message);
            }
        }
    }

    /// Returns the tools the server lists.
    pub(crate) fn tools(&mut self) -> TestResult<Vec<Value>> {
        let response = self.request("tools/list", &json!({}))?;
        let tools = response
            .pointer("/result/tools")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("no tool list: {response}"))?;
        Ok(tools.clone())
    }

    /// Calls a tool and returns its result.
    pub(crate) fn call(&mut self, name: &str, arguments: &Value) -> TestResult<Value> {
        let response = self.request(
            "tools/call",
            &json!({ "name": name, "arguments": arguments }),
        )?;
        response
            .get("result")
            .cloned()
            .ok_or_else(|| format!("{name} has no result: {response}").into())
    }

    /// Calls a tool that has to succeed and returns its structured content.
    pub(crate) fn ok(&mut self, name: &str, arguments: &Value) -> TestResult<Value> {
        let result = self.call(name, arguments)?;
        assert_ne!(
            result.get("isError"),
            Some(&Value::Bool(true)),
            "{name} succeeds: {result}"
        );
        result
            .get("structuredContent")
            .cloned()
            .ok_or_else(|| format!("{name} has no structured content: {result}").into())
    }

    /// Calls a tool that has to succeed and returns its result as the JSON text of the server.
    pub(crate) fn ok_text(&mut self, name: &str, arguments: &Value) -> TestResult<String> {
        let result = self.call(name, arguments)?;
        Ok(result
            .pointer("/content/0/text")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("{name} has no text: {result}"))?
            .to_owned())
    }

    /// Calls a tool that has to fail and returns its error text.
    pub(crate) fn error(&mut self, name: &str, arguments: &Value) -> TestResult<String> {
        let result = self.call(name, arguments)?;
        assert_eq!(
            result.get("isError"),
            Some(&Value::Bool(true)),
            "{name} fails: {result}"
        );
        Ok(result
            .pointer("/content/0/text")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned())
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ignored = self.child.kill();
        let _ignored = self.child.wait();
    }
}
