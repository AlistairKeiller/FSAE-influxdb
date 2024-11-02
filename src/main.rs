use futures_util::StreamExt;
use tokio;
use chrono::{DateTime, Utc};
use embedded_can::Frame as EmbeddedFrame;
use influxdb::{Client, InfluxDbWriteable};
use socketcan::{tokio::CanSocket, Id, StandardId, Result};

#[derive(InfluxDbWriteable)]
struct PackReading {
    time: DateTime<Utc>,
    current: i16,
    voltage: i16,
}

impl PackReading {
    const ID: u16 = 0x03B;
    const SIZE: usize = 4;
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut sock = loop {
        match CanSocket::open("can0") {
            Ok(socket) => break socket,
            Err(_) => {
                eprintln!("Failed to open socket, retrying...");
            }
        }
    };

    let client = Client::new("http://localhost:8086", "test");

    while let Some(Ok(frame)) = sock.next().await {
        let data = frame.data();
        let id = match StandardId::new(PackReading::ID) {
            Some(id) => Id::Standard(id),
            None => {
                eprintln!("Invalid CAN ID {}", PackReading::ID);
                continue;
            }
        };
        if frame.id() == id && data.len() >= PackReading::SIZE {
            let pack_reading = PackReading {
            time: Utc::now(),
            current: i16::from_be_bytes([data[0], data[1]]),
            voltage: i16::from_be_bytes([data[2], data[3]])
            };

            println!("Current: {}, Voltage: {}", pack_reading.current, pack_reading.voltage);

            if let Err(e) = client.query(pack_reading.into_query("pack")).await {
                eprintln!("Failed to write to InfluxDB: {}", e);
            }
        }
    }

    Ok(())
}
