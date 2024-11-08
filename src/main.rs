use chrono::{DateTime, Utc};
use embedded_can::Frame as EmbeddedFrame;
use futures_util::StreamExt;
use influxdb::{Client, InfluxDbWriteable};
use socketcan::ExtendedId;
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
struct CanReading1 {
    time: DateTime<Utc>,
    speed_rpm: u16,
    motor_current: f32,
    battery_voltage: f32,
    error_code: u16,
}

impl CanReading1 {
    const ID: u32 = 0x0CF11E05;
    const SIZE: usize = 8;
    const NAME: &str = "can_reading1";
}

#[derive(InfluxDbWriteable, Debug)]
struct CanReading2 {
    time: DateTime<Utc>,
    throttle_signal: u8,
    controller_temp: i8,
    motor_temp: i8,
    controller_status: u8,
    switch_status: u8,
}

impl CanReading2 {
    const ID: u32 = 0x0CF11F05;
    const SIZE: usize = 8;
    const NAME: &str = "can_reading2";
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

                        // Process CanReading1
                        if let Some(std_id) = ExtendedId::new(CanReading1::ID) {
                            if id == Id::Extended(std_id) && data.len() >= CanReading1::SIZE {
                                let can_reading_1 = CanReading1 {
                                    time: Utc::now(),
                                    speed_rpm: u16::from_le_bytes([data[0], data[1]]),
                                    motor_current: u16::from_le_bytes([data[2], data[3]]) as f32
                                        * 0.1,
                                    battery_voltage: u16::from_le_bytes([data[4], data[5]]) as f32
                                        * 0.1,
                                    error_code: u16::from_be_bytes([data[6], data[7]]),
                                };

                                println!("{:?}", can_reading_1);

                                if let Err(e) = client
                                    .query(can_reading_1.into_query(CanReading1::NAME))
                                    .await
                                {
                                    eprintln!("Failed to write to InfluxDB: {}", e);
                                }
                            }
                        }

                        // Process CanReading2
                        if let Some(std_id) = ExtendedId::new(CanReading2::ID) {
                            if id == Id::Extended(std_id) && data.len() >= CanReading2::SIZE {
                                let can_reading_2 = CanReading2 {
                                    time: Utc::now(),
                                    throttle_signal: data[0],
                                    controller_temp: data[1] as i8 - 40,
                                    motor_temp: data[2] as i8 - 30,
                                    controller_status: data[5],
                                    switch_status: data[6],
                                };

                                // println!("{:?}", can_reading_2);

                                if let Err(e) = client
                                    .query(can_reading_2.into_query(CanReading2::NAME))
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

#[tokio::test]
async fn add_multiple_packreadings_to_db() {
    let client = influxdb::Client::new("http://localhost:8086", "data");

    for i in 0..125 {
        let reading1 = PackReading1 {
            time: chrono::Utc::now(),
            current: 100 + i as i16,
            inst_voltage: 200 + i as i16,
        };

        let reading2 = PackReading2 {
            time: chrono::Utc::now(),
            dlc: (1 + i % 256) as u8,
            ccl: (2 + i % 256) as u8,
            simulated_soc: (3 + i % 256) as u8,
            high_temp: (4 + i % 256) as u8,
            low_temp: (5 + i % 256) as u8,
        };

        let reading3 = PackReading3 {
            time: chrono::Utc::now(),
            relay_state: (1 + i % 256) as u8,
            soc: (50 + i % 256) as u8,
            resistance: 100 + i as i16,
            open_voltage: 200 + i as i16,
            amphours: (10 + i % 256) as u8,
        };

        client
            .query(reading1.into_query(PackReading1::NAME))
            .await
            .unwrap();
        client
            .query(reading2.into_query(PackReading2::NAME))
            .await
            .unwrap();
        client
            .query(reading3.into_query(PackReading3::NAME))
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(8)).await;
    }
}

#[tokio::test]
async fn add_uart_readings_to_db() {
    let client = influxdb::Client::new("http://localhost:8086", "data");

    for i in 0..125 {
        let reading = UARTReading {
            time: chrono::Utc::now(),
            brake: 1000 + i as u16,
            shock_a: 2000 + i as u16,
            shock_b: 3000 + i as u16,
        };

        client.query(reading.into_query("uart")).await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(8)).await;
    }
}

#[tokio::test]
async fn test_database_backup() {
    let output = tokio::process::Command::new("influxd")
        .args(&["backup", "-portable", "/workspaces/FSAE-influxdb/backup/"])
        .output()
        .await
        .expect("Failed to execute backup command");

    assert!(
        output.status.success(),
        "Backup command failed with status: {}",
        output.status
    );
    if !output.stderr.is_empty() {
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    }
}
