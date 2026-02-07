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
    // port: Arc<Mutex<Box<dyn SerialPort>>>,
    latest_rgb: Arc<Mutex<(u8, u8, u8)>>,
}


// Data arrive from frontend
#[derive(Deserialize)]
struct RgbData {
    red:   u8,
    green: u8,
    blue:  u8,
}

#[tokio::main]
async fn main() {

    // Open serial port just one time
    let serial = serialport::new("/dev/ttyUSB0", 9600)
    .timeout(Duration::from_millis(10))
    .open()
    .expect("[  Fail] - No open serial port!");

    let latest_rgb = Arc::new(Mutex::new((0u8, 0u8, 0u8)));

    // Thread send data to Arduino =================
    let sender_port = serial.try_clone().expect("clone failed");
    let rgb_state = latest_rgb.clone();

    std::thread::spawn(move || {
        let mut port = sender_port;
        let mut last_rgb_sent = (255, 255, 255);

        loop {
            let current_rgb = {
                let rgb = rgb_state.lock().unwrap();
                *rgb
            };

            if current_rgb != last_rgb_sent {
                let msg = format!("{},{},{}\n", current_rgb.0, current_rgb.1, current_rgb.2);
                let _ = port.write_all(msg.as_bytes());
                let _ = port.flush();
                last_rgb_sent = current_rgb;
                println!("[  SEND] : Sent to Arduino: {}", msg.trim());
            }

            std::thread::sleep(Duration::from_millis(30));
        }
        
    });

    // Web Server =================
    let state = AppState { latest_rgb };

    let frontend_path = std::fs::canonicalize("../rgb_slider_frontend")
    .expect("[  Fail] : No found frontend directory!");

    let app = Router::new()
        .route("/set_rgb", post(set_rgb))
        .nest_service("/", ServeDir::new("../rgb_slider_frontend"))
        .with_state(state);

    println!("Files server from: {:?}", frontend_path);
    println!("Server on http://localhost:3000");

    axum::serve(
        tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap(),
        app,
    )
    .await
    .unwrap();    
}

// Handler =================
async fn set_rgb(
    State(state): State<AppState>,
    Form(data): Form<RgbData>,
) {
    let mut rgb = state.latest_rgb.lock().unwrap();
    *rgb = (data.red, data.green, data.blue);

    // println!("New RGB: {}, {}, {}", data.red, data.green, data.blue);
}





