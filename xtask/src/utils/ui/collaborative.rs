use crate::utils::logging;
use anyhow::Context;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_tungstenite::accept_async;

pub async fn start_collaboration_server(port: u16) -> anyhow::Result<()> {
    let addr = format!("0.0.0.0:{}", port);
    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(_) => {
            let alt_addr = format!("0.0.0.0:{}", port + 1);
            TcpListener::bind(&alt_addr)
                .await
                .context(format!("Failed to bind to both {} and {}", addr, alt_addr))?
        }
    };

    let local_addr = listener.local_addr()?;
    logging::status(
        "COLLAB",
        &format!("Collaboration server live at ws://{}", local_addr),
    );
    let (tx, _rx) = broadcast::channel(10);
    let tx = Arc::new(tx);

    let tx_clone = tx.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            if let Ok(term) = crate::utils::ui::vterm::VTERM.lock() {
                let msg = serde_json::to_string(&*term).unwrap_or_default();
                let _ = tx_clone.send(msg);
            }
        }
    });

    while let Ok((stream, _)) = listener.accept().await {
        let tx = tx.clone();
        tokio::spawn(async move {
            if let Ok(ws_stream) = accept_async(stream).await {
                let (mut ws_sender, _ws_receiver) = ws_stream.split();
                let mut rx = tx.subscribe();
                while let Ok(msg) = rx.recv().await {
                    if ws_sender
                        .send(tokio_tungstenite::tungstenite::Message::Text(msg))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            }
        });
    }
    Ok(())
}

pub async fn run_collaboration_client(addr: &str) -> anyhow::Result<()> {
    let (ws_stream, _) = tokio_tungstenite::connect_async(format!("ws://{}", addr)).await?;
    logging::status(
        "COLLAB",
        &format!("Joined session at {}. Watching...", addr),
    );

    let (_ws_sender, mut ws_receiver) = ws_stream.split();
    while let Some(msg) = ws_receiver.next().await {
        if let Ok(msg) = msg {
            if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                let term: crate::utils::ui::vterm::VirtualTerminal = serde_json::from_str(&text)?;
                // Clear and render the virtual terminal to stdout
                print!("\x1B[2J\x1B[1;1H"); // Clear
                println!("--- Collaborative View: {}x{} ---", term.width, term.height);
                // Simple character-based render
                for y in 0..term.height {
                    for x in 0..term.width {
                        let idx = (y * term.width + x) as usize;
                        if let Some(cell) = term.cells.get(idx) {
                            print!("{}", cell.char);
                        }
                    }
                    println!();
                }
            }
        }
    }
    Ok(())
}
