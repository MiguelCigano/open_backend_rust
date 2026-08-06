use cpal::traits::{DeviceTrait, HostTrait};

fn main() {
    println!("[INFO] VoicePad starting...");
    println!("[INFO] Initializing audio subsystem...");

    let host = cpal::default_host();

    println!("[INFO] Audio host: {}", host.id().name());

    let device = match host.default_input_device() {
        Some(device) => device,

        None => {
            println!("[ERROR] No default input device found.");
            return;
        }
    };

    println!("[INFO] Default input device:");

    match device.id() {
        Ok(id) => println!("[INFO]   {}", id),

        Err(error) => {
            println!("[WARN] Could not retrieve device ID: {}", error);
        }
    }

    println!();
    println!("[INFO] Querying supported input configurations...");

    println!();
    println!("[INFO] Querying default input configuration...");

    match device.default_input_config() {
        Ok(config) => {
            println!("[INFO] Default input configuration:");

            println!("[INFO]   Channels: {}", config.channels());

            println!(
                "[INFO]   Sample rate: {} Hz",
                config.sample_rate()
            );

            println!(
                "[INFO]   Sample format: {:?}",
                config.sample_format()
            );

            println!(
                "[INFO]   Buffer size: {:?}",
                config.buffer_size()
            );
        }

        Err(error) => {
            println!("[ERROR] Could not query default input configuration:");
            println!("[ERROR] {}", error);
            return;
        }
    }
    println!();
    println!("[INFO] Audio subsystem initialized successfully.");
}