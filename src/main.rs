use futures_util::StreamExt;
use tokio;
use chrono::{DateTime, Utc};
use embedded_can::Frame as EmbeddedFrame;
use influxdb::{Client, InfluxDbWriteable};
use socketcan::{tokio::CanSocket, Id, StandardId, Result};

#[derive(InfluxDbWriteable)]
struct PackReading {
    time: DateTime<Utc>,
    voltage: i16,
    current: i16,
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
        if let Some(id) = StandardId::new(0x03B) {
            if data.len() >= 5 && frame.id() == Id::Standard(id) {
                let pack_reading = PackReading {
                    time: Utc::now(),
                    voltage: i16::from_be_bytes([data[0], data[1]]),
                    current: i16::from_be_bytes([data[2], data[3]])
                };

                println!("Voltage: {}, Current: {}", pack_reading.voltage, pack_reading.current);

                if let Err(e) = client.query(pack_reading.into_query("pack")).await {
                    eprintln!("Failed to write to InfluxDB: {}", e);
                }
            } else {
                eprintln!("Received frame with insufficient data length");
            }
        }
    }

    Ok(())
}
