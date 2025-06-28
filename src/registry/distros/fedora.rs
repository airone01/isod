use super::*;
use anyhow::Result;

pub fn create_definition() -> Result<DistroDefinition> {
    // Fedora version detector using multiple sources
    let version_detector = detectors::composite()
        .add_detector(detectors::rss_feed(
            "https://fedoraproject.org/wiki/Releases?action=rss",
            r"Fedora (\d+)",
            ReleaseType::Stable,
        ))
        .add_detector(detectors::api(
            "https://bodhi.fedoraproject.org/releases/",
            "$.releases[*].version",
        ))
        // Static fallback with recent Fedora releases
        .add_detector(detectors::static_versions(vec![
            VersionInfo::new("40", ReleaseType::Stable)
                .with_release_date("2024-04-23")
                .with_download_base(
                    "https://download.fedoraproject.org/pub/fedora/linux/releases/40/",
                ),
            VersionInfo::new("39", ReleaseType::Stable)
                .with_release_date("2023-11-07")
                .with_download_base(
                    "https://download.fedoraproject.org/pub/fedora/linux/releases/39/",
                ),
            VersionInfo::new("38", ReleaseType::Stable)
                .with_release_date("2023-04-18")
                .with_download_base(
                    "https://download.fedoraproject.org/pub/fedora/linux/releases/38/",
                ),
            VersionInfo::new("37", ReleaseType::Stable)
                .with_release_date("2022-11-15")
                .with_download_base(
                    "https://download.fedoraproject.org/pub/fedora/linux/releases/37/",
                ),
        ]));

    // Fedora download sources including official and mirrors
    let download_sources = vec![
        // Official Fedora download
        DownloadSource::direct(
            "https://download.fedoraproject.org/pub/fedora/linux/releases/{version}/Workstation/{arch}/iso/{filename}",
            SourcePriority::Preferred
        ).with_description("Official Fedora downloads").verified(),

        // Alternative path for Server variant
        DownloadSource::direct(
            "https://download.fedoraproject.org/pub/fedora/linux/releases/{version}/Server/{arch}/iso/{filename}",
            SourcePriority::Preferred
        ).with_description("Official Fedora Server downloads").verified(),

        // Major mirrors
        DownloadSource::mirror(
            "https://mirrors.kernel.org/fedora/releases/{version}/Workstation/{arch}/iso/{filename}",
            SourcePriority::High,
            Some("US")
        ).with_description("Kernel.org mirror").with_speed_rating(9),

        DownloadSource::mirror(
            "https://fedora.mirror.constant.com/releases/{version}/Workstation/{arch}/iso/{filename}",
            SourcePriority::High,
            Some("US")
        ).with_description("Constant.com mirror"),

        DownloadSource::mirror(
            "https://mirror.aarnet.edu.au/pub/fedora/linux/releases/{version}/Workstation/{arch}/iso/{filename}",
            SourcePriority::Medium,
            Some("AU")
        ).with_description("AARNet mirror"),

        DownloadSource::mirror(
            "https://ftp.fau.de/fedora/linux/releases/{version}/Workstation/{arch}/iso/{filename}",
            SourcePriority::Medium,
            Some("DE")
        ).with_description("University of Erlangen mirror"),

        // Torrent support
        DownloadSource::torrent(
            "https://torrent.fedoraproject.org/torrents/{filename}.torrent",
            SourcePriority::High
        ).with_description("Official Fedora torrent"),
    ];

    Ok(DistroDefinition {
        name: "fedora".to_string(),
        display_name: "Fedora".to_string(),
        description: "A cutting-edge Linux distribution sponsored by Red Hat".to_string(),
        homepage: "https://getfedora.org".to_string(),
        supported_architectures: vec![
            "x86_64".to_string(),
            "aarch64".to_string(),
            "armhfp".to_string(),
            "ppc64le".to_string(),
            "s390x".to_string(),
        ],
        supported_variants: vec![
            "workstation".to_string(),
            "server".to_string(),
            "netinst".to_string(),
            "everything".to_string(),
        ],
        version_detector: Box::new(version_detector),
        download_sources,
        filename_pattern: "Fedora-{variant}-Live-{arch}-{version}-1.5.iso".to_string(),
        default_variant: Some("workstation".to_string()),
        checksum_urls: vec![
            "https://download.fedoraproject.org/pub/fedora/linux/releases/{version}/Workstation/{arch}/iso/Fedora-Workstation-{version}-1.5-{arch}-CHECKSUM".to_string(),
            "https://getfedora.org/static/checksums/Fedora-Workstation-{version}-1.5-{arch}-CHECKSUM".to_string(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fedora_definition_creation() {
        let definition = create_definition().unwrap();

        assert_eq!(definition.name, "fedora");
        assert_eq!(definition.display_name, "Fedora");
        assert!(
            definition
                .supported_architectures
                .contains(&"x86_64".to_string())
        );
        assert!(
            definition
                .supported_variants
                .contains(&"workstation".to_string())
        );
        assert_eq!(definition.default_variant, Some("workstation".to_string()));
    }

    #[test]
    fn test_fedora_filename_pattern() {
        let definition = create_definition().unwrap();
        assert_eq!(
            definition.filename_pattern,
            "Fedora-{variant}-Live-{arch}-{version}-1.5.iso"
        );
    }

    #[test]
    fn test_fedora_architectures() {
        let definition = create_definition().unwrap();

        // Fedora uses x86_64 instead of amd64
        assert!(
            definition
                .supported_architectures
                .contains(&"x86_64".to_string())
        );
        assert!(
            !definition
                .supported_architectures
                .contains(&"amd64".to_string())
        );
    }

    #[tokio::test]
    async fn test_fedora_version_detection() {
        let definition = create_definition().unwrap();

        let result = definition.version_detector.detect_versions().await;
        assert!(result.is_ok());

        let versions = result.unwrap();
        assert!(!versions.is_empty());

        // Check that we have some recent versions
        let has_recent_version = versions
            .iter()
            .any(|v| v.version.parse::<u32>().unwrap_or(0) >= 37);
        assert!(has_recent_version, "Should have Fedora 37 or newer");
    }
}
