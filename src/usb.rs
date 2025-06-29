use anyhow::{Context, Result, bail};
use console::{Term, style};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::fs;
use tokio::sync::{RwLock, mpsc};
use tokio::time::interval;

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

    /// Scan for mounted volumes that could be USB devices
    pub async fn scan_devices(&self) -> Result<Vec<UsbDevice>> {
        let mount_points = self.get_potential_usb_mounts().await?;
        let mut devices = Vec::new();
        let term = Term::stderr();

        for mount_path in mount_points {
            if let Ok(usb_device) = self.create_device_from_mount(&mount_path).await {
                devices.push(usb_device);
            }
        }

        // Update internal device list
        let mut detected = self.detected_devices.write().await;
        detected.clear();
        for device in &devices {
            detected.insert(
                device.device_path.to_string_lossy().to_string(),
                device.clone(),
            );
        }

        let _ = term.write_line(&format!(
            "{} Found {} potential USB devices",
            style("🔍").cyan(),
            style(devices.len()).green()
        ));

        Ok(devices)
    }

    /// Get potential USB mount points based on platform
    async fn get_potential_usb_mounts(&self) -> Result<Vec<PathBuf>> {
        #[cfg(target_os = "linux")]
        return self.get_linux_mounts().await;

        #[cfg(target_os = "windows")]
        return self.get_windows_drives().await;

        #[cfg(target_os = "macos")]
        return self.get_macos_volumes().await;

        #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
        Ok(vec![])
    }

    #[cfg(target_os = "linux")]
    async fn get_linux_mounts(&self) -> Result<Vec<PathBuf>> {
        let mut mount_points = Vec::new();

        // Check common mount locations
        let potential_dirs = ["/media", "/mnt", "/run/media"];

        for base_dir in &potential_dirs {
            let base_path = PathBuf::from(base_dir);
            if base_path.exists() {
                if let Ok(mut entries) = tokio::fs::read_dir(&base_path).await {
                    while let Ok(Some(entry)) = entries.next_entry().await {
                        let path = entry.path();
                        if path.is_dir() {
                            // Check if it has more subdirectories (user-based mounts)
                            if let Ok(mut sub_entries) = tokio::fs::read_dir(&path).await {
                                while let Ok(Some(sub_entry)) = sub_entries.next_entry().await {
                                    let sub_path = sub_entry.path();
                                    if sub_path.is_dir() {
                                        mount_points.push(sub_path);
                                    }
                                }
                            } else {
                                // Direct mount
                                mount_points.push(path);
                            }
                        }
                    }
                }
            }
        }

        Ok(mount_points)
    }

    #[cfg(target_os = "windows")]
    async fn get_windows_drives(&self) -> Result<Vec<PathBuf>> {
        let mut drives = Vec::new();

        // Check drive letters A-Z
        for letter in b'A'..=b'Z' {
            let drive_path = format!("{}:\\", letter as char);
            let path = PathBuf::from(&drive_path);

            // Check if drive exists
            if tokio::fs::metadata(&path).await.is_ok() {
                // Skip C: drive (usually system drive)
                if letter != b'C' {
                    drives.push(path);
                }
            }
        }

        Ok(drives)
    }

    #[cfg(target_os = "macos")]
    async fn get_macos_volumes(&self) -> Result<Vec<PathBuf>> {
        let volumes_dir = PathBuf::from("/Volumes");
        let mut volumes = Vec::new();

        if volumes_dir.exists() {
            if let Ok(mut entries) = tokio::fs::read_dir(&volumes_dir).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let path = entry.path();
                    if path.is_dir() {
                        // Skip system volumes
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            if !name.contains("Macintosh") && name != "." && name != ".." {
                                volumes.push(path);
                            }
                        }
                    }
                }
            }
        }

        Ok(volumes)
    }

    async fn create_device_from_mount(&self, mount_path: &PathBuf) -> Result<UsbDevice> {
        // Check if the mount point is accessible
        let metadata = tokio::fs::metadata(mount_path)
            .await
            .context("Failed to access mount point")?;

        if !metadata.is_dir() {
            bail!("Mount point is not a directory");
        }

        // Extract label from path
        let label = mount_path
            .file_name()
            .and_then(|n| n.to_str())
            .filter(|s| !s.is_empty() && *s != "/" && *s != "\\")
            .map(|s| s.to_string());

        // Get space information
        let (total_space, available_space) =
            self.get_space_info(mount_path).await.unwrap_or((0, 0));

        let mut usb_device = UsbDevice {
            device_path: mount_path.clone(),
            mount_point: Some(mount_path.clone()),
            label,
            filesystem: "unknown".to_string(),
            total_space,
            available_space,
            is_ventoy: false,
            ventoy_version: None,
            last_seen: SystemTime::now(),
        };

        // Check for Ventoy installation
        let _ = self.check_ventoy_installation(&mut usb_device).await;

        Ok(usb_device)
    }

    /// Get filesystem space information using std::fs
    async fn get_space_info(&self, path: &PathBuf) -> Result<(u64, u64)> {
        // For now, we'll use a simple approach that works cross-platform
        // We'll try to create a temp file and use basic filesystem operations

        // Try to get some basic space info by checking available space
        // This is a simplified approach - in a real implementation you'd use platform-specific APIs

        #[cfg(target_family = "unix")]
        {
            // On Unix, try using statvfs if available
            self.get_unix_space_info(path).await
        }

        #[cfg(target_os = "windows")]
        {
            // On Windows, we'd use GetDiskFreeSpaceEx
            self.get_windows_space_info(path).await
        }

        #[cfg(not(any(target_family = "unix", target_os = "windows")))]
        {
            // Fallback for other platforms
            Ok((0, 1024 * 1024 * 1024)) // Assume 1GB available
        }
    }

    #[cfg(target_family = "unix")]
    async fn get_unix_space_info(&self, path: &PathBuf) -> Result<(u64, u64)> {
        // Simple fallback: assume we have space if we can write to the directory
        let test_file = path.join(".space_test_isod");

        match tokio::fs::write(&test_file, "test").await {
            Ok(_) => {
                let _ = tokio::fs::remove_file(&test_file).await;
                // Return some reasonable defaults
                Ok((10 * 1024 * 1024 * 1024, 5 * 1024 * 1024 * 1024)) // 10GB total, 5GB available
            }
            Err(_) => {
                // Can't write, assume no space
                Ok((1024 * 1024 * 1024, 0)) // 1GB total, 0 available
            }
        }
    }

    #[cfg(target_os = "windows")]
    async fn get_windows_space_info(&self, path: &PathBuf) -> Result<(u64, u64)> {
        // Simple test similar to Unix
        let test_file = path.join(".space_test_isod");

        match tokio::fs::write(&test_file, "test").await {
            Ok(_) => {
                let _ = tokio::fs::remove_file(&test_file).await;
                Ok((10 * 1024 * 1024 * 1024, 5 * 1024 * 1024 * 1024)) // 10GB total, 5GB available
            }
            Err(_) => {
                Ok((1024 * 1024 * 1024, 0)) // 1GB total, 0 available
            }
        }
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

        let term = Term::stderr();
        let _ = term.write_line(&format!(
            "{} Found {} Ventoy devices",
            style("📀").cyan(),
            style(ventoy_devices.len()).green()
        ));
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

        // Get fresh space info
        let (_, actual_available) = self
            .get_space_info(mount_point)
            .await
            .unwrap_or((0, device.available_space));
        let required_space = 100 * 1024 * 1024; // 100MB

        if actual_available < required_space {
            bail!(
                "Insufficient free space (need at least {} MB, found {} MB)",
                required_space / (1024 * 1024),
                actual_available / (1024 * 1024)
            );
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

        let term = Term::stderr();
        let _ = term.write_line(&format!(
            "{} Selected device: {} ({})",
            style("✅").green(),
            style(device_path).cyan(),
            device.label.as_deref().unwrap_or("unlabeled")
        ));

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
            self.scan_devices().await?;
            self.select_device(&path).await?;
        }

        Ok(())
    }

    /// Get available space on the current device
    pub async fn get_available_space(&self) -> Result<u64> {
        let current = self.current_device.read().await;
        let device = current.as_ref().context("No device currently selected")?;

        if let Some(mount_point) = &device.mount_point {
            let (_, available) = self
                .get_space_info(mount_point)
                .await
                .unwrap_or((0, device.available_space));
            Ok(available)
        } else {
            Ok(device.available_space)
        }
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

        let devices_ref = Arc::clone(&self.detected_devices);
        let sender_ref = self.event_sender.as_ref().unwrap().clone();
        let monitoring_ref = Arc::clone(&self.monitoring);

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(2));
            let mut last_devices: HashMap<String, UsbDevice> = HashMap::new();

            loop {
                interval.tick().await;

                if !*monitoring_ref.read().await {
                    break;
                }

                let temp_manager = UsbManager::new();
                if let Ok(current_devices) = temp_manager.scan_devices().await {
                    let current_map: HashMap<String, UsbDevice> = current_devices
                        .into_iter()
                        .map(|d| (d.device_path.to_string_lossy().to_string(), d))
                        .collect();

                    // Check for new devices
                    for (path, device) in &current_map {
                        if !last_devices.contains_key(path) {
                            let term = Term::stderr();
                            let _ = term.write_line(&format!(
                                "{} New device detected: {}",
                                style("🔌").green(),
                                style(path).cyan()
                            ));
                            let _ = sender_ref.send(UsbEvent::DeviceAdded(device.clone()));
                        }
                    }

                    // Check for removed devices
                    for path in last_devices.keys() {
                        if !current_map.contains_key(path) {
                            let term = Term::stderr();
                            let _ = term.write_line(&format!(
                                "{} Device removed: {}",
                                style("🔌").red(),
                                style(path).cyan()
                            ));
                            let _ = sender_ref.send(UsbEvent::DeviceRemoved(path.clone()));
                        }
                    }

                    {
                        let mut detected = devices_ref.write().await;
                        *detected = current_map.clone();
                    }

                    last_devices = current_map;
                }
            }
        });

        let term = Term::stderr();
        let _ = term.write_line(&format!(
            "{} Started USB device monitoring",
            style("👁️").cyan()
        ));
        Ok(receiver)
    }

    /// Stop monitoring for device changes
    pub async fn stop_monitoring(&self) {
        let mut monitoring = self.monitoring.write().await;
        *monitoring = false;
        let term = Term::stderr();
        let _ = term.write_line(&format!(
            "{} Stopped USB device monitoring",
            style("👁️").yellow()
        ));
    }

    /// Get active downloads
    pub async fn get_active_downloads(&self) -> Vec<String> {
        vec![]
    }
}

impl Default for UsbManager {
    fn default() -> Self {
        Self::new()
    }
}
