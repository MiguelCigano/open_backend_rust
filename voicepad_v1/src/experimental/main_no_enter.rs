// ============================================================
// Imports
// ============================================================

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};


// ============================================================
// AudioBuffer
// ============================================================

struct AudioBuffer {
    samples: Vec<f32>,
    sample_rate: u32,
}

impl AudioBuffer {

    // --------------------------------------------------------
    // Constructor
    // --------------------------------------------------------

    fn new(sample_rate: u32) -> Self {
        println!(
            "[DEBUG] AudioBuffer created: {} Hz",
            sample_rate
        );

        Self {
            samples: Vec::new(),
            sample_rate,
        }
    }

    // --------------------------------------------------------
    // Add samples
    // --------------------------------------------------------

    fn push_samples(&mut self, samples: &[f32]) {
        self.samples.extend_from_slice(samples);
    }

    // --------------------------------------------------------
    // Number of samples
    // --------------------------------------------------------

    fn len(&self) -> usize {
        self.samples.len()
    }

    // --------------------------------------------------------
    // Save audio as WAV
    // --------------------------------------------------------

    fn save_wav(
        &self,
        filename: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {

        println!();
        println!("[INFO] Saving WAV...");
        println!("[INFO]   File: {}", filename);
        println!(
            "[INFO]   Sample rate: {} Hz",
            self.sample_rate
        );
        println!("[INFO]   Channels: 1");
        println!(
            "[INFO]   Samples: {}",
            self.samples.len()
        );

        // WAV configuration
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: self.sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        // Create WAV file
        let mut writer =
            hound::WavWriter::create(filename, spec)?;

        // Convert f32 [-1.0, 1.0]
        // into signed 16-bit PCM
        for sample in &self.samples {

            let sample_i16 =
                (sample.clamp(-1.0, 1.0)
                    * i16::MAX as f32)
                    as i16;

            writer.write_sample(sample_i16)?;
        }

        // Finish WAV file
        writer.finalize()?;

        println!("[INFO] WAV saved successfully.");

        Ok(())
    }
}


// ============================================================
// Main
// ============================================================

fn main() {

    // --------------------------------------------------------
    // Initialization
    // --------------------------------------------------------

    println!("[INFO] VoicePad starting...");
    println!("[INFO] Initializing audio subsystem...");

    let host = cpal::default_host();

    println!(
        "[INFO] Audio host: {}",
        host.id().name()
    );


    // --------------------------------------------------------
    // Default input device
    // --------------------------------------------------------

    let device = match host.default_input_device() {

        Some(device) => device,

        None => {
            println!(
                "[ERROR] No default input device found."
            );

            return;
        }
    };

    println!("[INFO] Default input device:");

    match device.id() {

        Ok(id) => {
            println!("[INFO]   {}", id);
        }

        Err(error) => {
            println!(
                "[WARN] Could not retrieve device ID: {}",
                error
            );
        }
    }


    // --------------------------------------------------------
    // Audio configuration
    // --------------------------------------------------------

    let supported_config =
        match device.default_input_config() {

            Ok(config) => config,

            Err(error) => {

                println!(
                    "[ERROR] Could not get default input configuration:"
                );

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


    // --------------------------------------------------------
    // Extract sample rate
    // --------------------------------------------------------

    let sample_rate =
        supported_config.sample_rate();


    // --------------------------------------------------------
    // Convert SupportedStreamConfig
    // into StreamConfig
    // --------------------------------------------------------

    let config: cpal::StreamConfig =
        supported_config.into();


    // --------------------------------------------------------
    // Audio buffer
    // --------------------------------------------------------

    let audio_buffer =
        Arc::new(
            Mutex::new(
                AudioBuffer::new(sample_rate)
            )
        );


    // --------------------------------------------------------
    // Clone buffer for CPAL callback
    // --------------------------------------------------------

    let callback_buffer =
        Arc::clone(&audio_buffer);


    // --------------------------------------------------------
    // Build input stream
    // --------------------------------------------------------

    println!();
    println!("[INFO] Building input stream...");

    let stream =
        match device.build_input_stream(

            config,

            move |data: &[f32], _info| {

                // CPAL gives us stereo samples:
                //
                // L R L R L R ...
                //
                // Convert them to:
                //
                // M M M M ...

                let mono =
                    stereo_to_mono(data);


                // Access AudioBuffer
                let mut buffer =
                    callback_buffer
                        .lock()
                        .unwrap();


                // Store samples
                buffer.push_samples(&mono);


                // Debug information
                //
                // Print every ~4096 samples.
                if buffer.len() % 4096 < mono.len() {

                    println!(
                        "[DEBUG] AudioBuffer contains {} samples",
                        buffer.len()
                    );
                }
            },

            |error| {

                eprintln!(
                    "[ERROR] Audio stream error: {}",
                    error
                );
            },

            None,
        ) {

            Ok(stream) => stream,

            Err(error) => {

                println!(
                    "[ERROR] Could not build input stream:"
                );

                println!("[ERROR] {}", error);

                return;
            }
        };


    println!(
        "[INFO] Input stream created successfully."
    );


    // --------------------------------------------------------
    // Start input stream
    // --------------------------------------------------------

    match stream.play() {

        Ok(_) => {

            println!(
                "[INFO] Input stream started."
            );
        }

        Err(error) => {

            println!(
                "[ERROR] Could not start input stream:"
            );

            println!("[ERROR] {}", error);

            return;
        }
    }


    // --------------------------------------------------------
    // Capture
    // --------------------------------------------------------

    println!();
    println!("[INFO] Listening...");
    println!(
        "[INFO] Speak into the microphone."
    );

    println!(
        "[INFO] Collecting approximately 1 second of audio."
    );


    // Capture for approximately one second
    thread::sleep(
        Duration::from_secs(10)
    );


    // --------------------------------------------------------
    // Stop stream
    // --------------------------------------------------------

    drop(stream);


    println!();
    println!(
        "[INFO] Audio capture finished."
    );


    // --------------------------------------------------------
    // Inspect captured audio
    // --------------------------------------------------------

    let buffer =
        audio_buffer
            .lock()
            .unwrap();

    println!(
        "[INFO] Total mono samples: {}",
        buffer.len()
    );

    println!(
        "[INFO] Sample rate: {} Hz",
        buffer.sample_rate
    );

    println!(
        "[INFO] Channels: 2 -> 1"
    );


    // --------------------------------------------------------
    // Save WAV
    // --------------------------------------------------------

    match buffer.save_wav("capture.wav") {

        Ok(_) => {

            println!();
            println!(
                "[INFO] Capture test finished successfully."
            );
        }

        Err(error) => {

            println!(
                "[ERROR] Could not save WAV:"
            );

            println!("[ERROR] {}", error);
        }
    }
}


// ============================================================
// Audio processing helpers
// ============================================================

fn stereo_to_mono(data: &[f32]) -> Vec<f32> {

    let mut mono =
        Vec::with_capacity(data.len() / 2);


    // CPAL gives us:
    //
    // [L, R, L, R, L, R, ...]
    //
    // We convert it to:
    //
    // [M, M, M, ...]
    //
    // where:
    //
    // M = (L + R) / 2

    for frame in data.chunks_exact(2) {

        let left = frame[0];
        let right = frame[1];

        let sample =
            (left + right) / 2.0;

        mono.push(sample);
    }

    mono
}