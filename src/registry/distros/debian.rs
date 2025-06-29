use super::*;
use anyhow::Result;

pub fn create_definition() -> Result<DistroDefinition> {
    // Debian version detector - combines official sources
    let version_detector = detectors::composite()
        .add_detector(detectors::web_scraper(
            "https://www.debian.org/releases/",
            ".release-info .version",
            r"(\d+\.\d+)",
        ))
        .add_detector(detectors::rss_feed(
            "https://www.debian.org/News/news",
            r"Debian (\d+\.\d+)",
            ReleaseType::Stable,
        ))
        // Static versions for stable Debian releases
        .add_detector(detectors::static_versions(vec![
            VersionInfo::new("12.11.0", ReleaseType::Stable)
                .with_release_date("2024-12-15")
                .with_download_base("https://cdimage.debian.org/debian-cd/current/")
                .with_notes("Bookworm - Current stable"),
            VersionInfo::new("12.10.0", ReleaseType::Stable)
                .with_release_date("2024-11-09")
                .with_download_base("https://cdimage.debian.org/debian-cd/12.10.0/")
                .with_notes("Bookworm - Previous stable"),
            VersionInfo::new("11.11.0", ReleaseType::Stable)
                .with_release_date("2024-11-09")
                .with_download_base("https://cdimage.debian.org/debian-cd/11.11.0/")
                .with_notes("Bullseye - Oldstable"),
            VersionInfo::new("13.0", ReleaseType::Beta)
                .with_notes("Trixie - Testing")
                .with_download_base("https://cdimage.debian.org/cdimage/weekly-builds/"),
        ]));

    // Debian download sources - official and mirrors
    let download_sources = vec![
        // Official Debian CD images
        DownloadSource::direct(
            "https://cdimage.debian.org/debian-cd/current/{arch}/iso-cd/{filename}",
            SourcePriority::Preferred,
        )
        .with_description("Official Debian CD images")
        .verified(),
        // Alternative official path for DVD images
        DownloadSource::direct(
            "https://cdimage.debian.org/debian-cd/current/{arch}/iso-dvd/{filename}",
            SourcePriority::Preferred,
        )
        .with_description("Official Debian DVD images")
        .verified(),
        // Major Debian mirrors
        DownloadSource::mirror(
            "https://mirrors.kernel.org/debian-cd/current/{arch}/iso-cd/{filename}",
            SourcePriority::High,
            Some("US"),
        )
        .with_description("Kernel.org mirror")
        .with_speed_rating(9),
        DownloadSource::mirror(
            "https://debian.osuosl.org/debian-cd/current/{arch}/iso-cd/{filename}",
            SourcePriority::High,
            Some("US"),
        )
        .with_description("Oregon State University mirror")
        .with_speed_rating(8),
        DownloadSource::mirror(
            "https://ftp.debian.org/debian-cd/current/{arch}/iso-cd/{filename}",
            SourcePriority::Medium,
            Some("EU"),
        )
        .with_description("Official FTP mirror"),
        DownloadSource::mirror(
            "https://mirror.aarnet.edu.au/pub/debian-cd/current/{arch}/iso-cd/{filename}",
            SourcePriority::Medium,
            Some("AU"),
        )
        .with_description("AARNet Australian mirror"),
        DownloadSource::mirror(
            "https://ftp.jaist.ac.jp/pub/Linux/debian-cd/current/{arch}/iso-cd/{filename}",
            SourcePriority::Medium,
            Some("JP"),
        )
        .with_description("JAIST Japan mirror"),
        // Debian also provides torrents
        DownloadSource::torrent(
            "https://cdimage.debian.org/debian-cd/current/{arch}/bt-cd/{filename}.torrent",
            SourcePriority::High,
        )
        .with_description("Official Debian torrent"),
    ];

    Ok(DistroDefinition {
        name: "debian".to_string(),
        display_name: "Debian".to_string(),
        description: "The universal operating system - a stable, free Linux distribution"
            .to_string(),
        homepage: "https://www.debian.org".to_string(),
        supported_architectures: vec![
            "amd64".to_string(),
            "i386".to_string(),
            "arm64".to_string(),
            "armel".to_string(),
            "armhf".to_string(),
            "mips64el".to_string(),
            "mipsel".to_string(),
            "ppc64el".to_string(),
            "s390x".to_string(),
        ],
        supported_variants: vec![
            "netinst".to_string(),  // Network installer (most common)
            "cd".to_string(),       // CD-sized image
            "dvd".to_string(),      // DVD-sized image
            "live".to_string(),     // Live image
            "firmware".to_string(), // Non-free firmware included
        ],
        version_detector: Box::new(version_detector),
        download_sources,
        filename_pattern: "debian-{version}-{arch}-{variant}.iso".to_string(),
        default_variant: Some("netinst".to_string()),
        checksum_urls: vec![
            "https://cdimage.debian.org/debian-cd/current/{arch}/iso-cd/SHA256SUMS".to_string(),
            "https://cdimage.debian.org/debian-cd/current/{arch}/iso-cd/SHA512SUMS".to_string(),
            "https://cdimage.debian.org/debian-cd/current/{arch}/iso-cd/MD5SUMS".to_string(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debian_definition_creation() {
        let definition = create_definition().unwrap();

        assert_eq!(definition.name, "debian");
        assert_eq!(definition.display_name, "Debian");
        assert!(
            definition
                .supported_architectures
                .contains(&"amd64".to_string())
        );
        assert!(
            definition
                .supported_variants
                .contains(&"netinst".to_string())
        );
        assert_eq!(definition.default_variant, Some("netinst".to_string()));
    }

    #[test]
    fn test_debian_architectures() {
        let definition = create_definition().unwrap();

        // Debian supports many architectures
        assert!(
            definition
                .supported_architectures
                .contains(&"amd64".to_string())
        );
        assert!(
            definition
                .supported_architectures
                .contains(&"arm64".to_string())
        );
        assert!(
            definition
                .supported_architectures
                .contains(&"i386".to_string())
        );
        assert!(definition.supported_architectures.len() >= 8);
    }

    #[test]
    fn test_debian_filename_pattern() {
        let definition = create_definition().unwrap();
        assert_eq!(
            definition.filename_pattern,
            "debian-{version}-{arch}-{variant}.iso"
        );
    }

    #[test]
    fn test_debian_variants() {
        let definition = create_definition().unwrap();

        // Check common Debian variants
        assert!(
            definition
                .supported_variants
                .contains(&"netinst".to_string())
        );
        assert!(definition.supported_variants.contains(&"cd".to_string()));
        assert!(definition.supported_variants.contains(&"dvd".to_string()));
        assert!(definition.supported_variants.contains(&"live".to_string()));
    }

    #[tokio::test]
    async fn test_debian_version_detection() {
        let definition = create_definition().unwrap();

        let result = definition.version_detector.detect_versions().await;
        assert!(result.is_ok());

        let versions = result.unwrap();
        assert!(!versions.is_empty());

        // Should have current stable (12.x series)
        let has_bookworm = versions.iter().any(|v| v.version.starts_with("12."));
        assert!(has_bookworm, "Should have Debian 12 (Bookworm)");
    }
}
