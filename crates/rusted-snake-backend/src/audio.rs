use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioSource {
    pub id: String,
    pub label: String,
}

/// Sources offered by PipeWire/PulseAudio, including monitor sources for what is playing.
pub fn sources() -> Vec<AudioSource> {
    let output = match Command::new("pactl").args(["list", "sources"]).output() {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).to_string()
        }
        _ => return Vec::new(),
    };
    let mut sources = Vec::new();
    let mut name: Option<String> = None;
    let mut description: Option<String> = None;
    for line in output.lines().map(str::trim) {
        if let Some(value) = line.strip_prefix("Name:") {
            name = Some(value.trim().to_string());
        } else if let Some(value) = line.strip_prefix("Description:") {
            description = Some(value.trim().to_string());
        } else if line.is_empty()
            && let (Some(id), Some(label)) = (name.take(), description.take())
        {
            sources.push(AudioSource { id, label });
        }
    }
    if let (Some(id), Some(label)) = (name.take(), description.take()) {
        sources.push(AudioSource { id, label });
    }
    sources.sort_by(|a, b| a.label.cmp(&b.label));
    sources
}

/// The monitor source for what is playing on the default output.
pub fn default_monitor() -> Option<String> {
    let output = Command::new("pactl")
        .args(["get-default-sink"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let sink = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if sink.is_empty() {
        None
    } else {
        Some(format!("{sink}.monitor"))
    }
}

pub struct LevelMeter {
    level: Arc<AtomicU32>,
    stop: Arc<AtomicBool>,
    child: Arc<Mutex<Option<std::process::Child>>>,
    thread: Option<thread::JoinHandle<()>>,
    error: Arc<Mutex<Option<String>>>,
}

impl LevelMeter {
    pub fn start(source: Option<String>, gain: f32) -> Self {
        let level = Arc::new(AtomicU32::new(0f32.to_bits()));
        let stop = Arc::new(AtomicBool::new(false));
        let child = Arc::new(Mutex::new(None));
        let error = Arc::new(Mutex::new(None));
        let thread = {
            let level = level.clone();
            let stop = stop.clone();
            let child = child.clone();
            let error = error.clone();
            thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    let mut command = Command::new("parec");
                    command.args([
                        "--format=float32le",
                        "--rate=44100",
                        "--channels=1",
                        "--latency-msec=20",
                        "--raw",
                    ]);
                    if let Some(source) = source.as_deref().filter(|source| !source.is_empty()) {
                        command.arg(format!("--device={source}"));
                    }
                    command.stdout(Stdio::piped()).stderr(Stdio::null());
                    let mut process = match command.spawn() {
                        Ok(process) => process,
                        Err(spawn_error) => {
                            *error_lock(&error) = Some(match spawn_error.kind() {
                                std::io::ErrorKind::NotFound => {
                                    "parec is not installed. Install pulseaudio-utils or pipewire-pulse.".into()
                                }
                                _ => format!("cannot start parec: {spawn_error}"),
                            });
                            wait_stopped(&stop, Duration::from_secs(2));
                            continue;
                        }
                    };
                    let stdout = process.stdout.take();
                    *child.lock().unwrap() = Some(process);
                    *error_lock(&error) = None;
                    if let Some(mut stdout) = stdout {
                        let mut buffer = [0u8; 4096];
                        let mut smoothed = 0f32;
                        loop {
                            if stop.load(Ordering::Relaxed) {
                                break;
                            }
                            match stdout.read(&mut buffer) {
                                Ok(0) => break,
                                Ok(read) => {
                                    let samples = buffer[..read].chunks_exact(4).map(|chunk| {
                                        f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
                                    });
                                    let mut sum = 0f32;
                                    let mut count = 0usize;
                                    for sample in samples {
                                        sum += sample * sample;
                                        count += 1;
                                    }
                                    if count > 0 {
                                        let rms = (sum / count as f32).sqrt();
                                        let target = (rms * gain * 6.0).clamp(0.0, 1.0);
                                        smoothed = smoothed * 0.62 + target * 0.38;
                                        level.store(smoothed.to_bits(), Ordering::Relaxed);
                                    }
                                }
                                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {
                                    continue;
                                }
                                Err(_) => break,
                            }
                        }
                    }
                    if !stop.load(Ordering::Relaxed) {
                        *error_lock(&error) =
                            Some("Audio capture stopped. Check the selected source.".into());
                    }
                    if let Some(mut process) = child.lock().unwrap().take() {
                        let _ = process.kill();
                        let _ = process.wait();
                    }
                    level.store(0f32.to_bits(), Ordering::Relaxed);
                    if !stop.load(Ordering::Relaxed) {
                        wait_stopped(&stop, Duration::from_millis(700));
                    }
                }
            })
        };
        Self {
            level,
            stop,
            child,
            thread: Some(thread),
            error,
        }
    }

    pub fn level(&self) -> f32 {
        f32::from_bits(self.level.load(Ordering::Relaxed))
    }

    pub fn error(&self) -> Option<String> {
        self.error.lock().unwrap().clone()
    }
}

impl Drop for LevelMeter {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(mut child) = self.child.lock().unwrap().take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn error_lock(lock: &Mutex<Option<String>>) -> std::sync::MutexGuard<'_, Option<String>> {
    lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn wait_stopped(stop: &AtomicBool, total: Duration) {
    let step = Duration::from_millis(50);
    let mut waited = Duration::ZERO;
    while waited < total && !stop.load(Ordering::Relaxed) {
        thread::sleep(step);
        waited += step;
    }
}
