use axum::{
    extract::{Form, State},
    routing::{get, post},
    Json, Router,
};

use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    sync::{Arc, Mutex},
    time::Duration,
};

use tower_http::services::ServeDir;
// use std::io::{BufRead, BufReader};
use::serialport::SerialPort; // Add to use serialport

#[derive(Clone)]
struct AppState {
    latest_rgb: Arc<Mutex<(u8, u8, u8)>>,
    arduino_value: Arc<Mutex<String>>,
}

// Data arrive from frontend
#[derive(Deserialize)]
struct RgbData {
    red:   u8,
    green: u8,
    blue:  u8,
}

// Estructura para responder al Frontend en formato JSON
#[derive(Serialize)]
struct ArduinoResponse {
    value: String,
}

#[tokio::main]
async fn main() {

    // Open serial port just one time
    let serial = serialport::new("/dev/ttyUSB1", 9600)
    .timeout(Duration::from_millis(100))
    .open()
    .expect("[  Fail] - No open serial port!");

    let latest_rgb = Arc::new(Mutex::new((0u8, 0u8, 0u8)));
    let arduino_value = Arc::new(Mutex::new("0".to_string()));

    // Thread send data to STM32 =================
    let sender_port: Box<dyn SerialPort> = serial.try_clone().expect("clone failed");
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
                println!("[  SEND] : Sent to STM32_: {}", msg.trim());
            }

            std::thread::sleep(Duration::from_millis(30));
        }
        
    });

    // Thread read data to Arduino (Modificado para guardar en el estado)
    let reader_port = serial;
    let read_state = arduino_value.clone();
    std::thread::spawn(move || {
        let mut port = reader_port;
        let mut buf = [0u8; 128];
        loop {
            if let Ok(n) = port.read(&mut buf) {
                if n > 0 {
                    let s = String::from_utf8_lossy(&buf[..n]).trim().to_string();
                    if !s.is_empty() {
                        let mut val = read_state.lock().unwrap();
                        *val = s.clone(); // Guardamos lo que dijo el Arduino
                        // println!("[  READ] : Arduino says: {}", s);
                    }
                }
            }
            // Un pequeño respiro para el CPU
            std::thread::sleep(Duration::from_millis(10));
        }
    });


    // Web Server =================
    let state = AppState { latest_rgb, arduino_value };

    let frontend_path = std::fs::canonicalize("../rgb_slider_frontend")
    .expect("[  Fail] : No found frontend directory!");

    let app = Router::new()
        .route("/set_rgb", post(set_rgb))
        .route("/get_value", get(get_arduino_value)) // Nueva ruta GET
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
) 
{
    let mut rgb = state.latest_rgb.lock().unwrap();
    *rgb = (data.red, data.green, data.blue);

    // println!("New RGB: {}, {}, {}", data.red, data.green, data.blue);
}

// Nuevo Handler para responder al Frontend
async fn get_arduino_value(State(state): State<AppState>) -> Json<ArduinoResponse> {
    let val = state.arduino_value.lock().unwrap();
    Json(ArduinoResponse { value: val.clone() })
}





