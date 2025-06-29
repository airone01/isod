use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore, mpsc};
use tokio::task::JoinHandle;
use uuid::Uuid;

use super::{ChecksumType, DownloadEngine, DownloadProgress, DownloadRequest, DownloadTask};
use crate::registry::sources::SourceType;
use crate::registry::{DownloadSource, IsoInfo};

#[derive(Debug, Clone)]
pub struct DownloadOptions {
    pub max_concurrent: usize,
    pub prefer_torrents: bool,
    pub output_directory: PathBuf,
    pub verify_checksums: bool,
    pub resume_downloads: bool,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            max_concurrent: 3,
            prefer_torrents: false,
            output_directory: std::env::current_dir().unwrap_or_default(),
            verify_checksums: true,
            resume_downloads: true,
        }
    }
}

pub struct DownloadManager {
    engine: Arc<DownloadEngine>,
    semaphore: Arc<Semaphore>,
    active_downloads: Arc<RwLock<HashMap<String, JoinHandle<()>>>>,
    progress_sender: mpsc::UnboundedSender<DownloadProgress>,
}

impl DownloadManager {
    pub fn new(
        options: DownloadOptions,
    ) -> Result<(Self, mpsc::UnboundedReceiver<DownloadProgress>)> {
        let engine = Arc::new(DownloadEngine::new()?);
        let semaphore = Arc::new(Semaphore::new(options.max_concurrent));
        let active_downloads = Arc::new(RwLock::new(HashMap::new()));
        let (progress_sender, progress_receiver) = mpsc::unbounded_channel();

        Ok((
            Self {
                engine,
                semaphore,
                active_downloads,
                progress_sender,
            },
            progress_receiver,
        ))
    }

    pub async fn download_iso(
        &self,
        iso_info: &IsoInfo,
        options: &DownloadOptions,
    ) -> Result<String> {
        let download_id = format!(
            "{}_{}",
            iso_info.distro,
            Uuid::new_v4().to_string()[..8].to_string()
        );

        // Select best download source
        let source = self.select_best_source(&iso_info.download_sources, options)?;
        let url = source
            .get_url()
            .context("Selected source has no URL")?
            .to_string();

        // Handle URL template replacement
        let resolved_url = self.resolve_url_template(&url, iso_info)?;

        let output_path = options.output_directory.join(&iso_info.filename);

        // Create download request
        let mut request = DownloadRequest::new(resolved_url, output_path);

        if options.verify_checksums {
            if let Some(checksum) = &iso_info.checksum {
                let checksum_type = match iso_info.checksum_type.as_deref() {
                    Some("md5") => ChecksumType::Md5,
                    Some("sha1") => ChecksumType::Sha1,
                    Some("sha256") => ChecksumType::Sha256,
                    Some("sha512") => ChecksumType::Sha512,
                    _ => ChecksumType::Sha256, // Default
                };

                request = request.with_checksum(checksum.clone(), checksum_type);
            }
        }

        if !options.resume_downloads {
            request = request.no_resume();
        }

        self.start_download(download_id.clone(), request).await?;
        Ok(download_id)
    }

    pub async fn start_download(&self, id: String, request: DownloadRequest) -> Result<()> {
        let permit = self.semaphore.clone().acquire_owned().await?;
        let engine = Arc::clone(&self.engine);
        let progress_sender = self.progress_sender.clone();
        let active_downloads = Arc::clone(&self.active_downloads);
        let id_clone = id.clone();

        let task = DownloadTask {
            id: id.clone(),
            request,
            progress_sender: progress_sender.clone(),
        };

        let handle = tokio::spawn(async move {
            let _permit = permit; // Keep permit until download completes
            let result = engine.download(task).await;

            // Remove from active downloads when complete
            active_downloads.write().await.remove(&id_clone);

            // Send final result if not already sent
            if result.success {
                // The engine already sends Completed progress
            } else {
                // The engine already sends Failed progress
            }
        });

        self.active_downloads.write().await.insert(id, handle);
        Ok(())
    }

    pub async fn cancel_download(&self, id: &str) -> Result<()> {
        let mut downloads = self.active_downloads.write().await;
        if let Some(handle) = downloads.remove(id) {
            handle.abort();
            let _ = self
                .progress_sender
                .send(DownloadProgress::Cancelled { id: id.to_string() });
        }
        Ok(())
    }

    pub async fn get_active_downloads(&self) -> Vec<String> {
        self.active_downloads.read().await.keys().cloned().collect()
    }

    fn select_best_source<'a>(
        &self,
        sources: &'a [DownloadSource],
        options: &DownloadOptions,
    ) -> Result<&'a DownloadSource> {
        if sources.is_empty() {
            anyhow::bail!("No download sources available");
        }

        // Sort sources by preference
        let mut sorted_sources = sources.iter().collect::<Vec<_>>();

        if options.prefer_torrents {
            // Prefer torrent/magnet sources
            sorted_sources.sort_by(|a, b| {
                let a_score = match a.source_type {
                    SourceType::Torrent | SourceType::Magnet => 1000,
                    _ => 0,
                } + a.get_selection_score();

                let b_score = match b.source_type {
                    SourceType::Torrent | SourceType::Magnet => 1000,
                    _ => 0,
                } + b.get_selection_score();

                b_score.cmp(&a_score)
            });
        } else {
            // Prefer HTTP sources
            sorted_sources.sort_by(|a, b| {
                let a_score = match a.source_type {
                    SourceType::Direct | SourceType::Mirror => 1000,
                    _ => 0,
                } + a.get_selection_score();

                let b_score = match b.source_type {
                    SourceType::Direct | SourceType::Mirror => 1000,
                    _ => 0,
                } + b.get_selection_score();

                b_score.cmp(&a_score)
            });
        }

        // Return the best usable source
        sorted_sources
            .iter()
            .find(|s| {
                s.is_usable() && matches!(s.source_type, SourceType::Direct | SourceType::Mirror)
            })
            .copied()
            .context("No usable HTTP sources found")
    }

    fn resolve_url_template(&self, url: &str, iso_info: &IsoInfo) -> Result<String> {
        let mut resolved = url.to_string();

        resolved = resolved.replace("{version}", &iso_info.version);
        resolved = resolved.replace("{arch}", &iso_info.architecture);
        resolved = resolved.replace("{filename}", &iso_info.filename);

        if let Some(variant) = &iso_info.variant {
            resolved = resolved.replace("{variant}", variant);
        } else {
            // Remove variant placeholders if no variant
            resolved = resolved.replace("/{variant}", "");
            resolved = resolved.replace("{variant}/", "");
            resolved = resolved.replace("{variant}", "");
        }

        Ok(resolved)
    }
}
