use super::*;
use anyhow::Result;

pub fn create_definition() -> Result<DistroDefinition> {
    // Arch Linux version detector - rolling release with monthly ISOs
    let version_detector = detectors::composite()
        .add_detector(detectors::web_scraper(
            "https://archlinux.org/download/",
            ".download-info",
            r"(\d{4}\.\d{2}\.\d{2})",
        ))
        .add_detector(detectors::rss_feed(
            "https://archlinux.org/feeds/news/",
            r"(\d{4}\.\d{2}\.\d{2})",
            ReleaseType::Stable,
        ))
        // GitLab API for arch-release-dates (if available)
        .add_detector(detectors::api(
            "https://gitlab.archlinux.org/api/v4/projects/archlinux%2Farch-release-dates/repository/tags",
            "$.[*].name",
        ))
        // Static fallback with recent monthly releases
        .add_detector(detectors::static_versions(vec![
            VersionInfo::new("2024.06.01", ReleaseType::Stable)
                .with_release_date("2024-06-01")
                .with_download_base("https://archive.archlinux.org/iso/2024.06.01/")
                .with_notes("Monthly rolling release"),
            VersionInfo::new("2024.05.01", ReleaseType::Stable)
                .with_release_date("2024-05-01")
                .with_download_base("https://archive.archlinux.org/iso/2024.05.01/"),
            VersionInfo::new("2024.04.01", ReleaseType::Stable)
                .with_release_date("2024-04-01")
                .with_download_base("https://archive.archlinux.org/iso/2024.04.01/"),
            // Current latest
            VersionInfo::new("latest", ReleaseType::Stable)
                .with_notes("Latest rolling release")
                .with_download_base("https://archlinux.org/iso/latest/"),
        ]));

    // Arch Linux download sources
    let download_sources = vec![
        // Official Arch mirrors
        DownloadSource::direct(
            "https://archlinux.org/iso/latest/{filename}",
            SourcePriority::Preferred,
        )
        .with_description("Official Arch Linux downloads")
        .verified(),
        // Archive for specific versions
        DownloadSource::direct(
            "https://archive.archlinux.org/iso/{version}/{filename}",
            SourcePriority::Preferred,
        )
        .with_description("Arch Linux archive")
        .verified(),
        // Major mirrors
        DownloadSource::mirror(
            "https://mirrors.kernel.org/archlinux/iso/latest/{filename}",
            SourcePriority::High,
            Some("US"),
        )
        .with_description("Kernel.org mirror")
        .with_speed_rating(9),
        DownloadSource::mirror(
            "https://mirror.rackspace.com/archlinux/iso/latest/{filename}",
            SourcePriority::High,
            Some("US"),
        )
        .with_description("Rackspace mirror")
        .with_speed_rating(8),
        DownloadSource::mirror(
            "https://america.mirror.pkgbuild.com/iso/latest/{filename}",
            SourcePriority::High,
            Some("US"),
        )
        .with_description("Official US mirror"),
        DownloadSource::mirror(
            "https://europe.mirror.pkgbuild.com/iso/latest/{filename}",
            SourcePriority::High,
            Some("EU"),
        )
        .with_description("Official EU mirror"),
        DownloadSource::mirror(
            "https://asia.mirror.pkgbuild.com/iso/latest/{filename}",
            SourcePriority::High,
            Some("AS"),
        )
        .with_description("Official Asia mirror"),
        DownloadSource::mirror(
            "https://ftp.jaist.ac.jp/pub/Linux/ArchLinux/iso/latest/{filename}",
            SourcePriority::Medium,
            Some("JP"),
        )
        .with_description("JAIST Japan mirror"),
        DownloadSource::mirror(
            "https://mirror.aarnet.edu.au/pub/archlinux/iso/latest/{filename}",
            SourcePriority::Medium,
            Some("AU"),
        )
        .with_description("AARNet Australian mirror"),
        // Torrents - Arch provides magnetlinks
        DownloadSource::magnet(
            "magnet:?xt=urn:btih:PLACEHOLDER&dn={filename}",
            SourcePriority::High,
            vec![
                "udp://tracker.archlinux.org:6969".to_string(),
                "udp://tracker.openbittorrent.com:80".to_string(),
                "udp://tracker.publicbt.com:80".to_string(),
            ],
        )
        .with_description("Arch Linux BitTorrent"),
    ];

    Ok(DistroDefinition {
        name: "arch".to_string(),
        display_name: "Arch Linux".to_string(),
        description:
            "A lightweight and flexible Linux distribution that follows the rolling release model"
                .to_string(),
        homepage: "https://archlinux.org".to_string(),
        supported_architectures: vec![
            "x86_64".to_string(),
            // Arch officially only supports x86_64 for the main distribution
            // ARM variants exist but are separate projects (Arch Linux ARM)
        ],
        supported_variants: vec![
            "base".to_string(), // Standard installation ISO
                                // Arch doesn't really have variants like other distros
                                // The ISO is a base system that you build upon
        ],
        version_detector: Box::new(version_detector),
        download_sources,
        filename_pattern: "archlinux-{version}-{arch}.iso".to_string(),
        default_variant: Some("base".to_string()),
        checksum_urls: vec![
            "https://archlinux.org/iso/latest/sha256sums.txt".to_string(),
            "https://archlinux.org/iso/latest/b2sums.txt".to_string(),
            "https://archive.archlinux.org/iso/{version}/sha256sums.txt".to_string(),
        ],
    })
}
