use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::{Context, Result};
use clap::Parser;
use nt::{EntryValue, NetworkTables};
use rumqttc::{AsyncClient, MqttOptions, QoS};

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
    println!("Got NT");
    let disconnected = Arc::new(AtomicBool::new(false));

    nt_client.add_connection_callback(nt::ConnectionCallbackType::ClientConnected, |addr| {
        println!("NT Connected {addr}");
    });

    let dc = disconnected.clone();
    nt_client.add_connection_callback(
        nt::ConnectionCallbackType::ClientDisconnected,
        move |addr| {
            println!("NT Disconnected {addr}");
            dc.store(true, Ordering::Relaxed);
        },
    );

    nt_client.add_callback(nt::CallbackType::Add, move |entry| {
        println!("Add Entry: {entry:?}");
    });

    nt_client.add_callback(nt::CallbackType::Update, move |entry| {
        println!("Update Entry: {entry:?}");
    });

    let mut mqttoptions = MqttOptions::new("rumqtt-async", "localhost", 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let (mqtt_client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
    let mqtt_client = Arc::new(mqtt_client);
    tokio::spawn(async move {
        loop {
            let event = eventloop.poll().await;
            // println!("Mqtt event: {event:?}");
        }
    });

    let mut up = true;
    let mut number = 0f32;
    loop {
        if disconnected.load(Ordering::Relaxed) {
            println!("Reconnecting");
            nt_client.reconnect().await;
            println!("Did reconnecting");
        }

        for entry in nt_client.entries().values() {
            let key = entry.name.replace(" ", "");
            let key = key.replace("/", "");
            // println!("Entry: {entry:?}");
            let EntryValue::Double(number) = entry.value else {
                continue;
            };
            mqtt_client
                .publish(&key, QoS::AtLeastOnce, true, number.to_string())
                .await
                .context("failed to publish")?;
            // println!("Published {} -> {}", key, number.to_string());
        }

        mqtt_client
            .publish("angle", QoS::AtLeastOnce, true, number.to_string())
            .await
            .context("failed to publish")?;

        mqtt_client
            .publish(
                "double_angle",
                QoS::AtLeastOnce,
                true,
                (number * 2.).to_string(),
            )
            .await
            .context("failed to publish")?;

        // println!("Published SmartDashboard/Angle->{number}");

        if up {
            number += 0.5;
        } else {
            number -= 0.5;
        }

        if number > 200. {
            up = false;
        } else if number < 100. {
            up = true;
        }

        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    // loop {
    //     tokio::time::sleep(Duration::from_millis(20)).await;
    // }
}
