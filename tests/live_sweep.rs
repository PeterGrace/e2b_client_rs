//! Kills every sandbox, and deletes every snapshot, this run left behind.
//! `just test-live` runs it last.

mod live_support;

use e2b_client::{Client, ListSandboxesParams, ListSnapshotsParams, SandboxState};
use live_support::{RUN_METADATA_KEY, client, resource_name, run_id};

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn sweep_leftover_sandboxes_of_this_run() {
    let killed = kill_leftover_sandboxes(&client()).await;
    eprintln!(
        "sweep: killed {killed} leftover sandbox(es) of run {}",
        run_id()
    );
}

/// Kills every running or paused sandbox tagged with this run's id.
///
/// # Returns
///
/// How many it killed.
async fn kill_leftover_sandboxes(client: &Client) -> usize {
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
        // Check the run id again on our side: a server that ignores the
        // metadata filter (sandbawks 0.1.28 does) would otherwise hand us
        // sandboxes this run does not own, and the sweep would kill them.
        leftovers.extend(
            page.sandboxes
                .into_iter()
                .filter(|info| {
                    info.metadata.get(RUN_METADATA_KEY).map(String::as_str) == Some(run_id())
                })
                .map(|info| info.sandbox_id),
        );
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
    leftovers.len()
}

#[tokio::test]
#[ignore = "live: run with `just test-live <target>`"]
async fn sweep_leftover_snapshots_of_this_run() {
    // Snapshots are persistent and billed, and outlive their sandbox, so a
    // test that panics before its own delete would leak one. The live tests
    // name theirs with `resource_name`, so the run's prefix finds them.
    //
    // Hosted E2B refuses to delete a snapshot while a sandbox made from it
    // runs, so the run's sandboxes go first. The sandbox sweep may run at
    // the same time; killing a sandbox it already killed is harmless.
    let client = client();
    kill_leftover_sandboxes(&client).await;
    let marker = resource_name("");
    let mut params = ListSnapshotsParams::new();
    let mut leftovers = Vec::new();
    loop {
        let page = client
            .list_snapshots(&params)
            .await
            .expect("list snapshots");
        leftovers.extend(
            page.snapshots
                .into_iter()
                .filter(|snapshot| snapshot.names.iter().any(|name| name.contains(&marker)))
                .map(|snapshot| snapshot.snapshot_id),
        );
        match page.next_token {
            Some(token) => params = ListSnapshotsParams::new().next_token(token),
            None => break,
        }
    }
    for snapshot_id in &leftovers {
        client
            .delete_snapshot(snapshot_id)
            .await
            .expect("delete leftover snapshot");
    }
    eprintln!(
        "sweep: deleted {} leftover snapshot(s) of run {}",
        leftovers.len(),
        run_id()
    );
}
