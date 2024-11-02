use futures_util::StreamExt;
use tokio;
use chrono::{DateTime, Utc};
use embedded_can::Frame as EmbeddedFrame;
use influxdb::{Client, InfluxDbWriteable};
use socketcan::{tokio::CanSocket, Id, StandardId, Result};

#[derive(InfluxDbWriteable)]
struct PackReading1 {
    time: DateTime<Utc>,
    current: i16,
    inst_voltage: i16,
}

impl PackReading1 {
    const ID: u16 = 0x03B;
    const SIZE: usize = 4;
}

struct PackReading2 {
    time: DateTime<Utc>,
    pack_dlc: u8,
    ccl: u8,
    simulated_soc: u8,
    high_temp: u8,
    low_temp: u8,
}
impl PackReading2 {
    const ID: u16 = 0x3CB;
    const SIZE: usize = 6;
}

struct PackReading3 {
    relay_state: DateTime<Utc>,
    soc: u8,
    resistance: i16,
    open_voltage: i16,
    open_amphours: u8,
}
impl PackReading3 {
    const ID: u16 = 0x6B2;
    const SIZE: usize = 7;
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
        let id = match StandardId::new(PackReading1::ID) {
            Some(id) => Id::Standard(id),
            None => {
                eprintln!("Invalid CAN ID {}", PackReading1::ID);
                continue;
            }
        };
        if frame.id() == id && data.len() >= PackReading1::SIZE {
            let pack_reading = PackReading1 {
            time: Utc::now(),
            current: i16::from_be_bytes([data[0], data[1]]),
            inst_voltage: i16::from_be_bytes([data[2], data[3]])
            };

            println!("Current: {}, Voltage: {}", pack_reading.current, pack_reading.inst_voltage);

            if let Err(e) = client.query(pack_reading.into_query("pack")).await {
                eprintln!("Failed to write to InfluxDB: {}", e);
            }
        }
    }

    Ok(())
}
