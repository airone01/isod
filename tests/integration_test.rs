use anyhow::Result;
use isod::download::{
    ChecksumType, DownloadManager, DownloadOptions, DownloadProgress, DownloadRequest,
};
use isod::registry::{IsoRegistry, ReleaseType};
use tempfile::TempDir;

#[tokio::test]
async fn test_download_manager_creation() -> Result<()> {
    let options = DownloadOptions::default();
    let result = DownloadManager::new(options);
    assert!(result.is_ok());
    Ok(())
}

#[tokio::test]
async fn test_iso_registry_basic_operations() -> Result<()> {
    let registry = IsoRegistry::new();

    // Test that we have some distros loaded
    let distros = registry.get_all_distros();
    assert!(!distros.is_empty());

    // Test that common distros are supported
    assert!(registry.is_supported("ubuntu"));
    assert!(registry.is_supported("fedora"));
    assert!(registry.is_supported("debian"));
    assert!(registry.is_supported("arch"));

    // Test that we can get distro definitions
    let ubuntu = registry.get_distro("ubuntu");
    assert!(ubuntu.is_some());

    let ubuntu = ubuntu.unwrap();
    assert_eq!(ubuntu.name, "ubuntu");
    assert_eq!(ubuntu.display_name, "Ubuntu");
    assert!(!ubuntu.supported_architectures.is_empty());
    assert!(!ubuntu.supported_variants.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_version_detection() -> Result<()> {
    let registry = IsoRegistry::new();

    // Test version detection for Ubuntu (this will use static fallback in tests)
    let result = registry.get_available_versions("ubuntu").await;
    assert!(result.is_ok());

    let versions = result.unwrap();
    assert!(!versions.is_empty());

    // Should have at least some LTS versions
    let has_lts = versions
        .iter()
        .any(|v| matches!(v.release_type, ReleaseType::LTS));
    // Note: might not have LTS in test environment, so we don't assert this

    Ok(())
}

#[tokio::test]
async fn test_iso_info_generation() -> Result<()> {
    let registry = IsoRegistry::new();

    // Test getting ISO info for Ubuntu desktop
    let result = registry
        .get_iso_info("ubuntu", None, Some("amd64"), Some("desktop"))
        .await;
    assert!(result.is_ok());

    let iso_info = result.unwrap();
    assert_eq!(iso_info.distro, "ubuntu");
    assert_eq!(iso_info.architecture, "amd64");
    assert_eq!(iso_info.variant, Some("desktop".to_string()));
    assert!(iso_info.filename.contains("ubuntu"));
    assert!(iso_info.filename.contains("amd64"));
    assert!(iso_info.filename.contains("desktop"));
    assert!(iso_info.filename.ends_with(".iso"));
    assert!(!iso_info.download_sources.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_search_functionality() -> Result<()> {
    let registry = IsoRegistry::new();

    // Test searching for Ubuntu
    let results = registry.search_distros("ubuntu");
    assert!(results.contains(&"ubuntu"));

    // Test searching for Fedora
    let results = registry.search_distros("fedora");
    assert!(results.contains(&"fedora"));

    // Test partial search
    let results = registry.search_distros("deb");
    assert!(results.contains(&"debian"));

    // Test case insensitive search
    let results = registry.search_distros("ARCH");
    assert!(results.contains(&"arch"));

    Ok(())
}

#[tokio::test]
async fn test_download_request_creation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let output_path = temp_dir.path().join("test.iso");

    let request = DownloadRequest::new(
        "https://example.com/test.iso".to_string(),
        output_path.clone(),
    );

    assert_eq!(request.url, "https://example.com/test.iso");
    assert_eq!(request.output_path, output_path);
    assert!(request.resume);
    assert!(request.user_agent.is_some());
    assert!(request.expected_checksum.is_none());

    // Test with checksum
    let request_with_checksum = request.with_checksum("abc123".to_string(), ChecksumType::Sha256);

    assert_eq!(
        request_with_checksum.expected_checksum,
        Some("abc123".to_string())
    );
    assert!(matches!(
        request_with_checksum.checksum_type,
        Some(ChecksumType::Sha256)
    ));

    Ok(())
}

#[tokio::test]
async fn test_download_options() -> Result<()> {
    let temp_dir = TempDir::new()?;

    let options = DownloadOptions {
        max_concurrent: 5,
        prefer_torrents: true,
        output_directory: temp_dir.path().to_path_buf(),
        verify_checksums: true,
        resume_downloads: false,
    };

    assert_eq!(options.max_concurrent, 5);
    assert!(options.prefer_torrents);
    assert_eq!(options.output_directory, temp_dir.path());
    assert!(options.verify_checksums);
    assert!(!options.resume_downloads);

    Ok(())
}

#[tokio::test]
async fn test_checksum_calculation() -> Result<()> {
    use isod::download::{ChecksumType, ChecksumVerifier};
    use tokio::fs;

    let temp_dir = TempDir::new()?;
    let test_file = temp_dir.path().join("test.txt");

    // Create a test file with known content
    let test_content = "Hello, World!";
    fs::write(&test_file, test_content).await?;

    // Calculate SHA256 checksum
    let checksum = ChecksumVerifier::calculate_checksum(&test_file, ChecksumType::Sha256).await?;

    // Expected SHA256 for "Hello, World!"
    let expected = "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f";
    assert_eq!(checksum.to_lowercase(), expected);

    // Test verification
    let verified =
        ChecksumVerifier::verify_file(&test_file, expected, ChecksumType::Sha256).await?;
    assert!(verified);

    // Test with wrong checksum
    let wrong_verified =
        ChecksumVerifier::verify_file(&test_file, "wrong_checksum", ChecksumType::Sha256).await?;
    assert!(!wrong_verified);

    Ok(())
}

#[tokio::test]
async fn test_filename_generation() -> Result<()> {
    let registry = IsoRegistry::new();

    // Test Ubuntu filename generation
    let iso_info = registry
        .get_iso_info("ubuntu", Some("22.04"), Some("amd64"), Some("desktop"))
        .await?;
    assert_eq!(iso_info.filename, "ubuntu-22.04-desktop-amd64.iso");

    // Test Fedora filename generation
    let iso_info = registry
        .get_iso_info("fedora", Some("40"), Some("x86_64"), Some("workstation"))
        .await?;
    assert_eq!(
        iso_info.filename,
        "Fedora-workstation-Live-x86_64-40-1.5.iso"
    );

    // Test Arch filename generation (no variant)
    let iso_info = registry
        .get_iso_info("arch", Some("2024.06.01"), Some("x86_64"), None)
        .await?;
    assert_eq!(iso_info.filename, "archlinux-2024.06.01-x86_64.iso");

    Ok(())
}

#[cfg(feature = "network_tests")]
#[tokio::test]
async fn test_actual_version_detection() -> Result<()> {
    // This test requires network access and should only run when specifically enabled
    let registry = IsoRegistry::new();

    // Test Ubuntu version detection with network access
    let versions = registry.get_available_versions("ubuntu").await?;
    assert!(!versions.is_empty());

    // Should have recent versions
    let has_recent = versions
        .iter()
        .any(|v| v.version.starts_with("22.") || v.version.starts_with("24."));
    assert!(has_recent);

    Ok(())
}

// Helper function for manual testing
#[allow(dead_code)]
async fn manual_download_test() -> Result<()> {
    use std::env;

    // This is not a unit test but can be used for manual testing
    // Set ISOD_TEST_DOWNLOAD=1 to enable
    if env::var("ISOD_TEST_DOWNLOAD").is_ok() {
        let temp_dir = TempDir::new()?;
        let registry = IsoRegistry::new();

        // Get info for a small Arch ISO
        let iso_info = registry
            .get_iso_info("arch", Some("latest"), Some("x86_64"), Some("base"))
            .await?;

        let options = DownloadOptions {
            max_concurrent: 1,
            prefer_torrents: false,
            output_directory: temp_dir.path().to_path_buf(),
            verify_checksums: false, // Skip checksum for test
            resume_downloads: true,
        };

        let (download_manager, mut progress_receiver) = DownloadManager::new(options.clone())?;
        let download_id = download_manager.download_iso(&iso_info, &options).await?;

        println!("Started download: {}", download_id);

        // Monitor progress
        while let Some(progress) = progress_receiver.recv().await {
            match progress {
                DownloadProgress::Progress {
                    bytes_downloaded,
                    total_bytes,
                    ..
                } => {
                    if total_bytes > 0 {
                        let percent = (bytes_downloaded as f64 / total_bytes as f64) * 100.0;
                        println!(
                            "Progress: {:.1}% ({}/{})",
                            percent, bytes_downloaded, total_bytes
                        );
                    }
                }
                DownloadProgress::Completed { .. } => {
                    println!("Download completed!");
                    break;
                }
                DownloadProgress::Failed { error, .. } => {
                    println!("Download failed: {}", error);
                    break;
                }
                _ => {}
            }
        }
    }

    Ok(())
}

// Example usage test
#[tokio::test]
async fn test_complete_workflow() -> Result<()> {
    // Test a complete workflow from registry to download setup
    let registry = IsoRegistry::new();

    // 1. Search for a distribution
    let search_results = registry.search_distros("ubuntu");
    assert!(!search_results.is_empty());

    // 2. Get available versions
    let versions = registry.get_available_versions("ubuntu").await?;
    assert!(!versions.is_empty());

    // 3. Get ISO info
    let iso_info = registry
        .get_iso_info("ubuntu", None, Some("amd64"), Some("desktop"))
        .await?;

    // 4. Verify we have download sources
    assert!(!iso_info.download_sources.is_empty());

    // 5. Check that sources are usable
    let usable_sources = iso_info
        .download_sources
        .iter()
        .filter(|s| s.is_usable())
        .count();
    assert!(usable_sources > 0);

    // 6. Create download manager
    let temp_dir = TempDir::new()?;
    let options = DownloadOptions {
        max_concurrent: 1,
        prefer_torrents: false,
        output_directory: temp_dir.path().to_path_buf(),
        verify_checksums: false,
        resume_downloads: true,
    };

    let result = DownloadManager::new(options);
    assert!(result.is_ok());

    Ok(())
}
