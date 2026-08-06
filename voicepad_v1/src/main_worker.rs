use std::io::{self, BufRead};
use std::process::Command;
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use hound::{SampleFormat, WavSpec, WavWriter};

const SAMPLE_RATE: u32 = 48_000;

const WHISPER_BIN: &str = "../whisper.cpp/build/bin/whisper-cli";
const WHISPER_MODEL: &str = "../whisper.cpp/models/ggml-base.bin";
const WAV_FILE: &str = "capture.wav";

fn main() {
    println!("[INFO] VoicePad starting...");
    println!("[INFO] Initializing audio subsystem...");

    // ============================================================
    // AUDIO HOST
    // ============================================================

    let host = cpal::default_host();

    println!("[INFO] Audio host: {}", host.id().name());

    // ============================================================
    // INPUT DEVICE
    // ============================================================

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

    // ============================================================
    // INPUT CONFIGURATION
    // ============================================================

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

    // ============================================================
    // SHARED AUDIO BUFFER
    // ============================================================

    let audio_buffer: Arc<Mutex<Vec<f32>>> =
        Arc::new(Mutex::new(Vec::new()));

    let callback_buffer = Arc::clone(&audio_buffer);

    // ============================================================
    // BUILD INPUT STREAM
    // ============================================================

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

    // ============================================================
    // START STREAM
    // ============================================================

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

    // ============================================================
    // USER INTERFACE
    // ============================================================

    println!();
    println!("=================================================");
    println!("                 VoicePad");
    println!("=================================================");
    println!();

    println!("[INFO] Press ENTER to start recording.");
    println!("[INFO] Press ENTER again to stop recording.");
    println!();

    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();

    // ============================================================
    // WAIT FOR FIRST ENTER
    // ============================================================

    print!("> ");

    if let Some(Ok(_)) = lines.next() {
        println!();

        println!("[INFO] Recording started.");
        println!("[INFO] Speak freely.");
        println!("[INFO] Pauses are allowed.");
        println!("[INFO] Press ENTER when finished.");
        println!();

        // Clear previous audio
        {
            let mut buffer = audio_buffer.lock().unwrap();

            buffer.clear();
        }
    } else {
        println!("[ERROR] Could not read keyboard input.");
        return;
    }

    // ============================================================
    // WAIT FOR SECOND ENTER
    // ============================================================

    print!("> ");

    if let Some(Ok(_)) = lines.next() {
        println!();

        println!("[INFO] Recording stopped.");
    } else {
        println!("[ERROR] Could not read keyboard input.");
        return;
    }

    // ============================================================
    // COPY AUDIO FROM BUFFER
    // ============================================================

    let samples = {
        let buffer = audio_buffer.lock().unwrap();

        buffer.clone()
    };

    println!();
    println!("[INFO] Audio capture finished.");

    println!(
        "[INFO] Total mono samples: {}",
        samples.len()
    );

    println!(
        "[INFO] Sample rate: {} Hz",
        SAMPLE_RATE
    );

    println!(
        "[INFO] Channels: {} -> 1",
        channels
    );

    // ============================================================
    // SAVE WAV
    // ============================================================

    println!();
    println!("[INFO] Saving WAV...");
    println!("[INFO] File: {}", WAV_FILE);

    match save_wav(
        WAV_FILE,
        &samples,
        SAMPLE_RATE,
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

    // ============================================================
    // RUN WHISPER
    // ============================================================

    println!();
    println!("=================================================");
    println!("                 Whisper");
    println!("=================================================");
    println!();

    println!("[INFO] Whisper executable:");
    println!("[INFO]   {}", WHISPER_BIN);

    println!("[INFO] Whisper model:");
    println!("[INFO]   {}", WHISPER_MODEL);

    println!();
    println!("[INFO] Transcribing...");
    println!();

    let status = Command::new(WHISPER_BIN)
        .arg("-m")
        .arg(WHISPER_MODEL)
        .arg("-f")
        .arg(WAV_FILE)
        .arg("-l")
        .arg("es")
        .status();

    match status {
        Ok(status) => {
            if status.success() {
                println!();
                println!("[INFO] Whisper finished successfully.");
            } else {
                println!();
                println!(
                    "[ERROR] Whisper exited with status: {}",
                    status
                );
            }
        }

        Err(error) => {
            println!();
            println!("[ERROR] Could not execute Whisper:");
            println!("[ERROR] {}", error);
        }
    }

    println!();
    println!("[INFO] VoicePad finished.");
}

// ================================================================
// CONVERT AUDIO TO MONO
// ================================================================

fn stereo_to_mono(
    data: &[f32],
    channels: u16,
) -> Vec<f32> {
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
// SAVE WAV
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

    let mut writer = WavWriter::create(
        filename,
        spec,
    )?;

    for &sample in samples {
        let sample = sample.clamp(-1.0, 1.0);

        let sample_i16 =
            (sample * i16::MAX as f32) as i16;

        writer.write_sample(sample_i16)?;
    }

    writer.finalize()?;

    Ok(())
}