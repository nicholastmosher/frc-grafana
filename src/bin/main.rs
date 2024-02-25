use std::{net::SocketAddr, time::Duration};

use anyhow::{Context, Result};
use clap::Parser;
use nt::NetworkTables;
use rumqttc::{AsyncClient, MqttOptions};

#[derive(Debug, Parser)]
struct Cli {
    #[clap(long, alias = "robot")]
    address: SocketAddr,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let address = cli.address;
    let mut nt_client = NetworkTables::connect(&address.to_string(), "grafana-mqtt")
        .await
        .with_context(|| format!("failed to connect to NetworkTables at address {address}"))?;

    let mut mqttoptions = MqttOptions::new("rumqtt-async", "localhost", 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let (_mqtt_client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
    tokio::spawn(async move {
        let event = eventloop.poll().await;
        println!("Mqtt event: {event:?}");
    });

    nt_client.add_callback(nt::CallbackType::Add, move |entry| {
        println!("Add Entry: {entry:?}");
        // mqtt_client.publish(entry.name, QosQoS::AtLeastOnce, true);
    });

    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}
