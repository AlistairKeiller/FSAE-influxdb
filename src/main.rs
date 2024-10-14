use std::time::Duration;
use socketcan::{CanSocket, Socket};
use embedded_can::Frame as EmbeddedFrame;

fn main() {
    // change to can0 once we have hardware
    let sock = loop {
        match CanSocket::open("vcan0") {
            Ok(socket) => break socket,
            Err(_) => {
                println!("Failed to open socket, retrying...");
            }
        }
    };

    loop {
        if let Ok(frame) = sock.read_frame_timeout(Duration::from_millis(100)) {
            println!("{:?}", frame.data());
        }
    }
}
