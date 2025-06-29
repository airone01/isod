use super::*;
use anyhow::Result;

pub fn create_definition() -> Result<DistroDefinition> {
    // Create version detector that combines multiple sources
    let version_detector = detectors::composite()
        // Try GitHub releases for Ubuntu releases (unofficial but useful)
        .add_detector(detectors::github("canonical", "ubuntu-releases", false))
        // Web scraping from Ubuntu releases page
        .add_detector(detectors::web_scraper(
            "https://releases.ubuntu.com/",
            ".release-row .version",
            r"(\d+\.\d+(?:\.\d+)?)",
        ))
        // Fallback with known LTS versions
        .add_detector(detectors::static_versions(vec![
            VersionInfo::new("24.04", ReleaseType::LTS)
                .with_release_date("2024-04-25")
                .with_download_base("https://releases.ubuntu.com/24.04/")
                .with_notes("Noble Numbat - Long Term Support"),
            VersionInfo::new("23.10", ReleaseType::Stable)
                .with_release_date("2023-10-12")
                .with_download_base("https://releases.ubuntu.com/23.10/")
                .with_notes("Mantic Minotaur"),
            VersionInfo::new("22.04", ReleaseType::LTS)
                .with_release_date("2022-04-21")
                .with_download_base("https://releases.ubuntu.com/22.04/")
                .with_notes("Jammy Jellyfish - Long Term Support"),
            VersionInfo::new("20.04", ReleaseType::LTS)
                .with_release_date("2020-04-23")
                .with_download_base("https://releases.ubuntu.com/20.04/")
                .with_notes("Focal Fossa - Long Term Support"),
        ]));

    // Define download sources with mirrors and official sources
    let download_sources = vec![
        // Official Ubuntu releases
        DownloadSource::direct(
            "https://releases.ubuntu.com/{version}/{filename}",
            SourcePriority::Preferred,
        )
        .with_description("Official Ubuntu releases")
        .verified(),
        // Old releases archive
        DownloadSource::direct(
            "https://old-releases.ubuntu.com/releases/{version}/{filename}",
            SourcePriority::High,
        )
        .with_description("Ubuntu old releases archive")
        .verified(),
        // Ubuntu mirrors
        DownloadSource::mirror(
            "https://mirror.arizona.edu/ubuntu-releases/{version}/{filename}",
            SourcePriority::High,
            Some("US"),
        )
        .with_description("University of Arizona mirror")
        .with_speed_rating(8),
        DownloadSource::mirror(
            "https://ftp.halifax.rwth-aachen.de/ubuntu-releases/{version}/{filename}",
            SourcePriority::High,
            Some("DE"),
        )
        .with_description("RWTH Aachen mirror")
        .with_speed_rating(9),
        DownloadSource::mirror(
            "https://mirror.us.leaseweb.net/ubuntu-releases/{version}/{filename}",
            SourcePriority::Medium,
            Some("US"),
        )
        .with_description("Leaseweb US mirror")
        .with_speed_rating(7),
        DownloadSource::mirror(
            "https://mirror.nl.leaseweb.net/ubuntu-releases/{version}/{filename}",
            SourcePriority::Medium,
            Some("EU"),
        )
        .with_description("Leaseweb Netherlands mirror")
        .with_speed_rating(7),
        DownloadSource::mirror(
            "https://ubuntu.mirror.constant.com/{version}/{filename}",
            SourcePriority::Medium,
            Some("US"),
        )
        .with_description("Constant.com mirror"),
        DownloadSource::mirror(
            "https://mirror.aarnet.edu.au/pub/ubuntu/releases/{version}/{filename}",
            SourcePriority::Medium,
            Some("AU"),
        )
        .with_description("AARNet Australian mirror"),
        // Torrent sources (Ubuntu often provides torrents for popular releases)
        DownloadSource::torrent(
            "https://releases.ubuntu.com/{version}/{filename}.torrent",
            SourcePriority::High,
        )
        .with_description("Official Ubuntu torrent"),
    ];

    Ok(DistroDefinition {
        name: "ubuntu".to_string(),
        display_name: "Ubuntu".to_string(),
        description: "A popular, user-friendly Linux distribution based on Debian".to_string(),
        homepage: "https://ubuntu.com".to_string(),
        supported_architectures: vec![
            "amd64".to_string(),
            "arm64".to_string(),
            "armhf".to_string(),
            "ppc64el".to_string(),
            "s390x".to_string(),
        ],
        supported_variants: vec![
            "desktop".to_string(),
            "server".to_string(),
            "live-server".to_string(),
        ],
        version_detector: Box::new(version_detector),
        download_sources,
        filename_pattern: "ubuntu-{version}-{variant}-{arch}.iso".to_string(),
        default_variant: Some("desktop".to_string()),
        checksum_urls: vec![
            "https://releases.ubuntu.com/{version}/SHA256SUMS".to_string(),
            "https://releases.ubuntu.com/{version}/MD5SUMS".to_string(),
            "https://old-releases.ubuntu.com/releases/{version}/SHA256SUMS".to_string(),
        ],
    })
}
