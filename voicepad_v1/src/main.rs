use std::sync::{Arc, Mutex};
use std::process::Command;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use eframe::egui;
use hound::{SampleFormat, WavSpec, WavWriter};


// CONFIGURATION
const SAMPLE_RATE: u32 = 48_000;
const WHISPER_BIN: &str = "../whisper.cpp/build/bin/whisper-cli";
// const WHISPER_MODEL: &str = "../whisper.cpp/models/ggml-base.bin";
const WHISPER_MODEL: &str = "../whisper.cpp/models/ggml-small.bin";
const WAV_FILE: &str = "capture.wav";


// APPLICATION STATE
struct VoicePadApp {
    audio_buffer: Arc<Mutex<Vec<f32>>>,
    channels: u16,

    // Keep the audio stream alive for the entire application.
    _stream: cpal::Stream,
    recording: bool,
    text: String,
    status: String,
}

fn main() -> eframe::Result<()> {
    println!("[INFO] Starting VoicePad GUI...");
    println!("[INFO] Initializing audio subsystem...");

    // AUDIO HOST
    let host = cpal::default_host();
    println!("[INFO] Audio host: {}", host.id().name());

    // INPUT DEVICE
    let device = match host.default_input_device() {
        Some(device) => device,

        None => {
            eprintln!("[ERROR] No default input device found.");
            return Ok(());
        }
    };

    println!("[INFO] Default input device:");

    match device.id() {
        Ok(id) => {
            println!("[INFO]   {}", id);
        }

        Err(error) => {
            println!("[WARN] Could not retrieve device ID: {}", error);
        }
    }

    // INPUT CONFIGURATION
    let supported_config = match device.default_input_config() {
        Ok(config) => config,

        Err(error) => {
            eprintln!(
                "[ERROR] Could not get default input configuration: {}",
                error
            );

            return Ok(());
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

    // SHARED AUDIO BUFFER
    let audio_buffer: Arc<Mutex<Vec<f32>>> =
        Arc::new(Mutex::new(Vec::new()));

    let callback_buffer = Arc::clone(&audio_buffer);

    // AUDIO CALLBACK
    let error_callback = |error| {
        eprintln!("[ERROR] Audio stream error: {}", error);
    };

    // BUILD INPUT STREAM
    println!();
    println!("[INFO] Building input stream...");

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
            eprintln!(
                "[ERROR] Could not build input stream: {}",
                error
            );

            return Ok(());
        }
    };

    println!("[INFO] Input stream created successfully.");

    // START AUDIO STREAM
    if let Err(error) = stream.play() {
        eprintln!(
            "[ERROR] Could not start input stream: {}",
            error
        );

        return Ok(());
    }

    println!("[INFO] Input stream started.");

    // CREATE APPLICATION
    let app = VoicePadApp {
        audio_buffer,
        channels,
        _stream: stream,
        recording: false,
        text: String::new(),
        status: "Listo. Presiona Grabar.".to_string(),
    };

    // GUI
    // let options = eframe::NativeOptions::default();

    // eframe::run_native(
    //     "VoicePad",
    //     options,
    //     Box::new(move |_cc| {
    //         Ok(Box::new(app))
    //     }),
    // )
    // GUI

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "VoicePad",
        options,
        Box::new(move |_cc| {
            Ok(Box::new(app))
        }),
    )
}

// GUI IMPLEMENTATION
impl eframe::App for VoicePadApp {
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        _frame: &mut eframe::Frame,
    ) {

        let enter_pressed =
            ui.input(|i| i.key_pressed(egui::Key::Enter));

        let escape_pressed =
            ui.input(|i| i.key_pressed(egui::Key::Escape));
        
        ui.heading("VoicePad");
        ui.separator();

        // STATUS
        ui.label(format!("Estado: {}", self.status));

        ui.separator();

        // RECORD BUTTONS
        ui.horizontal(|ui| {
            // ----------------------------------------------------
            // RECORD
            // ----------------------------------------------------

            // ----------------------------------------------------
            // BUTTONS
            // ----------------------------------------------------
            let start_recording = ui
                .add_enabled(
                    !self.recording,
                    egui::Button::new("Grabar"),
                )
                .clicked();

            let stop_recording = ui
                .add_enabled(
                    self.recording,
                    egui::Button::new("⏹ Detener"),
                )
                .clicked();

            if (start_recording || enter_pressed) && !self.recording {
                println!("[GUI] Grabar presionado");

                // Clear old audio.
                if let Ok(mut buffer) = self.audio_buffer.lock() {
                    buffer.clear();
                }

                self.text.clear();

                self.recording = true;

                self.status =
                    "Grabando... habla libremente.".to_string();

                println!("[INFO] Recording started.");
            }

            // ----------------------------------------------------
            // STOP
            // ----------------------------------------------------

            else if (stop_recording || enter_pressed || escape_pressed) && self.recording {

                println!("[GUI] Detener presionado");

                self.recording = false;

                self.status =
                    "Procesando audio...".to_string();

                println!("[INFO] Recording stopped.");

                // =================================================
                // COPY AUDIO
                // =================================================

                let samples = match self.audio_buffer.lock() {
                    Ok(buffer) => buffer.clone(),

                    Err(error) => {
                        self.status =
                            "Error accediendo al buffer de audio."
                                .to_string();

                        eprintln!(
                            "[ERROR] Audio buffer mutex: {}",
                            error
                        );

                        return;
                    }
                };

                println!(
                    "[INFO] Total mono samples: {}",
                    samples.len()
                );

                // =================================================
                // SAVE WAV
                // =================================================

                println!("[INFO] Saving WAV...");

                match save_wav(
                    WAV_FILE,
                    &samples,
                    SAMPLE_RATE,
                ) {
                    Ok(_) => {
                        println!(
                            "[INFO] WAV saved successfully."
                        );
                    }

                    Err(error) => {
                        self.status =
                            format!("Error guardando WAV: {}", error);

                        eprintln!(
                            "[ERROR] Could not save WAV: {}",
                            error
                        );

                        return;
                    }
                }

                // =================================================
                // RUN WHISPER
                // =================================================

                self.status =
                    "Transcribiendo con Whisper...".to_string();

                println!(
                    "[INFO] Running Whisper..."
                );

                match transcribe() {
                    Ok(transcription) => {
                        println!(
                            "[INFO] Whisper finished successfully."
                        );

                        self.text = transcription;

                        self.status =
                            "Transcripción terminada.".to_string();
                    }

                    Err(error) => {
                        eprintln!(
                            "[ERROR] Whisper failed: {}",
                            error
                        );

                        self.status =
                            format!(
                                "Error ejecutando Whisper: {}",
                                error
                            );
                    }
                }
            }
        });

        ui.separator();

        // ========================================================
        // TEXT AREA
        // ========================================================

        ui.label("Texto:");

        ui.add(
            egui::TextEdit::multiline(&mut self.text)
                .desired_rows(15)
                .desired_width(f32::INFINITY),
        );

        ui.separator();

        // ========================================================
        // COPY BUTTON
        // ========================================================

        if ui
            .button("Copiar texto")
            .clicked()
        {
            println!("[GUI] Texto copiado al portapapeles");

            ui.ctx().copy_text(self.text.clone());

            self.status =
                "Texto copiado al portapapeles.".to_string();
        }
    }
}

// ================================================================
// WHISPER
// ================================================================

fn transcribe() -> Result<String, String> {
    let output = Command::new(WHISPER_BIN)
        .arg("-m")
        .arg(WHISPER_MODEL)
        .arg("-f")
        .arg(WAV_FILE)
        .arg("-l")
        .arg("es")
        .arg("-np")
        .arg("-nt")
        .output()
        .map_err(|error| {
            format!(
                "No se pudo ejecutar Whisper: {}",
                error
            )
        })?;

    if !output.status.success() {
        let stderr =
            String::from_utf8_lossy(&output.stderr);

        return Err(format!(
            "Whisper terminó con error: {}",
            stderr
        ));
    }

    let text =
        String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string();

    Ok(text)
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

    let mut mono =
        Vec::with_capacity(frames);

    for frame in data.chunks_exact(channels) {
        let sum: f32 = frame.iter().sum();

        let sample =
            sum / channels as f32;

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

    let mut writer =
        WavWriter::create(
            filename,
            spec,
        )?;

    for &sample in samples {
        let sample =
            sample.clamp(-1.0, 1.0);

        let sample_i16 =
            (sample * i16::MAX as f32) as i16;

        writer.write_sample(sample_i16)?;
    }

    writer.finalize()?;

    Ok(())
}