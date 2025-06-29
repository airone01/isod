use super::*;
use anyhow::Result;

pub fn create_definition() -> Result<DistroDefinition> {
    // Fedora version detector using multiple sources
    let version_detector = detectors::composite()
        // Official Fedora API
        .add_detector(detectors::api(
            "https://bodhi.fedoraproject.org/releases/?rows=20",
            "$.releases[*].version",
        ))
        // GitHub releases for Fedora (alternative source)
        .add_detector(detectors::github("fedora-linux", "fedora", false))
        // RSS feed from Fedora Magazine
        .add_detector(detectors::rss_feed(
            "https://fedoramagazine.org/feed/",
            r"Fedora (\d+)",
            ReleaseType::Stable,
        ))
        // Static fallback with recent Fedora releases
        .add_detector(detectors::static_versions(vec![
            VersionInfo::new("40", ReleaseType::Stable)
                .with_release_date("2024-04-23")
                .with_download_base(
                    "https://download.fedoraproject.org/pub/fedora/linux/releases/40/",
                )
                .with_notes("Latest stable release"),
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
        ).with_description("Constant.com mirror")
        .with_speed_rating(8),

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

        DownloadSource::mirror(
            "https://fedora.ip-connect.info/releases/{version}/Workstation/{arch}/iso/{filename}",
            SourcePriority::Medium,
            Some("EU")
        ).with_description("IP-Connect mirror"),

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
