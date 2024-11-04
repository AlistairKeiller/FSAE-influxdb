use chrono::{DateTime, Utc};
use embedded_can::Frame as EmbeddedFrame;
use futures_util::StreamExt;
use influxdb::{Client, InfluxDbWriteable};
use socketcan::{tokio::CanSocket, Id, Result, StandardId};
use tokio;
use tokio_serial::SerialPortBuilderExt;
use tokio::io::{AsyncBufReadExt, BufReader};

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

#[derive(InfluxDbWriteable)]
struct PackReading2 {
    time: DateTime<Utc>,
    dlc: u8,
    ccl: u8,
    simulated_soc: u8,
    high_temp: u8,
    low_temp: u8,
}

impl PackReading2 {
    const ID: u16 = 0x3CB;
    const SIZE: usize = 6;
}

#[derive(InfluxDbWriteable)]
struct PackReading3 {
    time: DateTime<Utc>,
    relay_state: u8,
    soc: u8,
    resistance: i16,
    open_voltage: i16,
    amphours: u8,
}

impl PackReading3 {
    const ID: u16 = 0x6B2;
    const SIZE: usize = 7;
}

#[derive(InfluxDbWriteable)]
struct UARTReading {
    time: DateTime<Utc>,
    brake: u16,
    shock_a: u16,
    shock_b: u16,
}

#[tokio::main]
async fn main() -> Result<()> {

    tokio::spawn(async move {
        let client = Client::new("http://localhost:8086", "data");

        loop {
            match CanSocket::open("can0") {
                Ok(mut sock) => {
                    while let Some(Ok(frame)) = sock.next().await {
                        let data = frame.data();
                        let id = frame.id();

                        // Process PackReading1
                        if let Some(std_id) = StandardId::new(PackReading1::ID) {
                            if id == Id::Standard(std_id) && data.len() >= PackReading1::SIZE {
                                let pack_reading = PackReading1 {
                                    time: Utc::now(),
                                    current: i16::from_be_bytes([data[0], data[1]]),
                                    inst_voltage: i16::from_be_bytes([data[2], data[3]]),
                                };

                                println!(
                                    "Current: {}, Voltage: {}",
                                    pack_reading.current, pack_reading.inst_voltage
                                );

                                if let Err(e) = client.query(pack_reading.into_query("pack")).await {
                                    eprintln!("Failed to write to InfluxDB: {}", e);
                                }
                                continue;
                            }
                        }

                        // Process PackReading2
                        if let Some(std_id) = StandardId::new(PackReading2::ID) {
                            if id == Id::Standard(std_id) && data.len() >= PackReading2::SIZE {
                                let pack_reading = PackReading2 {
                                    time: Utc::now(),
                                    dlc: data[0],
                                    ccl: data[1],
                                    simulated_soc: data[2],
                                    high_temp: data[3],
                                    low_temp: data[4],
                                };

                                println!(
                                    "DLC: {}, CCL: {}, SOC: {}, High Temp: {}, Low Temp: {}",
                                    pack_reading.dlc,
                                    pack_reading.ccl,
                                    pack_reading.simulated_soc,
                                    pack_reading.high_temp,
                                    pack_reading.low_temp
                                );

                                if let Err(e) = client.query(pack_reading.into_query("pack")).await {
                                    eprintln!("Failed to write to InfluxDB: {}", e);
                                }
                                continue;
                            }
                        }

                        // Process PackReading3
                        if let Some(std_id) = StandardId::new(PackReading3::ID) {
                            if id == Id::Standard(std_id) && data.len() >= PackReading3::SIZE {
                                let pack_reading = PackReading3 {
                                    time: Utc::now(),
                                    relay_state: data[0],
                                    soc: data[1],
                                    resistance: i16::from_be_bytes([data[2], data[3]]),
                                    open_voltage: i16::from_be_bytes([data[4], data[5]]),
                                    amphours: data[6],
                                };

                                println!(
                                    "Relay State: {}, SOC: {}, Resistance: {}, Open Voltage: {}, Open Amphours: {}",
                                    pack_reading.relay_state,
                                    pack_reading.soc,
                                    pack_reading.resistance,
                                    pack_reading.open_voltage,
                                    pack_reading.amphours
                                );

                                if let Err(e) = client.query(pack_reading.into_query("pack")).await {
                                    eprintln!("Failed to write to InfluxDB: {}", e);
                                }
                            }
                        }
                    }
                    eprintln!("CAN socket disconnected...");
                }
                Err(_) => {
                    eprintln!("Failed to open CAN socket, retrying...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    });

    tokio::spawn(async move {
        let client = Client::new("http://localhost:8086", "data");
        let port = "/dev/ttyUSB0";
        let baud_rate = 9600;

        loop {
            match tokio_serial::new(port, baud_rate).open_native_async() {
                Ok(serial) => {
                    let reader = BufReader::new(serial);
                    let mut lines = reader.lines();

                    while let Ok(Some(line)) = lines.next_line().await {
                        let parts: Vec<&str> = line.trim().split_whitespace().collect();
                        if parts.len() == 3 {
                            if let (Ok(brake), Ok(shock_a), Ok(shock_b)) = (
                                parts[0].parse::<u16>(),
                                parts[1].parse::<u16>(),
                                parts[2].parse::<u16>(),
                            ) {
                                println!("Brake: {}, ShockA: {}, ShockB: {}", brake, shock_a, shock_b);

                                let reading = UARTReading {
                                    time: Utc::now(),
                                    brake,
                                    shock_a,
                                    shock_b,
                                };

                                if let Err(e) = client.query(reading.into_query("uart")).await {
                                    eprintln!("Failed to write to InfluxDB: {}", e);
                                }
                            } else {
                                eprintln!("Failed to parse integers from line: {}", line);
                            }
                        } else {
                            eprintln!("Invalid data format: {}", line);
                        }
                    }
                    eprintln!("Serial port disconnected...");
                }
                Err(e) => {
                    eprintln!("Failed to open serial port: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    });

    tokio::signal::ctrl_c().await?;

    Ok(())
}
