use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::time::Duration;

#[test]
fn reports_do_not_contact_the_retired_ledger_even_with_legacy_configuration() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let done = Arc::new(AtomicBool::new(false));
    let calls = Arc::new(AtomicUsize::new(0));
    let server_done = done.clone();
    let server_calls = calls.clone();
    let server = std::thread::spawn(move || {
        while !server_done.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((mut connection, _)) => {
                    server_calls.fetch_add(1, Ordering::SeqCst);
                    connection
                        .set_read_timeout(Some(Duration::from_secs(2)))
                        .unwrap();
                    let mut request = [0; 4096];
                    let _ = connection.read(&mut request);
                    let body = r#"{"cards":[]}"#;
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = connection.write_all(response.as_bytes());
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("local test server failed: {error}"),
            }
        }
    });

    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    let output = Command::new(env!("CARGO_BIN_EXE_fleet-retro"))
        .args(["--no-synthesis", "--dry-run", "--no-publish"])
        .arg("--dev-root")
        .arg(root)
        .arg("--feed-dir")
        .arg(root.join("feed"))
        .arg("--campaign-dir")
        .arg(root.join("receipts"))
        .arg("--metrics-dir")
        .arg(root.join("metrics"))
        .arg("--out-root")
        .arg(root.join("reports"))
        .arg("--moment-scorer-script")
        .arg(root.join("absent-scorer.py"))
        .env_remove("FLEET_RETRO_BB_PLANE")
        .env("POWDER_API_BASE_URL", format!("http://{address}"))
        .env("POWDER_API_KEY", "retirement-test-placeholder")
        .output();
    done.store(true, Ordering::SeqCst);
    server.join().unwrap();
    let output = output.unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let spec: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        spec.is_object(),
        "the remaining sources still produce a report"
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "retired services must receive no requests"
    );
}
