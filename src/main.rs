use std::time::Duration;
use socketcan::{CanSocket, Socket};
use embedded_can::Frame as EmbeddedFrame;
use influxdb::{Client, Error, InfluxDbWriteable, ReadQuery, Timestamp};
use chrono::{DateTime, Utc};



#[derive(InfluxDbWriteable)]
struct PackReading {
    time: DateTime<Utc>,
    voltage: u16,
    current: u16,
    highest_temp: u8,
    avg_temp: u8,
    relay: u8,
    crc: u8
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // change to can0 once we have hardware
    let sock = loop {
        match CanSocket::open("vcan0") {
            Ok(socket) => break socket,
            Err(_) => {
                println!("Failed to open socket, retrying...");
            }
        }
    };

    // update to our db
    let client = Client::new("http://localhost:8086", "test");

    loop {
        if let Ok(frame) = sock.read_frame_timeout(Duration::from_millis(100)) {
            let data = frame.data();
            if data.len() >= 8 {
                let pack_reading = PackReading {
                    time: Utc::now(),
                    voltage: u16::from_be_bytes([data[0], data[1]]),
                    current: u16::from_be_bytes([data[2], data[3]]),
                    highest_temp: data[4],
                    avg_temp: data[5],
                    relay: data[6],
                    crc: data[7],
                };
            

                if let Err(e) = client.query(pack_reading.into_query("weather")).await {
                    eprintln!("Failed to write to InfluxDB: {}", e);
                }
            } else {
                eprintln!("Received frame with insufficient data length");
            }
        }
    }
}
