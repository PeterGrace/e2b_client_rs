//! Shared helpers for the live test suites (`tests/live_*.rs`).
//!
//! Every live test is `#[ignore]`d. Run them with `just test-live <target>`.
//! Settings come from the process environment first, then from the nearest
//! `.env` found walking up from the package directory:
//!
//! | Variable | Meaning |
//! |---|---|
//! | `E2B_TEST_TARGET` | `hosted` or `sandbawks` (required) |
//! | `E2B_TEST_RUN_ID` | tags every sandbox of this run (default: clock seconds and pid) |
//! | `E2B_API_KEY_<TARGET>` | API key for the target (required) |
//! | `E2B_DOMAIN_<TARGET>` | domain (hosted default `e2b.app`; required for sandbawks) |
//! | `E2B_TEMPLATE_<TARGET>` | template (default `base`) |
//!
//! `<TARGET>` is `HOSTED` or `SANDBAWKS`. The standard `E2B_API_KEY`,
//! `E2B_API_URL`, `E2B_DOMAIN` and `E2B_SANDBOX_URL` variables must be unset,
//! so a run can never silently talk to the wrong server.

// Each test binary uses a different subset of these helpers; Rust compiles
// this module once per binary and would warn about the unused ones.
#![allow(dead_code)]

use std::collections::HashMap;
use std::future::Future;
use std::ops::Deref;
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use e2b_client::{
    Client, CommandOptions, ConnectionConfig, CreateSandboxParams, RequestOptions, Sandbox,
};

/// Metadata key carrying the run id on every sandbox a live test creates.
pub const RUN_METADATA_KEY: &str = "e2b-client-test";

/// Default lifetime of a test sandbox.
pub const SANDBOX_TIMEOUT: Duration = Duration::from_secs(60);

/// Which server a live run talks to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// Hosted E2B, the canonical server.
    Hosted,
    /// pgrace's E2B-compatible sandbawks gateway.
    Sandbawks,
}

impl Target {
    fn suffix(self) -> &'static str {
        match self {
            Self::Hosted => "HOSTED",
            Self::Sandbawks => "SANDBAWKS",
        }
    }
}

/// The `.env` values, read once. `dotenv_iter` only reads the file; unlike
/// `dotenvy::dotenv()` it never mutates the process environment (which is
/// `unsafe` in Rust 2024).
fn dotenv_values() -> &'static HashMap<String, String> {
    static VALUES: OnceLock<HashMap<String, String>> = OnceLock::new();
    VALUES.get_or_init(|| {
        dotenvy::dotenv_iter()
            .map(|entries| entries.filter_map(Result::ok).collect())
            .unwrap_or_default()
    })
}

/// A setting from the process environment, else `.env`; empty counts as unset.
fn setting(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .or_else(|| dotenv_values().get(name).cloned())
        .filter(|value| !value.is_empty())
}

/// The target selected by `E2B_TEST_TARGET`.
pub fn target() -> Target {
    match setting("E2B_TEST_TARGET").as_deref() {
        Some("hosted") => Target::Hosted,
        Some("sandbawks") => Target::Sandbawks,
        other => panic!("E2B_TEST_TARGET must be `hosted` or `sandbawks`, got {other:?}"),
    }
}

/// The id tagging every sandbox of this run.
pub fn run_id() -> &'static str {
    static RUN_ID: OnceLock<String> = OnceLock::new();
    RUN_ID.get_or_init(|| {
        setting("E2B_TEST_RUN_ID").unwrap_or_else(|| {
            let seconds = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|elapsed| elapsed.as_secs())
                .unwrap_or_default();
            format!("{seconds}-{}", std::process::id())
        })
    })
}

/// Seconds since the Unix epoch, for comparing with `SandboxInfo::end_at`.
pub fn unix_now() -> i64 {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after 1970")
        .as_secs();
    i64::try_from(seconds).expect("seconds fit in i64")
}

/// A client for the selected target.
pub fn client() -> Client {
    for name in [
        "E2B_API_KEY",
        "E2B_API_URL",
        "E2B_DOMAIN",
        "E2B_SANDBOX_URL",
    ] {
        assert!(
            std::env::var_os(name).is_none(),
            "unset {name}: live tests take their settings from E2B_*_<TARGET>"
        );
    }
    let target = target();
    let suffix = target.suffix();
    let api_key = setting(&format!("E2B_API_KEY_{suffix}"))
        .unwrap_or_else(|| panic!("E2B_API_KEY_{suffix} must be set"));
    let mut builder = ConnectionConfig::builder().api_key(api_key);
    match (target, setting(&format!("E2B_DOMAIN_{suffix}"))) {
        (_, Some(domain)) => builder = builder.domain(domain),
        (Target::Hosted, None) => {}
        (Target::Sandbawks, None) => panic!("E2B_DOMAIN_SANDBAWKS must be set"),
    }
    Client::new(builder.build().expect("live config")).expect("live client")
}

/// The template for the selected target.
pub fn template() -> String {
    setting(&format!("E2B_TEMPLATE_{}", target().suffix())).unwrap_or_else(|| "base".into())
}

/// Create parameters every live sandbox starts from: the target's template,
/// [`SANDBOX_TIMEOUT`] and the run-id metadata.
pub fn params() -> CreateSandboxParams {
    params_from(&template())
}

/// Like [`params`], from another template (for example a snapshot ID).
pub fn params_from(template: &str) -> CreateSandboxParams {
    CreateSandboxParams::new(template)
        .timeout(SANDBOX_TIMEOUT)
        .metadata(RUN_METADATA_KEY, run_id())
}

/// A name for a snapshot or volume this run creates, with the run id in it
/// so the sweep finds it. Lowercase, since the API's names are.
/// `resource_name("")` is the prefix every such name of this run starts
/// with, ending in `-`. The sweep finds leftovers only by this prefix, so
/// every snapshot and volume a live test creates must be named with it.
pub fn resource_name(slug: &str) -> String {
    format!("e2b-client-test-{}-{slug}", run_id().to_ascii_lowercase())
}

/// A sandbox that is killed when dropped, even if the test panics.
#[derive(Debug)]
pub struct LiveSandbox {
    client: Client,
    sandbox: Sandbox,
}

impl LiveSandbox {
    /// Create a sandbox from `params` (which should come from [`params`]).
    pub async fn create(params: &CreateSandboxParams) -> Self {
        let client = client();
        let sandbox = client.create_sandbox(params).await.expect("create sandbox");
        Self { client, sandbox }
    }

    /// Create a sandbox with the default [`params`].
    pub async fn new() -> Self {
        Self::create(&params()).await
    }

    /// Takes ownership of a sandbox this test did not create with
    /// [`LiveSandbox::create`] (a fork), so it is killed on drop too.
    pub fn adopt(sandbox: Sandbox) -> Self {
        Self {
            client: client(),
            sandbox,
        }
    }

    /// The client that created this sandbox.
    pub fn client(&self) -> &Client {
        &self.client
    }
}

// `Deref` lets a test call `Sandbox` methods directly on a `LiveSandbox`,
// the way a Python wrapper class would forward attribute access.
impl Deref for LiveSandbox {
    type Target = Sandbox;

    fn deref(&self) -> &Sandbox {
        &self.sandbox
    }
}

impl Drop for LiveSandbox {
    fn drop(&mut self) {
        // `Drop` cannot be async. Kill from a fresh thread with its own runtime
        // and its own `Client`: the test's current-thread runtime is blocked
        // in this `join`, so connections owned by it could not make progress.
        let sandbox_id = self.sandbox.sandbox_id().to_owned();
        let thread_sandbox_id = sandbox_id.clone();
        let outcome = std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map(|runtime| {
                    runtime
                        .block_on(client().kill_sandbox(&thread_sandbox_id, &RequestOptions::new()))
                })
        })
        .join();
        if !matches!(outcome, Ok(Ok(Ok(_)))) {
            eprintln!("warning: could not kill sandbox {sandbox_id}; the sweep will retry");
        }
    }
}

/// Poll `probe` every 2 s until it returns `Some`, panicking after `limit`.
pub async fn poll_until<T, F, Fut>(what: &str, limit: Duration, mut probe: F) -> T
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Option<T>>,
{
    let deadline = tokio::time::Instant::now() + limit;
    loop {
        if let Some(value) = probe().await {
            return value;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out after {limit:?} waiting for {what}"
        );
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

/// Whether `sandbox` gets no HTTP reply from `ip` on two probes in a row
/// (see [`http_reply`]), so that one transient failure does not read as
/// egress being denied.
pub async fn no_http_reply_twice(sandbox: &Sandbox, ip: &str) -> bool {
    // Rust note: `&&` short-circuits as in Python's `and`, so a reply on the
    // first probe skips the second.
    !http_reply(sandbox, ip).await && !http_reply(sandbox, ip).await
}

/// Whether `sandbox` gets an HTTP reply from port 80 of `ip`, within 10 s.
///
/// It needs a reply, not just a connection: hosted E2B completes the TCP
/// handshake on the host even when egress is denied (M1, CLM-0012). The
/// targets are Cloudflare's resolvers (1.1.1.1, 1.0.0.1), which answer HTTP.
pub async fn http_reply(sandbox: &Sandbox, ip: &str) -> bool {
    // `format!` treats `{` and `}` as placeholders; the probe has none
    // besides `{ip}`.
    let probe = format!(
        r#"timeout 10 bash -c 'exec 3<>/dev/tcp/{ip}/80
printf "GET / HTTP/1.0\r\nHost: one.one.one.one\r\n\r\n" >&3
read -r status <&3
[[ $status == HTTP/* ]]'"#
    );
    sandbox
        .commands()
        .run(&probe, &CommandOptions::default())
        .await
        .expect("probe")
        .exit_code
        == 0
}
