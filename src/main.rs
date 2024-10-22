use chrono::{DateTime, Utc};
use embedded_can::Frame as EmbeddedFrame;
use influxdb::{Client, Error, InfluxDbWriteable};
use socketcan::{CanSocket, Id, Socket, StandardId};
use std::time::Duration;

#[derive(InfluxDbWriteable)]
struct PackReading {
    time: DateTime<Utc>,
    voltage: i16,
    current: i16,
}

// fn crc(data: &[u8]) -> u8 {
//     let mut crc = 0u8;

//     for &byte in data {
//         for i in 0..8 {
//             if ((crc >> 7) ^ (byte >> i) & 0x01) != 0 {
//                 crc = (crc << 1) ^ 0x07;
//             } else {
//                 crc <<= 1;
//             }
//         }
//     }

//     crc
// }

#[tokio::main]
async fn main() -> Result<(), Error> {
    // change to can0 once we have hardware
    let sock = loop {
        match CanSocket::open("can0") {
            Ok(socket) => break socket,
            Err(_) => {
                eprintln!("Failed to open socket, retrying...");
            }
        }
    };

    // update to our db
    let client = Client::new("http://localhost:8086", "test");

    loop {
        if let Ok(frame) = sock.read_frame_timeout(Duration::from_millis(100)) {
            let data = frame.data();
            if let Some(id) = StandardId::new(0x03B) {
                if data.len() >= 5 && frame.id() == Id::Standard(id) {
                    let pack_reading = PackReading {
                        time: Utc::now(),
                        voltage: i16::from_be_bytes([data[0], data[1]]),
                        current: i16::from_be_bytes([data[2], data[3]])
                    };

                    // if crc(&data[..7]) != data[7] {
                    //     eprintln!("CRC mismatch, skipping frame");
                    //     continue;
                    // }

                    print!("Voltage: {}, Current: {}\n", pack_reading.voltage, pack_reading.current);

                    if let Err(e) = client.query(pack_reading.into_query("pack")).await {
                        eprintln!("Failed to write to InfluxDB: {}", e);
                    }
                } else {
                    eprintln!("Received frame with insufficient data length");
                }
            }
        }
    }
}
