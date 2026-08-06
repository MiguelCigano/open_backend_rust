use std::io::{self, BufRead};
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use hound::{SampleFormat, WavSpec, WavWriter};

const TARGET_SAMPLE_RATE: u32 = 48_000;

fn main() {
    println!("[INFO] VoicePad starting...");
    println!("[INFO] Initializing audio subsystem...");

    // ------------------------------------------------------------
    // Audio host
    // ------------------------------------------------------------

    let host = cpal::default_host();

    println!("[INFO] Audio host: {}", host.id().name());

    // ------------------------------------------------------------
    // Input device
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
    // Input configuration
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

    let channels = supported_config.channels();

    let config: cpal::StreamConfig = supported_config.into();

    // ------------------------------------------------------------
    // Shared audio buffer
    // ------------------------------------------------------------

    let audio_buffer: Arc<Mutex<Vec<f32>>> =
        Arc::new(Mutex::new(Vec::new()));

    let callback_buffer = Arc::clone(&audio_buffer);

    // ------------------------------------------------------------
    // Build input stream
    // ------------------------------------------------------------

    println!();
    println!("[INFO] Building input stream...");

    let error_callback = |error| {
        eprintln!("[ERROR] Audio stream error: {}", error);
    };

    let stream = match device.build_input_stream(
        config,
        move |data: &[f32], _info| {
            let mono = stereo_to_mono(data, channels);

            let mut buffer = match callback_buffer.lock() {
                Ok(buffer) => buffer,

                Err(error) => {
                    eprintln!(
                        "[ERROR] Audio buffer mutex poisoned: {}",
                        error
                    );
                    return;
                }
            };

            buffer.extend_from_slice(&mono);
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
    // Start stream
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

    println!();
    println!();
    println!("[INFO] Press ENTER to start recording.");
    println!("[INFO] Press ENTER again to stop recording.");
    println!();

    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();

    // ------------------------------------------------------------
    // Wait for first ENTER
    // ------------------------------------------------------------

    print!("> ");

    if let Some(Ok(_)) = lines.next() {
        println!();
        println!("[INFO] Recording started.");
        println!("[INFO] Speak freely.");
        println!("[INFO] Pauses are allowed.");
        println!("[INFO] Press ENTER when finished.");
        println!();

        // Clear any previous audio
        {
            let mut buffer = audio_buffer.lock().unwrap();
            buffer.clear();
        }
    } else {
        println!("[ERROR] Could not read keyboard input.");
        return;
    }

    // ------------------------------------------------------------
    // Wait for second ENTER
    // ------------------------------------------------------------

    print!("> ");

    if let Some(Ok(_)) = lines.next() {
        println!();
        println!("[INFO] Recording stopped.");
    } else {
        println!("[ERROR] Could not read keyboard input.");
        return;
    }

    // ------------------------------------------------------------
    // Copy audio from shared buffer
    // ------------------------------------------------------------

    let samples = {
        let buffer = audio_buffer.lock().unwrap();

        buffer.clone()
    };

    println!();
    println!("[INFO] Audio capture finished.");
    println!("[INFO] Total mono samples: {}", samples.len());
    println!("[INFO] Sample rate: {} Hz", TARGET_SAMPLE_RATE);
    println!("[INFO] Channels: {} -> 1", channels);

    // ------------------------------------------------------------
    // Save WAV
    // ------------------------------------------------------------

    println!();
    println!("[INFO] Saving WAV...");
    println!("[INFO] File: capture.wav");

    match save_wav(
        "capture.wav",
        &samples,
        TARGET_SAMPLE_RATE,
    ) {
        Ok(_) => {
            println!("[INFO] WAV saved successfully.");
        }

        Err(error) => {
            println!("[ERROR] Could not save WAV:");
            println!("[ERROR] {}", error);
            return;
        }
    }

    println!();
    println!("[INFO] VoicePad capture finished successfully.");
}

// ================================================================
// Convert input to mono
// ================================================================

fn stereo_to_mono(data: &[f32], channels: u16) -> Vec<f32> {
    if channels == 1 {
        return data.to_vec();
    }

    let channels = channels as usize;

    let frames = data.len() / channels;

    let mut mono = Vec::with_capacity(frames);

    for frame in data.chunks_exact(channels) {
        let sum: f32 = frame.iter().sum();

        let sample = sum / channels as f32;

        mono.push(sample);
    }

    mono
}

// ================================================================
// Save WAV
// ================================================================

fn save_wav(
    filename: &str,
    samples: &[f32],
    sample_rate: u32,
) -> Result<(), hound::Error> {
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let mut writer = WavWriter::create(filename, spec)?;

    for &sample in samples {
        let sample = sample.clamp(-1.0, 1.0);

        let sample_i16 =
            (sample * i16::MAX as f32) as i16;

        writer.write_sample(sample_i16)?;
    }

    writer.finalize()?;

    Ok(())
}