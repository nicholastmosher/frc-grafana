use std::{net::SocketAddr, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use futures::Stream;
use nt::{Client, ConnectionCallbackType, NetworkTables};
use tokio::{
    sync::{mpsc, watch},
    task::JoinHandle,
};

pub struct NetworkTablesContext {
    client: NetworkTables<Client>,
}

pub struct NetworkTablesHandle {
    handle: JoinHandle<()>,
}

async fn init_network_tables_task(address: &str) -> Result<()> {
    // let client = NetworkTables::connect("10.20.79.2:1735", "grafana-mqtt").await?;
    let client = NetworkTables::connect(address, "grafana-mqtt")
        .await
        .with_context(|| format!("failed to connect to NetworkTables at address {address}"))?;
    spawn_network_tables_task(client).await?;
    Ok(())
}

async fn spawn_network_tables_task(client: NetworkTables<Client>) -> Result<NetworkTablesHandle> {
    let task_context = NetworkTablesContext { client };
    let task_future = network_tables_loop(task_context);
    let handle = tokio::spawn(task_future);
    let task_handle = NetworkTablesHandle { handle };
    Ok(task_handle)
}

async fn network_tables_loop(task_context: NetworkTablesContext) {
    loop {
        let future = async {
            try_network_tables_loop(&task_context).await?;
            Ok::<_, anyhow::Error>(())
        };

        let result = future.await;
        match result {
            Ok(()) => return,
            Err(error) => {
                tracing::error!(
                    %error,
                    "NetworkTables client error, retrying",
                );
            }
        }
    }
}

/// Events that this task can react to
pub enum NtClientInput {
    /// The NetworkTables client connected to the target
    ClientConnected(SocketAddr),

    /// The NetworkTables client disconnected from the target
    ClientDisconnected(SocketAddr),
}

fn try_network_tables_stream(
    task_context: &NetworkTablesContext,
) -> Result<impl Stream<Item = NtClientInput>> {
    let (client_tx, client_rx) = mpsc::channel(1);
    let client_tx = Arc::new(client_tx);

    // Configure "Connected" callback events
    task_context.client.add_connection_callback(
        ConnectionCallbackType::ClientConnected,
        move |addr| {
            let client_tx = client_tx.clone();
            client_tx.send()
        },
    );

    // Configure "Disconnected" callback events
    task_context.client.add_connection_callback(
        ConnectionCallbackType::ClientDisconnected,
        |addr| {
            client_tx.send_replace(NtClientInput::ClientDisconnected(addr.clone()));
        },
    );

    Ok(futures::stream::empty())
}

async fn try_network_tables_loop(task_context: &NetworkTablesContext) -> Result<()> {
    let client = &task_context.client;
    client.add_connection_callback(ConnectionCallbackType::ClientDisconnected, |_| {
        println!("Client has disconnected from the server");
    });

    println!("Listing entries");
    for (id, data) in client.entries() {
        println!("{} => {:?}", id, data);
    }

    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
    }

    Ok(())
}
