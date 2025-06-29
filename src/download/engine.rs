use anyhow::{Context, Result, bail};
use futures_util::StreamExt;
use reqwest::Client;
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::sleep;

use super::{ChecksumVerifier, DownloadProgress, DownloadRequest};

#[derive(Debug)]
pub struct DownloadResult {
    pub success: bool,
    pub bytes_downloaded: u64,
    pub duration: Duration,
    pub error: Option<String>,
    pub checksum_verified: bool,
}

#[derive(Debug)]
pub struct DownloadTask {
    pub id: String,
    pub request: DownloadRequest,
    pub progress_sender: mpsc::UnboundedSender<DownloadProgress>,
}

pub struct DownloadEngine {
    client: Client,
    max_retries: u32,
    retry_delay: Duration,
}

impl DownloadEngine {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("isod/0.1.0")
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            max_retries: 3,
            retry_delay: Duration::from_secs(2),
        })
    }

    pub async fn download(&self, task: DownloadTask) -> DownloadResult {
        let start_time = Instant::now();
        let mut attempt = 0;

        // Send initial progress
        let _ = task.progress_sender.send(DownloadProgress::Started {
            id: task.id.clone(),
            url: task.request.url.clone(),
            output_path: task.request.output_path.clone(),
        });

        loop {
            attempt += 1;

            match self.download_attempt(&task).await {
                Ok(result) => {
                    let duration = start_time.elapsed();

                    // Verify checksum if provided
                    let checksum_verified = if let (Some(expected), Some(checksum_type)) =
                        (&task.request.expected_checksum, &task.request.checksum_type)
                    {
                        let _ = task
                            .progress_sender
                            .send(DownloadProgress::VerifyingChecksum {
                                id: task.id.clone(),
                            });

                        match ChecksumVerifier::verify_file(
                            &task.request.output_path,
                            expected,
                            *checksum_type,
                        )
                        .await
                        {
                            Ok(verified) => {
                                if verified {
                                    let _ = task.progress_sender.send(
                                        DownloadProgress::ChecksumVerified {
                                            id: task.id.clone(),
                                        },
                                    );
                                } else {
                                    let _ = task.progress_sender.send(
                                        DownloadProgress::ChecksumFailed {
                                            id: task.id.clone(),
                                            expected: expected.clone(),
                                        },
                                    );
                                }
                                verified
                            }
                            Err(e) => {
                                let _ = task.progress_sender.send(DownloadProgress::Error {
                                    id: task.id.clone(),
                                    error: format!("Checksum verification failed: {}", e),
                                });
                                false
                            }
                        }
                    } else {
                        true // No checksum to verify
                    };

                    let _ = task.progress_sender.send(DownloadProgress::Completed {
                        id: task.id.clone(),
                        bytes_downloaded: result,
                        checksum_verified,
                    });

                    return DownloadResult {
                        success: true,
                        bytes_downloaded: result,
                        duration,
                        error: None,
                        checksum_verified,
                    };
                }
                Err(e) => {
                    if attempt >= self.max_retries {
                        let _ = task.progress_sender.send(DownloadProgress::Failed {
                            id: task.id.clone(),
                            error: e.to_string(),
                            attempts: attempt,
                        });

                        return DownloadResult {
                            success: false,
                            bytes_downloaded: 0,
                            duration: start_time.elapsed(),
                            error: Some(e.to_string()),
                            checksum_verified: false,
                        };
                    }

                    let _ = task.progress_sender.send(DownloadProgress::Retry {
                        id: task.id.clone(),
                        attempt,
                        max_attempts: self.max_retries,
                        delay: self.retry_delay,
                    });

                    sleep(self.retry_delay).await;
                }
            }
        }
    }

    async fn download_attempt(&self, task: &DownloadTask) -> Result<u64> {
        let request = &task.request;

        // Check if file exists and we should resume
        let (resume_from, existing_size) = if request.resume && request.output_path.exists() {
            let metadata = std::fs::metadata(&request.output_path)
                .context("Failed to get existing file metadata")?;
            (metadata.len(), metadata.len())
        } else {
            (0, 0)
        };

        // Build request with range header for resume
        let mut req_builder = self.client.get(&request.url);

        if let Some(user_agent) = &request.user_agent {
            req_builder = req_builder.header("User-Agent", user_agent);
        }

        if resume_from > 0 {
            req_builder = req_builder.header("Range", format!("bytes={}-", resume_from));
        }

        let response = req_builder
            .send()
            .await
            .context("Failed to send HTTP request")?;

        if !response.status().is_success() && response.status().as_u16() != 206 {
            bail!("HTTP request failed with status: {}", response.status());
        }

        let total_size = if resume_from > 0 {
            // For resume, get content-range header
            response
                .headers()
                .get("content-range")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| {
                    // Parse "bytes 1024-2047/2048" format
                    s.split('/').nth(1)?.parse().ok()
                })
                .unwrap_or(existing_size + response.content_length().unwrap_or(0))
        } else {
            response.content_length().unwrap_or(0)
        };

        // Open file for writing
        let mut file = if resume_from > 0 {
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .append(true)
                .open(&request.output_path)
                .context("Failed to open file for resume")?;
            f.seek(SeekFrom::End(0))
                .context("Failed to seek to end of file")?;
            f
        } else {
            // Create parent directories if they don't exist
            if let Some(parent) = request.output_path.parent() {
                std::fs::create_dir_all(parent).context("Failed to create parent directories")?;
            }

            File::create(&request.output_path).context("Failed to create output file")?
        };

        let mut downloaded = existing_size;
        let mut last_progress_update = Instant::now();
        let mut last_bytes = downloaded;
        const PROGRESS_UPDATE_INTERVAL: Duration = Duration::from_millis(250);

        // Download with progress tracking
        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.context("Failed to read chunk from response")?;

            file.write_all(&chunk)
                .context("Failed to write chunk to file")?;

            downloaded += chunk.len() as u64;

            // Send progress updates periodically
            if last_progress_update.elapsed() >= PROGRESS_UPDATE_INTERVAL {
                let progress = if total_size > 0 {
                    (downloaded as f64 / total_size as f64 * 100.0) as u8
                } else {
                    0
                };

                // Calculate speed in bytes per second
                let elapsed = last_progress_update.elapsed().as_secs_f64();
                let speed_bps = if elapsed > 0.0 {
                    ((downloaded - last_bytes) as f64 / elapsed) as u64
                } else {
                    0
                };

                let _ = task.progress_sender.send(DownloadProgress::Progress {
                    id: task.id.clone(),
                    bytes_downloaded: downloaded,
                    total_bytes: total_size,
                    progress_percent: progress,
                    speed_bps,
                });

                last_progress_update = Instant::now();
                last_bytes = downloaded;
            }
        }

        file.flush().context("Failed to flush file")?;
        Ok(downloaded)
    }
}

impl Default for DownloadEngine {
    fn default() -> Self {
        Self::new().expect("Failed to create default DownloadEngine")
    }
}
