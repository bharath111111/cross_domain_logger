#[cfg(feature = "vxl-can")]
mod vxl_capture;
            // Optional: CAN/ETH capture via vxlapi.dll
            // Example usage (uncomment to test):
            // match vxl_capture::try_open_driver() {
            //     Ok(_) => println!("vxlapi driver opened successfully"),
            //     Err(e) => println!("vxlapi error: {}", e),
            // }
            // vxl_capture::try_close_driver();
use eframe::egui;
use std::process::{Command, Child, Stdio};
use std::fs;
use chrono::Local;
use std::path::PathBuf;
use std::time::Instant;
use std::io::{Read, Seek, SeekFrom};

struct LoggerApp {
    testing_session_name: String,
    qnx_ip: String,
    android_ip: String,
    interfaces: Vec<String>,
    selected_interface: usize,
    status: String,
    qnx_connected: bool,
    android_connected: bool,
    eth_connected: bool,
    can_connected: bool,

    capture_qnx: bool,
    capture_android: bool,
    capture_can: bool,
    capture_eth: bool,

    qnx_process: Option<Child>,
    android_process: Option<Child>,
    eth_process: Option<Child>,
    can_process: Option<Child>,

    log_folder: Option<PathBuf>,

    // UI log viewer
    log_view: usize, // 0 = QNX, 1 = Android, 2 = Ethernet
    log_buffer: String,
    last_log_update: Instant,
    can_channels: Vec<(String, String)>,
    can_channels_status: String,
}

impl LoggerApp {
    fn sanitize_folder_name(name: &str) -> String {
        let sanitized: String = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();

        sanitized.trim_matches('_').to_string()
    }

    fn detect_interfaces() -> Vec<String> {
        if let Ok(output) = Command::new("ifconfig").arg("-l").output() {
            if output.status.success() {
                let s = String::from_utf8_lossy(&output.stdout).to_string();
                return s.split_whitespace().map(|s| s.to_string()).collect();
            }
        }
        vec!["en0".into(), "en1".into(), "eth0".into()]
    }

    fn read_tail(path: &PathBuf, max_bytes: usize) -> String {
        if let Ok(mut f) = fs::File::open(path) {
            if let Ok(size) = f.seek(SeekFrom::End(0)) {
                let start = if size > max_bytes as u64 {
                    size - max_bytes as u64
                } else {
                    0
                };
                let _ = f.seek(SeekFrom::Start(start));
                let mut buf = String::new();
                let _ = f.read_to_string(&mut buf);
                return buf;
            }
        }
        String::new()
    }

    fn check_qnx(&self) -> bool {
        if self.qnx_ip.trim().is_empty() {
            return true;
        }
        let status = Command::new("ssh")
            .arg("-o")
            .arg("BatchMode=yes")
            .arg("-o")
            .arg("ConnectTimeout=5")
            .arg(format!("root@{}", self.qnx_ip))
            .arg("echo ok")
            .status();
        matches!(status, Ok(s) if s.success())
    }

    fn check_android(&self) -> bool {
        if let Ok(out) = Command::new("adb").arg("devices").output() {
            let s = String::from_utf8_lossy(&out.stdout);
            // skip header line
            let mut lines = s.lines().skip(1).map(|l| l.trim()).filter(|l| !l.is_empty());
            if self.android_ip.trim().is_empty() {
                return lines.next().is_some();
            }
            return lines.any(|l| l.contains(&self.android_ip));
        }
        false
    }

    fn check_eth(&self) -> bool {
        self.interfaces.get(self.selected_interface).is_some()
    }

    fn refresh_can_channels(&mut self) {
        let exe = match std::env::current_exe() {
            Ok(path) => path,
            Err(e) => {
                self.can_connected = false;
                self.can_channels.clear();
                self.can_channels_status = format!("Unable to resolve app path: {}", e);
                return;
            }
        };

        let output = Command::new(exe)
            .arg("--test-can")
            .arg("--can-map")
            .arg("--can-app")
            .arg("CANoe")
            .arg("--can-max-channels")
            .arg("64")
            .output();

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let mut mapped_channels: Vec<(String, String)> = Vec::new();
                for line in stdout.lines() {
                    if line.contains("ch=") && line.contains("->") && !line.contains("hwType=0") {
                        let channel = line
                            .split_whitespace()
                            .find(|part| part.starts_with("ch="))
                            .map(|part| part.trim_start_matches("ch=").to_string())
                            .unwrap_or_else(|| "?".to_string());

                        let network = line
                            .rsplit("->")
                            .next()
                            .map(|part| part.trim().to_string())
                            .unwrap_or_else(|| "UNMAPPED".to_string());

                        if network != "UNMAPPED" {
                            mapped_channels.push((channel, network));
                        }
                    }
                }

                if mapped_channels.is_empty() {
                    self.can_connected = false;
                    self.can_channels.clear();
                    self.can_channels_status = "No mapped CAN channels found for app 'CANoe'.".into();
                } else {
                    self.can_connected = true;
                    self.can_channels = mapped_channels;
                    self.can_channels_status.clear();
                }
            }
            Err(e) => {
                self.can_connected = false;
                self.can_channels.clear();
                self.can_channels_status = format!("Failed to query CAN mapping: {}", e);
            }
        }
    }

    fn start_logging(&mut self) {
        if !self.capture_qnx && !self.capture_android && !self.capture_can && !self.capture_eth {
            self.status = "Select at least one log source (QNX/Android/CAN/Ethernet).".into();
            return;
        }

        // Connectivity checks
        self.qnx_connected = self.check_qnx();
        self.android_connected = self.check_android();
        self.eth_connected = self.check_eth();
        self.refresh_can_channels();

        if self.capture_qnx && !self.qnx_connected {
            self.status = "QNX not reachable".into();
            return;
        }
        if self.capture_android && !self.android_connected {
            self.status = "Android not reachable".into();
            return;
        }
        if self.capture_can && !self.can_connected {
            self.status = "CAN mapping unavailable (Refresh Connectivity and verify CANoe mapping).".into();
            return;
        }
        if self.capture_eth && !self.eth_connected {
            self.status = "Ethernet interface not available".into();
            return;
        }

        let default_name = Local::now().format("session_%Y_%m_%d_%H_%M_%S").to_string();
        let requested_name = self.testing_session_name.trim();
        let base_name = if requested_name.is_empty() {
            default_name
        } else {
            let cleaned = LoggerApp::sanitize_folder_name(requested_name);
            if cleaned.is_empty() {
                Local::now().format("session_%Y_%m_%d_%H_%M_%S").to_string()
            } else {
                cleaned
            }
        };

        let mut folder = PathBuf::from("logs").join(&base_name);
        if folder.exists() {
            let suffix = Local::now().format("%Y_%m_%d_%H_%M_%S").to_string();
            folder = PathBuf::from("logs").join(format!("{}_{}", base_name, suffix));
        }

        if let Err(e) = fs::create_dir_all(&folder) {
            self.status = format!("Failed to create log folder: {}", e);
            return;
        }

        let can_folder = folder.join("CAN_LOGS");
        if self.capture_can {
            if let Err(e) = fs::create_dir_all(&can_folder) {
                self.status = format!("Failed to create CAN_LOGS folder: {}", e);
                return;
            }
        }

        self.status = "Running".into();
        self.log_folder = Some(folder.clone());

        // QNX
        if self.capture_qnx && !self.qnx_ip.trim().is_empty() {
            let qnx_log_path = folder.join("qnx.log");
            let qnx_file = match fs::File::create(&qnx_log_path) {
                Ok(f) => f,
                Err(e) => { self.status = format!("Failed to create qnx log: {}", e); return; }
            };

            let qnx_child = Command::new("ssh")
                .arg(format!("root@{}", self.qnx_ip))
                .arg("sh -l -c 'exec slog2info -w'")
                .stdout(qnx_file)
                .spawn();

            match qnx_child {
                Ok(child) => self.qnx_process = Some(child),
                Err(e) => { self.status = format!("Failed to start SSH: {}", e); }
            }
        }

        // Android
        if self.capture_android {
            let android_log_path = folder.join("android.log");
            let android_file = match fs::File::create(&android_log_path) {
                Ok(f) => f,
                Err(e) => { self.status = format!("Failed to create android log: {}", e); return; }
            };

            let mut adb_cmd = Command::new("adb");
            if !self.android_ip.trim().is_empty() {
                let _ = adb_cmd.arg("-s").arg(&self.android_ip);
            }
            let android_child = adb_cmd.arg("logcat").arg("-v").arg("threadtime")
                .stdout(android_file)
                .spawn();

            match android_child {
                Ok(child) => self.android_process = Some(child),
                Err(e) => { self.status = format!("Failed to start adb: {}", e); }
            }
        }

        // Ethernet
        if self.capture_eth {
            let iface = self.interfaces.get(self.selected_interface).cloned().unwrap_or_else(|| "en0".into());
            let eth_path = folder.join("ethernet.pcapng");
            let eth_child = Command::new("dumpcap")
                .arg("-i")
                .arg(&iface)
                .arg("-w")
                .arg(eth_path.to_str().unwrap())
                .spawn();

            match eth_child {
                Ok(child) => self.eth_process = Some(child),
                Err(e) => { self.status = format!("Failed to start dumpcap: {}", e); }
            }
        }

        if self.capture_can {
            let can_console_path = folder.join("can_capture_console.log");
            let can_stdout = match fs::File::create(&can_console_path) {
                Ok(f) => f,
                Err(e) => {
                    self.status = format!("Running (CAN not started: failed to create CAN console log: {})", e);
                    return;
                }
            };
            let can_stderr = match can_stdout.try_clone() {
                Ok(f) => f,
                Err(e) => {
                    self.status = format!("Running (CAN not started: failed to clone CAN console log handle: {})", e);
                    return;
                }
            };

            let exe = match std::env::current_exe() {
                Ok(path) => path,
                Err(e) => {
                    self.status = format!("Running (CAN not started: unable to resolve app path: {})", e);
                    return;
                }
            };

            let can_child = Command::new(exe)
                .arg("--test-can")
                .arg("--can-listen-all")
                .arg("--can-max-channels")
                .arg("64")
                .arg("--can-app")
                .arg("CANoe")
                .arg("--can-iface-version")
                .arg("4")
                .arg("--can-log-format")
                .arg("asc")
                .arg("--can-output-dir")
                .arg(can_folder.to_string_lossy().to_string())
                .stdout(Stdio::from(can_stdout))
                .stderr(Stdio::from(can_stderr))
                .spawn();

            match can_child {
                Ok(child) => self.can_process = Some(child),
                Err(e) => {
                    self.status = format!("Running (CAN not started: {})", e);
                }
            }
        }
    }

    fn stop_logging(&mut self) {
        if let Some(child) = &mut self.qnx_process {
            let _ = child.kill();
        }

        if let Some(child) = &mut self.android_process {
            let _ = child.kill();
        }

        if let Some(child) = &mut self.eth_process {
            let _ = child.kill();
        }

        if let Some(child) = &mut self.can_process {
            let _ = child.kill();
        }

        self.status = "Stopped".into();

        self.qnx_process = None;
        self.android_process = None;
        self.eth_process = None;
        self.can_process = None;
    }
}

impl Default for LoggerApp {
    fn default() -> Self {
        let ifs = LoggerApp::detect_interfaces();
        Self {
            testing_session_name: String::new(),
            qnx_ip: "".into(),
            android_ip: "".into(),
            interfaces: ifs,
            selected_interface: 0,
            status: "Idle".into(),
            qnx_connected: false,
            android_connected: false,
            eth_connected: false,
            can_connected: false,
            capture_qnx: true,
            capture_android: true,
            capture_can: true,
            capture_eth: true,
            qnx_process: None,
            android_process: None,
            eth_process: None,
            can_process: None,
            log_folder: None,
            log_view: 0,
            log_buffer: String::new(),
            last_log_update: Instant::now(),
            can_channels: Vec::new(),
            can_channels_status: "Click Refresh Connectivity to load CAN channel mapping.".into(),
        }
    }
}

impl eframe::App for LoggerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {

            ui.heading("Cross Domain Log Collector");
            ui.separator();

            ui.label("Testing Session Name (optional):");
            ui.text_edit_singleline(&mut self.testing_session_name);

            ui.label("QNX IP: 192.168.164.115");
            ui.text_edit_singleline(&mut self.qnx_ip);

            ui.label("Android IP/ID: {NA_A2B: 7d186538}");
            ui.text_edit_singleline(&mut self.android_ip);

            ui.label("Ethernet Interface:");
            egui::ComboBox::from_label("Interface")
                .selected_text(
                    self.interfaces.get(self.selected_interface).cloned().unwrap_or_else(|| "".into()),
                )
                .show_ui(ui, |ui| {
                    for (i, ifname) in self.interfaces.iter().enumerate() {
                        ui.selectable_value(&mut self.selected_interface, i, ifname);
                    }
                });

            ui.add_space(10.0);

            ui.label("Select Logs to Capture:");
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.capture_qnx, "QNX");
                ui.checkbox(&mut self.capture_android, "Android");
                ui.checkbox(&mut self.capture_can, "CAN");
                ui.checkbox(&mut self.capture_eth, "Ethernet");
            });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                // Connection / collecting indicators
                let qnx_collecting = self.qnx_process.is_some();
                let qnx_color = if qnx_collecting {
                    egui::Color32::from_rgb(0, 122, 255)
                } else if self.qnx_connected {
                    egui::Color32::from_rgb(0, 200, 0)
                } else {
                    egui::Color32::from_rgb(200, 0, 0)
                };
                ui.label(egui::RichText::new("●").color(qnx_color));
                ui.label("QNX");

                let android_collecting = self.android_process.is_some();
                let android_color = if android_collecting {
                    egui::Color32::from_rgb(0, 122, 255)
                } else if self.android_connected {
                    egui::Color32::from_rgb(0, 200, 0)
                } else {
                    egui::Color32::from_rgb(200, 0, 0)
                };
                ui.label(egui::RichText::new("●").color(android_color));
                ui.label("Android");

                let eth_collecting = self.eth_process.is_some();
                let eth_color = if eth_collecting {
                    egui::Color32::from_rgb(0, 122, 255)
                } else if self.eth_connected {
                    egui::Color32::from_rgb(0, 200, 0)
                } else {
                    egui::Color32::from_rgb(200, 0, 0)
                };
                ui.label(egui::RichText::new("●").color(eth_color));
                ui.label("Ethernet");

                let can_collecting = self.can_process.is_some();
                let can_color = if can_collecting {
                    egui::Color32::from_rgb(0, 122, 255)
                } else if self.can_connected {
                    egui::Color32::from_rgb(0, 200, 0)
                } else {
                    egui::Color32::from_rgb(200, 0, 0)
                };
                ui.label(egui::RichText::new("●").color(can_color));
                ui.label("CAN");

                if ui.button("Refresh Connectivity").clicked() {
                    self.qnx_connected = self.check_qnx();
                    self.android_connected = self.check_android();
                    self.eth_connected = self.check_eth();
                    self.refresh_can_channels();
                }
            });

            ui.horizontal(|ui| {
                if ui.button("START").clicked() {
                    self.start_logging();
                }

                if ui.button("STOP").clicked() {
                    self.stop_logging();
                }
            });

            ui.add_space(6.0);
            ui.label(format!("Status: {}", self.status));

            ui.add_space(6.0);
            egui::CollapsingHeader::new("Available CAN Channels (CANoe mapping)")
                .default_open(true)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical().max_height(100.0).show(ui, |ui| {
                        if self.can_channels.is_empty() {
                            ui.label(&self.can_channels_status);
                        } else {
                            for (channel, network) in &self.can_channels {
                                ui.colored_label(
                                    egui::Color32::from_rgb(0, 200, 0),
                                    format!("CAN Channel {} -> {}", channel, network),
                                );
                            }
                        }
                    });
                });

            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Log View:");
                egui::ComboBox::from_id_source("log_view")
                    .selected_text(match self.log_view {
                        0 => "QNX",
                        1 => "Android",
                        _ => "Ethernet",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.log_view, 0, "QNX");
                        ui.selectable_value(&mut self.log_view, 1, "Android");
                        ui.selectable_value(&mut self.log_view, 2, "Ethernet");
                    });
            });

            // Update log buffer at most every 500ms
            if self.last_log_update.elapsed().as_millis() > 500 {
                if let Some(folder) = &self.log_folder {
                    let path = match self.log_view {
                        0 => folder.join("qnx.log"),
                        1 => folder.join("android.log"),
                        _ => folder.join("ethernet.pcapng"),
                    };
                    // For pcap, show a small message
                    if self.log_view == 2 {
                        self.log_buffer = format!("Capturing to: {}", path.display());
                    } else {
                        self.log_buffer = LoggerApp::read_tail(&path, 32 * 1024);
                    }
                }
                self.last_log_update = Instant::now();
            }

            egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                ui.label(egui::RichText::new(self.log_buffer.clone()).monospace());
            });

        });

        ctx.request_repaint();
    }
}

fn main() -> Result<(), eframe::Error> {
    let args: Vec<String> = std::env::args().collect();

    // Check for --test-can flag (for Vector CAN test)
    if args.iter().any(|arg| arg == "--test-can") {
        #[cfg(feature = "vxl-can")]
        {
            let mut selected_channel: Option<u32> = None;
            let mut timeout_ms: u64 = 300;
            let mut can_app_name = String::from("CANoe");
            let mut can_iface_version: u32 = 4;
            let mut can_max_channels: u32 = 64;
            let mut can_diagnose = false;
            let mut can_map = false;
            let mut can_listen = false;
            let mut can_listen_all = false;
            let mut can_duration_ms: Option<u64> = None;
            let mut can_log_file: Option<String> = None;
            let mut can_log_format = String::from("text");
            let mut can_output_dir: Option<String> = None;
            let mut index = 0usize;
            while index < args.len() {
                if args[index] == "--can-channel" && index + 1 < args.len() {
                    selected_channel = args[index + 1].parse::<u32>().ok();
                }
                if args[index] == "--can-timeout-ms" && index + 1 < args.len() {
                    if let Ok(parsed) = args[index + 1].parse::<u64>() {
                        timeout_ms = parsed;
                    }
                }
                if args[index] == "--can-app" && index + 1 < args.len() {
                    can_app_name = args[index + 1].clone();
                }
                if args[index] == "--can-iface-version" && index + 1 < args.len() {
                    if let Ok(parsed) = args[index + 1].parse::<u32>() {
                        can_iface_version = parsed;
                    }
                }
                if args[index] == "--can-max-channels" && index + 1 < args.len() {
                    if let Ok(parsed) = args[index + 1].parse::<u32>() {
                        can_max_channels = parsed.max(1).min(64);
                    }
                }
                if args[index] == "--can-diagnose" {
                    can_diagnose = true;
                }
                if args[index] == "--can-map" {
                    can_map = true;
                }
                if args[index] == "--can-listen" {
                    can_listen = true;
                }
                if args[index] == "--can-listen-all" {
                    can_listen_all = true;
                }
                if args[index] == "--can-duration-ms" && index + 1 < args.len() {
                    if let Ok(parsed) = args[index + 1].parse::<u64>() {
                        can_duration_ms = Some(parsed);
                    }
                }
                if args[index] == "--can-log-file" && index + 1 < args.len() {
                    can_log_file = Some(args[index + 1].clone());
                }
                if args[index] == "--can-output-dir" && index + 1 < args.len() {
                    can_output_dir = Some(args[index + 1].clone());
                }
                if args[index] == "--can-log-format" && index + 1 < args.len() {
                    can_log_format = args[index + 1].to_lowercase();
                }
                index += 1;
            }

            match vxl_capture::try_open_driver() {
                Ok(_) => println!("vxlapi driver opened successfully"),
                Err(e) => {
                    println!("vxlapi error: {}", e);
                    return Ok(());
                }
            }

            if can_map {
                vxl_capture::print_can_channel_mapping(can_max_channels, &can_app_name);
            } else if can_diagnose {
                vxl_capture::diagnose_can_setup(can_max_channels, &can_app_name, can_iface_version);
            } else if can_listen_all {
                let format = if can_log_format == "asc" {
                    vxl_capture::CanLogFormat::Asc
                } else {
                    vxl_capture::CanLogFormat::Text
                };
                match vxl_capture::listen_can_all_connected(
                    can_max_channels,
                    &can_app_name,
                    can_iface_version,
                    can_duration_ms,
                    can_output_dir.as_deref(),
                    format,
                ) {
                    Ok(()) => {}
                    Err(e) => println!("CAN listen-all error: {}", e),
                }
            } else if can_listen {
                if let Some(channel) = selected_channel {
                    let format = if can_log_format == "asc" {
                        vxl_capture::CanLogFormat::Asc
                    } else {
                        vxl_capture::CanLogFormat::Text
                    };
                    match vxl_capture::listen_can_on_channel(
                        channel,
                        &can_app_name,
                        can_iface_version,
                        can_duration_ms,
                        can_log_file.as_deref(),
                        format,
                    ) {
                        Ok(()) => {}
                        Err(e) => println!("CAN listen error: {}", e),
                    }
                } else {
                    println!("--can-listen requires --can-channel <n>");
                }
            } else if let Some(channel) = selected_channel {
                println!(
                    "Listening on CAN channel {} for {} ms (app: {}, iface: {})...",
                    channel, timeout_ms, can_app_name, can_iface_version
                );
                match vxl_capture::try_capture_can_on_channel(
                    channel,
                    timeout_ms,
                    &can_app_name,
                    can_iface_version,
                ) {
                    Ok(true) => println!("CAN capture complete on channel {}", channel),
                    Ok(false) => println!("No CAN frame received on channel {}", channel),
                    Err(e) => println!("CAN capture error on channel {}: {}", channel, e),
                }
            } else {
                println!(
                    "Auto-scanning CAN channels 0..{} for traffic (app: {}, iface: {})...",
                    can_max_channels.saturating_sub(1),
                    can_app_name,
                    can_iface_version
                );
                match vxl_capture::try_capture_any_can(
                    can_max_channels,
                    timeout_ms,
                    &can_app_name,
                    can_iface_version,
                ) {
                    Ok(channel) => println!("Detected traffic on CAN channel {}", channel),
                    Err(e) => println!("CAN auto-scan result: {}", e),
                }
            }

            vxl_capture::try_close_driver();
            return Ok(());
        }

        #[cfg(not(feature = "vxl-can"))]
        {
            println!(
                "--test-can requested, but vxl CAN support is disabled. Rebuild with --features vxl-can"
            );
            return Ok(());
        }
    }
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Cross Domain Logger",
        options,
        Box::new(|_cc| Box::new(LoggerApp::default())),
    )
}