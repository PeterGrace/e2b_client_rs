//! Kills every sandbox this run left behind. `just test-live` runs it last.

mod live_support;

use e2b_client::{ListSandboxesParams, SandboxState};
use live_support::{RUN_METADATA_KEY, client, run_id};

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn sweep_leftover_sandboxes_of_this_run() {
    let client = client();
    let base = ListSandboxesParams::new()
        .metadata(RUN_METADATA_KEY, run_id())
        .state(SandboxState::Running)
        .state(SandboxState::Paused);
    let mut leftovers = Vec::new();
    let mut params = base.clone();
    loop {
        let page = client
            .list_sandboxes(&params)
            .await
            .expect("list run sandboxes");
        leftovers.extend(page.sandboxes.into_iter().map(|info| info.sandbox_id));
        match page.next_token {
            Some(token) => params = base.clone().next_token(token),
            None => break,
        }
    }
    for sandbox_id in &leftovers {
        client
            .kill_sandbox(sandbox_id)
            .await
            .expect("kill leftover sandbox");
    }
    eprintln!(
        "sweep: killed {} leftover sandbox(es) of run {}",
        leftovers.len(),
        run_id()
    );
}
