use eframe::egui;
use std::process::{Command, Child};
use std::fs;
use chrono::Local;
use std::path::PathBuf;
use std::time::Instant;
use std::io::{Read, Seek, SeekFrom};

struct LoggerApp {
    qnx_ip: String,
    android_ip: String,
    interfaces: Vec<String>,
    selected_interface: usize,
    status: String,
    qnx_connected: bool,
    android_connected: bool,
    eth_connected: bool,

    qnx_process: Option<Child>,
    android_process: Option<Child>,
    eth_process: Option<Child>,

    log_folder: Option<PathBuf>,

    // UI log viewer
    log_view: usize, // 0 = QNX, 1 = Android, 2 = Ethernet
    log_buffer: String,
    last_log_update: Instant,
}

impl LoggerApp {
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

    fn start_logging(&mut self) {
        // Connectivity checks
        self.qnx_connected = self.check_qnx();
        self.android_connected = self.check_android();
        self.eth_connected = self.check_eth();

        if !self.qnx_connected {
            self.status = "QNX not reachable".into();
            return;
        }
        if !self.android_connected {
            self.status = "Android not reachable".into();
            return;
        }

        let timestamp = Local::now().format("%Y_%m_%d_%H_%M_%S").to_string();
        let folder = PathBuf::from(format!("logs/{}", timestamp));

        if let Err(e) = fs::create_dir_all(&folder) {
            self.status = format!("Failed to create log folder: {}", e);
            return;
        }

        self.status = "Running".into();
        self.log_folder = Some(folder.clone());

        // QNX
        let qnx_log_path = folder.join("qnx.log");
        let qnx_file = match fs::File::create(&qnx_log_path) {
            Ok(f) => f,
            Err(e) => { self.status = format!("Failed to create qnx log: {}", e); return; }
        };

        if !self.qnx_ip.trim().is_empty() {
            let qnx_child = Command::new("ssh")
                .arg(format!("root@{}", self.qnx_ip))
                .arg("echo QNX_READY && sleep 1 && slog2info -w")
                .stdout(qnx_file)
                .spawn();

            match qnx_child {
                Ok(child) => self.qnx_process = Some(child),
                Err(e) => { self.status = format!("Failed to start SSH: {}", e); }
            }
        }

        // Android
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

        // Ethernet
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

        self.status = "Stopped".into();

        self.qnx_process = None;
        self.android_process = None;
        self.eth_process = None;
    }
}

impl Default for LoggerApp {
    fn default() -> Self {
        let ifs = LoggerApp::detect_interfaces();
        Self {
            qnx_ip: "".into(),
            android_ip: "".into(),
            interfaces: ifs,
            selected_interface: 0,
            status: "Idle".into(),
            qnx_connected: false,
            android_connected: false,
            eth_connected: false,
            qnx_process: None,
            android_process: None,
            eth_process: None,
            log_folder: None,
            log_view: 0,
            log_buffer: String::new(),
            last_log_update: Instant::now(),
        }
    }
}

impl eframe::App for LoggerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {

            ui.heading("Cross Domain Log Collector");
            ui.separator();

            ui.label("QNX IP:");
            ui.text_edit_singleline(&mut self.qnx_ip);

            ui.label("Android IP/ID:");
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

                if ui.button("Refresh Connectivity").clicked() {
                    self.qnx_connected = self.check_qnx();
                    self.android_connected = self.check_android();
                    self.eth_connected = self.check_eth();
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
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Cross Domain Logger",
        options,
        Box::new(|_cc| Box::new(LoggerApp::default())),
    )
}