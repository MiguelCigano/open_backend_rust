use axum::{
    extract::{Form, State},
    routing::{get, post},
    Json, Router,
};

use serde::{
    Deserialize,
    Serialize
};

use std::{
    io::{Read, Write, BufRead, BufReader},
    sync::{Arc, Mutex},
    time::Duration,
};

use tower_http::services::ServeDir;
use serialport::SerialPort;

#[derive(Clone)]
struct AppState {
    // Guarda el string que viene de Arduino: Color 1, Color 2, Boton
    traffic_data: Arc<Mutex<String>>,
    // Enviar comando al arduino (web_stop)
    serial_sender: Arc<Mutex<Box<dyn serialport::SerialPort>>>,
}

// Estructura para responder al Frontend en formato JSON
#[derive(Serialize)]
struct TrafficResponse {
    s1: String,
    s2: String,
    btn_state: String,
}

#[derive(Deserialize)]
struct EmergencyCommand {
    stop: bool,
}

#[tokio::main]
async fn main() {
    // 1. Set Serial Port 
    let serial_port = serialport::new("/dev/ttyUSB0", 9600)
        .timeout(Duration::from_millis(100))
        .open()
        .expect("[  Fail] - No open serial port");

    let traffic_data = Arc::new(Mutex::new("red,red,ok".to_string()));
    let serial_sender = Arc::new(Mutex::new(serial_port.try_clone().expect("Error!")));
    
    // 2. Thread reader (Read data from Arduino into Backend)
    let reader_port = serial_port;
    let read_state = traffic_data.clone();

    std::thread::spawn(move || {
        let mut reader = BufReader::new(reader_port);
        let mut line = String::new();
        loop {
            if reader.read_line(&mut line).is_ok() {
                let trimmed = line.trim().to_string();
                if !trimmed.is_empty() {
                    let mut val = read_state.lock().unwrap();
                    println!("[  READ] : Arduino says: {}", trimmed);
                    *val = trimmed;
                }
                line.clear();
            }
            std::thread::sleep(Duration::from_millis(30));
        }
    });

    // 3. ================= Axum Web Server =================
    let state = AppState { traffic_data,serial_sender };

    let fronted_path = std::fs::canonicalize("../semaforo_doble_frontend")
        .expect("[  Fail] : No found frontend directory!");
    
    let app = Router::new()
        .route("/get_value", get(get_traffic_light))
        .route("/set_emergency", post(set_emergency))
        .nest_service("/", ServeDir::new(fronted_path))
        .with_state(state);

    println!("Files server from: {:?}", "../semaforo_doble_frontend");
    println!("Server on http:...");

    axum::serve(
        tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap(),
        app,
    ).await.unwrap();

}


// Handler: Send semapho state into frontend
async fn get_traffic_light(State(state): State<AppState>) 
    -> Json<TrafficResponse> 
{
    let raw = state.traffic_data.lock().unwrap();
    let parts: Vec<&str> = raw.split(',').collect();

    Json(TrafficResponse {
        s1: parts.get(0).unwrap_or(&"red").to_string(),
        s2: parts.get(1).unwrap_or(&"green").to_string(),
        btn_state: parts.get(2).unwrap_or(&"OK").to_string(),
    })
}

// Handler: Get Web Command and send into to Arduino
async fn set_emergency(State(state): State<AppState>, 
                    Json(cmd): Json<EmergencyCommand>)
{
    if cmd.stop {
        let mut ser_port = state.serial_sender.lock().unwrap();
        let _ = ser_port.write_all(b"STOP\n");
        println!("[ WEB] Stop emergency from arduino");
    }
}