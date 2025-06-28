use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::fs;
use tokio::sync::{RwLock, mpsc};
use tokio::time::interval;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbDevice {
    pub device_path: PathBuf,
    pub mount_point: Option<PathBuf>,
    pub label: Option<String>,
    pub filesystem: String,
    pub total_space: u64,
    pub available_space: u64,
    pub is_ventoy: bool,
    pub ventoy_version: Option<String>,
    pub last_seen: SystemTime,
}

#[derive(Debug, Clone)]
pub enum UsbEvent {
    DeviceAdded(UsbDevice),
    DeviceRemoved(String), // device path
    DeviceUpdated(UsbDevice),
    VentoyDetected(UsbDevice),
}

pub type UsbEventCallback = Box<dyn Fn(UsbEvent) + Send + Sync>;

pub struct UsbManager {
    detected_devices: Arc<RwLock<HashMap<String, UsbDevice>>>,
    current_device: Arc<RwLock<Option<UsbDevice>>>,
    event_sender: Option<mpsc::UnboundedSender<UsbEvent>>,
    monitoring: Arc<RwLock<bool>>,
}

impl UsbManager {
    pub fn new() -> Self {
        Self {
            detected_devices: Arc::new(RwLock::new(HashMap::new())),
            current_device: Arc::new(RwLock::new(None)),
            event_sender: None,
            monitoring: Arc::new(RwLock::new(false)),
        }
    }

    /// Scan for all USB storage devices
    pub async fn scan_devices(&self) -> Result<Vec<UsbDevice>> {
        debug!("Scanning for USB devices...");

        #[cfg(target_os = "linux")]
        let devices = self.scan_devices_linux().await?;
        #[cfg(target_os = "windows")]
        let devices = self.scan_devices_windows().await?;
        #[cfg(target_os = "macos")]
        let devices = self.scan_devices_macos().await?;

        // Update internal device list
        let mut detected = self.detected_devices.write().await;
        detected.clear();
        for device in &devices {
            detected.insert(
                device.device_path.to_string_lossy().to_string(),
                device.clone(),
            );
        }

        info!("Found {} USB storage devices", devices.len());
        Ok(devices)
    }

    /// Find devices with Ventoy installed
    pub async fn find_ventoy_devices(&self) -> Result<Vec<UsbDevice>> {
        let devices = self.scan_devices().await?;
        let mut ventoy_devices = Vec::new();

        for mut device in devices {
            if self.check_ventoy_installation(&mut device).await.is_ok() {
                ventoy_devices.push(device);
            }
        }

        info!("Found {} Ventoy devices", ventoy_devices.len());
        Ok(ventoy_devices)
    }

    /// Validate that a device is a proper Ventoy installation
    pub async fn validate_ventoy_device(&self, device: &UsbDevice) -> Result<()> {
        if !device.is_ventoy {
            bail!("Device is not a Ventoy installation");
        }

        let mount_point = device
            .mount_point
            .as_ref()
            .context("Device is not mounted")?;

        // Check for Ventoy signature files
        let ventoy_dir = mount_point.join("ventoy");
        if !ventoy_dir.exists() {
            bail!("Ventoy directory not found");
        }

        let ventoy_json = ventoy_dir.join("ventoy.json");
        if !ventoy_json.exists() {
            bail!("Ventoy configuration file not found");
        }

        // Check write permissions
        let test_file = mount_point.join(".isod_write_test");
        match fs::write(&test_file, "test").await {
            Ok(_) => {
                let _ = fs::remove_file(&test_file).await;
            }
            Err(_) => bail!("No write permission to device"),
        }

        // Verify minimum free space (100MB)
        if device.available_space < 100 * 1024 * 1024 {
            bail!("Insufficient free space (need at least 100MB)");
        }

        Ok(())
    }

    /// Select a device as the current working device
    pub async fn select_device(&self, device_path: &str) -> Result<()> {
        let devices = self.detected_devices.read().await;
        let device = devices
            .get(device_path)
            .context("Device not found in detected devices")?
            .clone();

        self.validate_ventoy_device(&device).await?;

        let mut current = self.current_device.write().await;
        *current = Some(device.clone());

        info!(
            "Selected device: {} ({})",
            device_path,
            device.label.as_deref().unwrap_or("unlabeled")
        );

        if let Some(sender) = &self.event_sender {
            let _ = sender.send(UsbEvent::VentoyDetected(device));
        }

        Ok(())
    }

    /// Get the currently selected device
    pub async fn get_current_device(&self) -> Option<UsbDevice> {
        self.current_device.read().await.clone()
    }

    /// Refresh information for the current device
    pub async fn refresh_current_device(&self) -> Result<()> {
        let current_path = {
            let current = self.current_device.read().await;
            current
                .as_ref()
                .map(|d| d.device_path.to_string_lossy().to_string())
        };

        if let Some(path) = current_path {
            // Re-scan devices to get updated info
            self.scan_devices().await?;

            // Re-select the device to update current device info
            self.select_device(&path).await?;
        }

        Ok(())
    }

    /// Start monitoring for device changes
    pub async fn start_monitoring(&mut self) -> Result<mpsc::UnboundedReceiver<UsbEvent>> {
        let (sender, receiver) = mpsc::unbounded_channel();
        self.event_sender = Some(sender);

        let mut monitoring = self.monitoring.write().await;
        if *monitoring {
            bail!("Already monitoring device changes");
        }
        *monitoring = true;

        // Start polling task
        let devices_ref = Arc::clone(&self.detected_devices);
        let sender_ref = self.event_sender.as_ref().unwrap().clone();
        let monitoring_ref = Arc::clone(&self.monitoring);

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(2));
            let mut last_devices: HashMap<String, UsbDevice> = HashMap::new();

            loop {
                interval.tick().await;

                // Check if we should stop monitoring
                if !*monitoring_ref.read().await {
                    break;
                }

                // Create a temporary manager for scanning
                let temp_manager = UsbManager::new();
                if let Ok(current_devices) = temp_manager.scan_devices().await {
                    let current_map: HashMap<String, UsbDevice> = current_devices
                        .into_iter()
                        .map(|d| (d.device_path.to_string_lossy().to_string(), d))
                        .collect();

                    // Check for new devices
                    for (path, device) in &current_map {
                        if !last_devices.contains_key(path) {
                            debug!("New device detected: {}", path);
                            let _ = sender_ref.send(UsbEvent::DeviceAdded(device.clone()));
                        } else if let Some(old_device) = last_devices.get(path) {
                            // Check if device was updated (mount status changed)
                            if old_device.mount_point != device.mount_point {
                                let _ = sender_ref.send(UsbEvent::DeviceUpdated(device.clone()));
                            }
                        }
                    }

                    // Check for removed devices
                    for path in last_devices.keys() {
                        if !current_map.contains_key(path) {
                            debug!("Device removed: {}", path);
                            let _ = sender_ref.send(UsbEvent::DeviceRemoved(path.clone()));
                        }
                    }

                    // Update devices in the manager
                    {
                        let mut detected = devices_ref.write().await;
                        *detected = current_map.clone();
                    }

                    last_devices = current_map;
                }
            }
        });

        info!("Started USB device monitoring");
        Ok(receiver)
    }

    /// Stop monitoring for device changes
    pub async fn stop_monitoring(&self) {
        let mut monitoring = self.monitoring.write().await;
        *monitoring = false;
        info!("Stopped USB device monitoring");
    }

    /// Get the ISO directory for the current device
    pub async fn get_iso_directory(&self) -> Result<PathBuf> {
        let current = self.current_device.read().await;
        let device = current.as_ref().context("No device currently selected")?;

        let mount_point = device
            .mount_point
            .as_ref()
            .context("Current device is not mounted")?;

        Ok(mount_point.join("iso"))
    }

    /// Get available space on the current device
    pub async fn get_available_space(&self) -> Result<u64> {
        let current = self.current_device.read().await;
        let device = current.as_ref().context("No device currently selected")?;

        Ok(device.available_space)
    }

    /// Create isod metadata directory on current device
    pub async fn create_isod_metadata_dir(&self) -> Result<PathBuf> {
        let current = self.current_device.read().await;
        let device = current.as_ref().context("No device currently selected")?;

        let mount_point = device
            .mount_point
            .as_ref()
            .context("Current device is not mounted")?;

        let metadata_dir = mount_point.join("isod");
        fs::create_dir_all(&metadata_dir)
            .await
            .with_context(|| format!("Failed to create metadata directory: {:?}", metadata_dir))?;

        Ok(metadata_dir)
    }

    /// Check if a device has Ventoy installed and update device info
    async fn check_ventoy_installation(&self, device: &mut UsbDevice) -> Result<()> {
        let mount_point = device
            .mount_point
            .as_ref()
            .context("Device is not mounted")?;

        let ventoy_dir = mount_point.join("ventoy");
        let ventoy_json = ventoy_dir.join("ventoy.json");

        if !ventoy_json.exists() {
            bail!("No Ventoy installation found");
        }

        // Try to read Ventoy version
        if let Ok(content) = fs::read_to_string(&ventoy_json).await {
            if let Ok(ventoy_config) = serde_json::from_str::<serde_json::Value>(&content) {
                device.ventoy_version = ventoy_config
                    .get("VENTOY_VERSION")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
            }
        }

        device.is_ventoy = true;
        debug!(
            "Ventoy installation detected on {}",
            device.device_path.display()
        );
        Ok(())
    }

    #[cfg(target_os = "linux")]
    async fn scan_devices_linux(&self) -> Result<Vec<UsbDevice>> {
        let mut devices = Vec::new();

        // Use lsblk to get device information
        let output = Command::new("lsblk")
            .args(&[
                "-J",
                "-o",
                "NAME,MOUNTPOINT,LABEL,FSTYPE,SIZE,AVAIL,TYPE,HOTPLUG",
            ])
            .output()
            .context("Failed to execute lsblk command")?;

        if !output.status.success() {
            bail!("lsblk command failed");
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        let lsblk_output: serde_json::Value =
            serde_json::from_str(&json_str).context("Failed to parse lsblk JSON output")?;

        if let Some(blockdevices) = lsblk_output.get("blockdevices").and_then(|v| v.as_array()) {
            for device in blockdevices {
                // Only process removable devices
                if let Some(hotplug) = device.get("hotplug").and_then(|v| v.as_str()) {
                    if hotplug != "1" {
                        continue;
                    }
                }

                // Process children (partitions)
                if let Some(children) = device.get("children").and_then(|v| v.as_array()) {
                    for child in children {
                        if let Ok(usb_device) = self.parse_linux_device(child).await {
                            devices.push(usb_device);
                        }
                    }
                }
            }
        }

        Ok(devices)
    }

    #[cfg(target_os = "linux")]
    async fn parse_linux_device(&self, device: &serde_json::Value) -> Result<UsbDevice> {
        let name = device
            .get("name")
            .and_then(|v| v.as_str())
            .context("Device name not found")?;

        let device_path = PathBuf::from(format!("/dev/{}", name));

        let mount_point = device
            .get("mountpoint")
            .and_then(|v| v.as_str())
            .map(PathBuf::from);

        let label = device
            .get("label")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let filesystem = device
            .get("fstype")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        // Parse size (comes as human readable, e.g., "8G")
        let total_space =
            self.parse_size_string(device.get("size").and_then(|v| v.as_str()).unwrap_or("0"))?;

        let available_space =
            self.parse_size_string(device.get("avail").and_then(|v| v.as_str()).unwrap_or("0"))?;

        let mut usb_device = UsbDevice {
            device_path,
            mount_point,
            label,
            filesystem,
            total_space,
            available_space,
            is_ventoy: false,
            ventoy_version: None,
            last_seen: SystemTime::now(),
        };

        // Check for Ventoy installation if mounted
        if usb_device.mount_point.is_some() {
            let _ = self.check_ventoy_installation(&mut usb_device).await;
        }

        Ok(usb_device)
    }

    #[cfg(target_os = "windows")]
    async fn scan_devices_windows(&self) -> Result<Vec<UsbDevice>> {
        let mut devices = Vec::new();

        // Use PowerShell to get removable drives
        let output = Command::new("powershell")
            .args(&["-Command",
                   "Get-WmiObject -Class Win32_LogicalDisk | Where-Object {$_.DriveType -eq 2} | ConvertTo-Json"])
            .output()
            .context("Failed to execute PowerShell command")?;

        if !output.status.success() {
            #[allow(unused_imports)]
            use tracing::warn;
            warn!("PowerShell command failed, trying alternative method");
            return self.scan_devices_windows_fallback().await;
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        if let Ok(drives) = serde_json::from_str::<serde_json::Value>(&json_str) {
            let drives_array = if drives.is_array() {
                drives.as_array().unwrap()
            } else {
                // Single drive, wrap in array
                std::slice::from_ref(&drives)
            };

            for drive in drives_array {
                if let Ok(usb_device) = self.parse_windows_device(drive).await {
                    devices.push(usb_device);
                }
            }
        }

        Ok(devices)
    }

    #[cfg(target_os = "windows")]
    async fn scan_devices_windows_fallback(&self) -> Result<Vec<UsbDevice>> {
        // Fallback: scan drive letters A-Z for removable drives
        let mut devices = Vec::new();

        for letter in 'A'..='Z' {
            let drive_path = format!("{}:\\", letter);
            let path = Path::new(&drive_path);

            if path.exists() {
                // Check if it's a removable drive using fsutil
                let output = Command::new("fsutil")
                    .args(&["fsinfo", "drivetype", &drive_path])
                    .output();

                if let Ok(output) = output {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    if output_str.contains("Removable Drive") {
                        if let Ok(usb_device) = self.create_windows_device_from_path(&path).await {
                            devices.push(usb_device);
                        }
                    }
                }
            }
        }

        Ok(devices)
    }

    #[cfg(target_os = "windows")]
    async fn parse_windows_device(&self, device: &serde_json::Value) -> Result<UsbDevice> {
        let device_id = device
            .get("DeviceID")
            .and_then(|v| v.as_str())
            .context("Device ID not found")?;

        let device_path = PathBuf::from(device_id);
        let mount_point = Some(PathBuf::from(device_id));

        let label = device
            .get("VolumeName")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let total_space = device.get("Size").and_then(|v| v.as_u64()).unwrap_or(0);

        let available_space = device
            .get("FreeSpace")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let mut usb_device = UsbDevice {
            device_path,
            mount_point,
            label,
            filesystem: "NTFS".to_string(), // Default assumption
            total_space,
            available_space,
            is_ventoy: false,
            ventoy_version: None,
            last_seen: SystemTime::now(),
        };

        let _ = self.check_ventoy_installation(&mut usb_device).await;
        Ok(usb_device)
    }

    #[cfg(target_os = "windows")]
    async fn create_windows_device_from_path(&self, path: &Path) -> Result<UsbDevice> {
        // Basic implementation for fallback method
        let mut usb_device = UsbDevice {
            device_path: path.to_path_buf(),
            mount_point: Some(path.to_path_buf()),
            label: None,
            filesystem: "Unknown".to_string(),
            total_space: 0,
            available_space: 0,
            is_ventoy: false,
            ventoy_version: None,
            last_seen: SystemTime::now(),
        };

        let _ = self.check_ventoy_installation(&mut usb_device).await;
        Ok(usb_device)
    }

    #[cfg(target_os = "macos")]
    async fn scan_devices_macos(&self) -> Result<Vec<UsbDevice>> {
        let mut devices = Vec::new();

        // Use diskutil to list external drives
        let output = Command::new("diskutil")
            .args(&["list", "-plist", "external"])
            .output()
            .context("Failed to execute diskutil command")?;

        if !output.status.success() {
            bail!("diskutil command failed");
        }

        // Parse plist output (simplified - you might want to use a plist crate)
        let output_str = String::from_utf8_lossy(&output.stdout);

        // This is a simplified implementation
        // In practice, you'd want to properly parse the plist and get detailed info
        for line in output_str.lines() {
            if line.contains("/dev/disk") {
                if let Ok(usb_device) = self.create_macos_device_from_line(line).await {
                    devices.push(usb_device);
                }
            }
        }

        Ok(devices)
    }

    #[cfg(target_os = "macos")]
    async fn create_macos_device_from_line(&self, line: &str) -> Result<UsbDevice> {
        // Extract device path from diskutil output
        let device_path = PathBuf::from(line.trim());

        let mut usb_device = UsbDevice {
            device_path,
            mount_point: None, // Would need additional diskutil calls to get mount point
            label: None,
            filesystem: "Unknown".to_string(),
            total_space: 0,
            available_space: 0,
            is_ventoy: false,
            ventoy_version: None,
            last_seen: SystemTime::now(),
        };

        // You'd implement proper diskutil info parsing here
        Ok(usb_device)
    }

    /// Parse human-readable size strings (e.g., "8G", "512M") to bytes
    fn parse_size_string(&self, size_str: &str) -> Result<u64> {
        if size_str.is_empty() || size_str == "-" {
            return Ok(0);
        }

        let size_str = size_str.trim();
        let (number_str, unit) = if let Some(last_char) = size_str.chars().last() {
            if last_char.is_alphabetic() {
                (
                    &size_str[..size_str.len() - 1],
                    last_char.to_uppercase().next().unwrap(),
                )
            } else {
                (size_str, 'B')
            }
        } else {
            return Ok(0);
        };

        let number: f64 = number_str
            .parse()
            .with_context(|| format!("Failed to parse size number: {}", number_str))?;

        let multiplier = match unit {
            'B' => 1,
            'K' => 1024,
            'M' => 1024 * 1024,
            'G' => 1024 * 1024 * 1024,
            'T' => 1024_u64.pow(4),
            _ => bail!("Unknown size unit: {}", unit),
        };

        Ok((number * multiplier as f64) as u64)
    }
}

impl Default for UsbManager {
    fn default() -> Self {
        Self::new()
    }
}
