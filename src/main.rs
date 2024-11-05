use chrono::{DateTime, Utc};
use embedded_can::Frame as EmbeddedFrame;
use futures_util::StreamExt;
use influxdb::{Client, InfluxDbWriteable};
use socketcan::{tokio::CanSocket, Id, Result, StandardId};
use tokio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_serial::SerialPortBuilderExt;

const INFLUXDB_URL: &str = "http://localhost:8086";
const INFLUXDB_DATABASE: &str = "data";
const CAN_INTERFACE: &str = "can0";
const SERIAL_PORT: &str = "/dev/ttyACM0";
const SERIAL_BAUD_RATE: u32 = 9600;
const BACKUP_INTERVAL_SECS: u64 = 60;
const BACKUP_PATH: &str = "/home/dashpi/influxdbbackup";

#[derive(InfluxDbWriteable, Debug)]
struct PackReading1 {
    time: DateTime<Utc>,
    current: i16,
    inst_voltage: i16,
}

impl PackReading1 {
    const ID: u16 = 0x03B;
    const SIZE: usize = 4;
    const NAME: &str = "pack1";
}

#[derive(InfluxDbWriteable, Debug)]
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
    const NAME: &str = "pack2";
}

#[derive(InfluxDbWriteable, Debug)]
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
    const NAME: &str = "pack3";
}

#[derive(InfluxDbWriteable, Debug)]
struct UARTReading {
    time: DateTime<Utc>,
    brake: u16,
    shock_a: u16,
    shock_b: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    tokio::spawn(async move {
        let client = Client::new(INFLUXDB_URL, INFLUXDB_DATABASE);

        loop {
            match CanSocket::open(CAN_INTERFACE) {
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

                                println!("{:?}", pack_reading);

                                if let Err(e) = client
                                    .query(pack_reading.into_query(PackReading1::NAME))
                                    .await
                                {
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

                                println!("{:?}", pack_reading);

                                if let Err(e) = client
                                    .query(pack_reading.into_query(PackReading2::NAME))
                                    .await
                                {
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

                                println!("{:?}", pack_reading);

                                if let Err(e) = client
                                    .query(pack_reading.into_query(PackReading3::NAME))
                                    .await
                                {
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
        let client = Client::new(INFLUXDB_URL, INFLUXDB_DATABASE);

        loop {
            match tokio_serial::new(SERIAL_PORT, SERIAL_BAUD_RATE).open_native_async() {
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
                                let reading = UARTReading {
                                    time: Utc::now(),
                                    brake,
                                    shock_a,
                                    shock_b,
                                };

                                println!("{:?}", reading);

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

    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(tokio::time::Duration::from_secs(BACKUP_INTERVAL_SECS));
        loop {
            interval.tick().await;
            let output = tokio::process::Command::new("influxd")
                .args(&["backup", "-portable", BACKUP_PATH])
                .output()
                .await;

            match output {
                Ok(output) => {
                    if !output.status.success() {
                        eprintln!("Backup command failed with status: {}", output.status);
                        if !output.stderr.is_empty() {
                            eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
                        }
                    } else {
                        println!("Backup completed successfully");
                    }
                }
                Err(e) => {
                    eprintln!("Failed to execute backup command: {}", e);
                }
            }
        }
    });

    tokio::signal::ctrl_c().await?;

    Ok(())
}
