use axum::{
    extract::{Form, State},
    routing::post,
    Router,
};

use serde::Deserialize;
use std::{
    io::{Read, Write},
    sync::{Arc, Mutex},
    time::Duration,
};

use tower_http::services::ServeDir;

#[derive(Clone)]
struct AppState {
    latest_value: Arc<Mutex<u8>>,
}

#[derive(Deserialize)]
struct SliderData {
    value: u8,
}

#[tokio::main]
async fn main() {
    // Open the serial port just one time
    let serial = serialport::new("/dev/ttyUSB1", 9600)
        .timeout(Duration::from_millis(10))
        .open()
        .expect("[Fail] - No open serial port!");

    let latest_value = Arc::new(Mutex::new(0u8));

    // Thread send data to Arduino =================
    let sender_port = serial.try_clone().expect("clone failed");
    let value_ref = latest_value.clone();

    std::thread::spawn(move || {
        let mut port = sender_port;
        let mut last_sent = 255;

        loop {
            let val = *value_ref.lock().unwrap();

            if val != last_sent {
                let msg = format!("{}\n", val);
                let _ = port.write_all(msg.as_bytes());
                let _ = port.flush();
                last_sent = val;
                println!("[SEND] : Sent to Arduino: {}", val);
            }

            std::thread::sleep(Duration::from_millis(30));
        }
    });

    // Thread read data from Arduino =================
    std::thread::spawn(move || {
        let mut port = serial;
        let mut buf = [0u8; 128];

        loop {
            match port.read(&mut buf) {
                Ok(n) if n > 0 => {
                    let s = String::from_utf8_lossy(&buf[..n]);
                    println!("[READ] : Arduino says: {}", s.trim());
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(e) => eprintln!("[Fail] : Serial read error: {:?}", e),
                _=> {}
            }
        }
    });

    // Web Server =================
    let state = AppState { latest_value };

    let app = Router::new()
        .route("/set", post(set_value))
        .nest_service("/", ServeDir::new("../slider_control_fronted"))
        .with_state(state);

    println!("Server on http://localhost:3000");

    axum::serve(
        tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap(),
        app,
    )
    .await
    .unwrap();
}

// Handler =================
async fn set_value(
    State(state): State<AppState>,
    Form(data): Form<SliderData>,
) {
    let mut val = state.latest_value.lock().unwrap();
    *val = data.value;
}
