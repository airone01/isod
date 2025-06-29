use anyhow::{Context, Result};
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, BufReader};

#[derive(Debug, Clone, Copy)]
pub enum ChecksumType {
    Md5,
    Sha1,
    Sha256,
    Sha512,
}

pub struct ChecksumVerifier;

impl ChecksumVerifier {
    pub async fn verify_file(
        file_path: &Path,
        expected_checksum: &str,
        checksum_type: ChecksumType,
    ) -> Result<bool> {
        let actual_checksum = Self::calculate_checksum(file_path, checksum_type).await?;
        Ok(actual_checksum.to_lowercase() == expected_checksum.to_lowercase())
    }

    pub async fn calculate_checksum(
        file_path: &Path,
        checksum_type: ChecksumType,
    ) -> Result<String> {
        let file = File::open(file_path)
            .await
            .with_context(|| format!("Failed to open file: {:?}", file_path))?;

        let mut reader = BufReader::new(file);
        let mut buffer = [0; 8192];

        match checksum_type {
            ChecksumType::Sha256 => {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                loop {
                    let bytes_read = reader.read(&mut buffer).await?;
                    if bytes_read == 0 {
                        break;
                    }
                    hasher.update(&buffer[..bytes_read]);
                }
                Ok(format!("{:x}", hasher.finalize()))
            }
            ChecksumType::Sha512 => {
                use sha2::{Digest, Sha512};
                let mut hasher = Sha512::new();
                loop {
                    let bytes_read = reader.read(&mut buffer).await?;
                    if bytes_read == 0 {
                        break;
                    }
                    hasher.update(&buffer[..bytes_read]);
                }
                Ok(format!("{:x}", hasher.finalize()))
            }
            ChecksumType::Md5 => {
                let mut context = md5::Context::new();
                loop {
                    let bytes_read = reader.read(&mut buffer).await?;
                    if bytes_read == 0 {
                        break;
                    }
                    context.consume(&buffer[..bytes_read]);
                }
                let digest = context.finalize();
                Ok(format!("{:x}", digest))
            }
            ChecksumType::Sha1 => {
                use sha1::{Digest, Sha1};
                let mut hasher = Sha1::new();
                loop {
                    let bytes_read = reader.read(&mut buffer).await?;
                    if bytes_read == 0 {
                        break;
                    }
                    hasher.update(&buffer[..bytes_read]);
                }
                Ok(format!("{:x}", hasher.finalize()))
            }
        }
    }
}
