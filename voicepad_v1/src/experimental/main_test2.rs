mod audio_buffer;

use audio_buffer::AudioBuffer;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

fn main() {
    println!("[INFO] VoicePad starting...");
    println!("[INFO] Initializing audio subsystem...");

    // ------------------------------------------------------------
    // AUDIO HOST
    // ------------------------------------------------------------

    let host = cpal::default_host();

    println!("[INFO] Audio host: {}", host.id().name());

    // ------------------------------------------------------------
    // INPUT DEVICE
    // ------------------------------------------------------------

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

    // ------------------------------------------------------------
    // INPUT CONFIGURATION
    // ------------------------------------------------------------

    let supported_config = match device.default_input_config() {
        Ok(config) => config,

        Err(error) => {
            println!("[ERROR] Could not get default input configuration:");
            println!("[ERROR] {}", error);
            return;
        }
    };

    println!();
    println!("[INFO] Default input configuration:");

    println!(
        "[INFO]   Channels: {}",
        supported_config.channels()
    );

    println!(
        "[INFO]   Sample rate: {} Hz",
        supported_config.sample_rate()
    );

    println!(
        "[INFO]   Sample format: {:?}",
        supported_config.sample_format()
    );

    println!(
        "[INFO]   Buffer size: {:?}",
        supported_config.buffer_size()
    );

    let sample_rate = supported_config.sample_rate();

    let channels = supported_config.channels();

    let config: cpal::StreamConfig = supported_config.into();

    // ------------------------------------------------------------
    // AUDIO CHANNEL
    // ------------------------------------------------------------

    let (audio_sender, audio_receiver) = mpsc::channel::<Vec<f32>>();

    // ------------------------------------------------------------
    // CALLBACK COUNTER
    // ------------------------------------------------------------

    let buffer_counter = Arc::new(AtomicU64::new(0));

    let callback_counter = Arc::clone(&buffer_counter);

    // ------------------------------------------------------------
    // ERROR CALLBACK
    // ------------------------------------------------------------

    let error_callback = |error| {
        eprintln!("[ERROR] Audio stream error: {}", error);
    };

    // ------------------------------------------------------------
    // BUILD INPUT STREAM
    // ------------------------------------------------------------

    println!();
    println!("[INFO] Building input stream...");

    let stream = match device.build_input_stream(
        config,
        move |data: &[f32], _info| {
            let buffer_number =
                callback_counter.fetch_add(1, Ordering::Relaxed) + 1;

            // ----------------------------------------------------
            // STEREO -> MONO
            // ----------------------------------------------------

            let mono = stereo_to_mono(data);

            if buffer_number == 1 {
                println!("[DEBUG] First audio buffer received");

                println!(
                    "[DEBUG] Stereo samples: {}",
                    data.len()
                );

                println!(
                    "[DEBUG] Mono samples: {}",
                    mono.len()
                );
            }

            // ----------------------------------------------------
            // SEND AUDIO TO MAIN THREAD
            // ----------------------------------------------------

            if let Err(error) = audio_sender.send(mono) {
                eprintln!(
                    "[ERROR] Could not send audio buffer: {}",
                    error
                );
            }

            // ----------------------------------------------------
            // DEBUG RMS
            // ----------------------------------------------------

            if buffer_number % 100 == 0 {
                let rms = calculate_rms(data);

                println!(
                    "[DEBUG] Buffer: {} | Stereo samples: {} | RMS: {:.6}",
                    buffer_number,
                    data.len(),
                    rms
                );
            }
        },
        error_callback,
        None,
    ) {
        Ok(stream) => stream,

        Err(error) => {
            println!("[ERROR] Could not build input stream:");
            println!("[ERROR] {}", error);
            return;
        }
    };

    println!("[INFO] Input stream created successfully.");

    // ------------------------------------------------------------
    // START STREAM
    // ------------------------------------------------------------

    match stream.play() {
        Ok(_) => {
            println!("[INFO] Input stream started.");
        }

        Err(error) => {
            println!("[ERROR] Could not start input stream:");
            println!("[ERROR] {}", error);
            return;
        }
    }

    // ------------------------------------------------------------
    // AUDIO BUFFER
    // ------------------------------------------------------------

    println!();
    println!("[INFO] Listening...");
    println!("[INFO] Speak into the microphone.");
    println!("[INFO] Collecting approximately 1 second of audio.");

    let mut audio_buffer = AudioBuffer::new(sample_rate);

    println!(
        "[DEBUG] AudioBuffer ready: {} Hz",
        audio_buffer.sample_rate()
    );

    // ------------------------------------------------------------
    // RECEIVE AUDIO
    // ------------------------------------------------------------

    for received_audio in audio_receiver {
        audio_buffer.push_samples(&received_audio);

        println!(
            "[DEBUG] AudioBuffer contains {} samples",
            audio_buffer.len()
        );

        // --------------------------------------------------------
        // APPROXIMATELY ONE SECOND
        // --------------------------------------------------------

        if audio_buffer.len() >= sample_rate as usize {
            println!();
            println!(
                "[INFO] Collected approximately 1 second of audio."
            );

            println!(
                "[INFO] Total mono samples: {}",
                audio_buffer.len()
            );

            println!(
                "[INFO] Sample rate: {} Hz",
                audio_buffer.sample_rate()
            );

            println!(
                "[INFO] Channels: {} -> 1",
                channels
            );

            break;
        }
    }

    println!("[INFO] VoicePad capture test finished.");
}

// ================================================================
// CALCULATE RMS
// ================================================================

fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let sum_of_squares: f32 =
        samples.iter()
            .map(|sample| sample * sample)
            .sum();

    (sum_of_squares / samples.len() as f32).sqrt()
}

// ================================================================
// STEREO -> MONO
// ================================================================

fn stereo_to_mono(data: &[f32]) -> Vec<f32> {
    let mut mono = Vec::with_capacity(data.len() / 2);

    for frame in data.chunks_exact(2) {
        let left = frame[0];
        let right = frame[1];

        let sample = (left + right) / 2.0;

        mono.push(sample);
    }

    mono
}