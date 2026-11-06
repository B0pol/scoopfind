use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents the top-level response envelope from Azure Cognitive Search
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AzureSearchResponse {
    #[serde(default)]
    pub value: Vec<ScoopPackage>,
}

/// Represents an individual Scoop package manifest from the search index
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScoopPackage {
    #[serde(rename = "Id")]
    pub id: String,

    #[serde(rename = "Name")]
    pub name: String,

    #[serde(rename = "Version")]
    pub version: String,

    #[serde(rename = "Description", default)]
    pub description: Option<String>,

    #[serde(rename = "Homepage", default)]
    pub homepage: Option<String>,

    #[serde(rename = "License", default)]
    pub license: Option<String>,

    #[serde(rename = "Notes", default)]
    pub notes: Option<String>,

    #[serde(rename = "Metadata")]
    pub metadata: PackageMetadata,
}

/// Metadata concerning the git repository and bucket hosting the package
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PackageMetadata {
    #[serde(rename = "Repository")]
    pub repository: String,

    #[serde(rename = "OfficialRepository", default)]
    pub official_repository: bool,

    #[serde(rename = "RepositoryStars", default)]
    pub repository_stars: Option<u64>,

    #[serde(rename = "FilePath", default)]
    pub file_path: String,

    #[serde(rename = "Committed", default)]
    pub committed: Option<String>,

    #[serde(rename = "Sha", default)]
    pub sha: Option<String>,
}

pub fn is_bucket_installed_at<P: AsRef<std::path::Path>>(bucket: &str, scoop_base: P) -> bool {
    scoop_base.as_ref().join("buckets").join(bucket).is_dir()
}

pub fn is_bucket_installed(bucket: &str) -> bool {
    let scoop_base = std::env::var("SCOOP")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("USERPROFILE").unwrap_or_default();
            std::path::PathBuf::from(home).join("scoop")
        });

    if is_bucket_installed_at(bucket, &scoop_base) {
        return true;
    }

    let scoop_global = std::env::var("SCOOP_GLOBAL")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            let prog_data = std::env::var("ProgramData").unwrap_or_default();
            std::path::PathBuf::from(prog_data).join("scoop")
        });

    is_bucket_installed_at(bucket, &scoop_global)
}

impl ScoopPackage {
    /// Extracts a user-friendly bucket name from the repository URL.
    /// E.g. "https://github.com/ScoopInstaller/Main" -> "main"
    pub fn bucket_name(&self) -> String {
        let repo = self.metadata.repository.trim_end_matches('/');
        let repo_lower = repo.to_lowercase();

        // Standard official scoop buckets mapping
        if repo_lower.ends_with("/scoopinstaller/main") {
            "main".to_string()
        } else if repo_lower.ends_with("/scoopinstaller/extras") {
            "extras".to_string()
        } else if repo_lower.ends_with("/scoopinstaller/versions") {
            "versions".to_string()
        } else if repo_lower.ends_with("/scoopinstaller/games") {
            "games".to_string()
        } else if repo_lower.ends_with("/scoopinstaller/nerd-fonts") {
            "nerd-fonts".to_string()
        } else if repo_lower.ends_with("/scoopinstaller/sysinternals") {
            "sysinternals".to_string()
        } else if repo_lower.ends_with("/scoopinstaller/java") {
            "java".to_string()
        } else if repo_lower.ends_with("/scoopinstaller/php") {
            "php".to_string()
        } else {
            // Extract the last path component
            repo.split('/')
                .next_back()
                .unwrap_or("unknown")
                .to_lowercase()
        }
    }

    /// Generates the standard Scoop command to install the package, checking
    /// whether the required bucket is already added locally to avoid redundant warnings.
    pub fn install_command_with_bucket_check<F>(&self, is_bucket_present: F) -> String
    where
        F: Fn(&str) -> bool,
    {
        let bucket = self.bucket_name();
        let bucket_installed = bucket == "main" || is_bucket_present(&bucket);

        if bucket_installed {
            if bucket == "main" {
                format!("scoop install {}", self.name)
            } else {
                format!("scoop install {}/{}", bucket, self.name)
            }
        } else if self.metadata.official_repository {
            format!("scoop bucket add {}; scoop install {}/{}", bucket, bucket, self.name)
        } else {
            format!(
                "scoop bucket add {} {}; scoop install {}/{}",
                bucket, self.metadata.repository, bucket, self.name
            )
        }
    }

    /// Generates the standard Scoop command, automatically verifying if the target bucket
    /// is already added in the local Scoop environment.
    pub fn install_command(&self) -> String {
        self.install_command_with_bucket_check(is_bucket_installed)
    }

    /// Parses the ISO 8601 committed timestamp from Azure Search
    pub fn committed_datetime(&self) -> Option<DateTime<Utc>> {
        self.metadata.committed.as_deref().and_then(|s| {
            DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.with_timezone(&Utc))
        })
    }

    /// Full UTC timestamp: "YYYY-MM-DD HH:MM UTC"
    pub fn full_datetime_utc(&self) -> String {
        if let Some(ref date_str) = self.metadata.committed {
            if date_str.len() >= 16 {
                let date_part = &date_str[..10];
                let time_part = &date_str[11..16];
                return format!("{} {} UTC", date_part, time_part);
            }
            return date_str.clone();
        }
        "Unknown".to_string()
    }

    /// Ultra-short relative age for list view (e.g. "now", "4h", "12d", "2mo", "1y")
    pub fn short_relative_age(&self) -> String {
        let Some(dt) = self.committed_datetime() else {
            return "-".to_string();
        };

        let now = Utc::now();
        Self::format_short_relative_age(dt, now)
    }

    pub fn format_short_relative_age(dt: DateTime<Utc>, now: DateTime<Utc>) -> String {
        let duration = now.signed_duration_since(dt);
        let total_seconds = duration.num_seconds();
        if total_seconds < 3600 {
            return "now".to_string();
        }

        let total_hours = total_seconds / 3600;
        if total_hours < 24 {
            return format!("{}h", total_hours);
        }

        let total_days = total_hours / 24;
        if total_days < 30 {
            return format!("{}d", total_days);
        }

        if total_days < 365 {
            let months = total_days / 30;
            return format!("{}mo", months);
        }

        let years = total_days / 365;
        format!("{}y", years)
    }

    /// Full dynamic relative age for details pane:
    /// - "x hours ago"
    /// - "x days ago"
    /// - "x months x days ago"
    /// - "x years x months ago"
    pub fn full_relative_age(&self) -> String {
        let Some(dt) = self.committed_datetime() else {
            return "unknown".to_string();
        };

        let now = Utc::now();
        Self::format_full_relative_age(dt, now)
    }

    pub fn format_full_relative_age(dt: DateTime<Utc>, now: DateTime<Utc>) -> String {
        let duration = now.signed_duration_since(dt);
        let total_seconds = duration.num_seconds();

        if total_seconds < 0 {
            return "just now".to_string();
        }

        let total_minutes = total_seconds / 60;
        let total_hours = total_minutes / 60;
        let total_days = total_hours / 24;

        if total_minutes < 60 {
            if total_minutes <= 1 {
                "just now".to_string()
            } else {
                format!("{} minutes ago", total_minutes)
            }
        } else if total_hours < 24 {
            let unit = if total_hours == 1 { "hour" } else { "hours" };
            format!("{} {} ago", total_hours, unit)
        } else if total_days < 30 {
            let unit = if total_days == 1 { "day" } else { "days" };
            format!("{} {} ago", total_days, unit)
        } else if total_days < 365 {
            let months = total_days / 30;
            let days = total_days % 30;
            let m_unit = if months == 1 { "month" } else { "months" };
            if days == 0 {
                format!("{} {} ago", months, m_unit)
            } else {
                let d_unit = if days == 1 { "day" } else { "days" };
                format!("{} {} {} {} ago", months, m_unit, days, d_unit)
            }
        } else {
            let years = total_days / 365;
            let remaining_days = total_days % 365;
            let months = remaining_days / 30;
            let y_unit = if years == 1 { "year" } else { "years" };
            if months == 0 {
                format!("{} {} ago", years, y_unit)
            } else {
                let m_unit = if months == 1 { "month" } else { "months" };
                format!("{} {} {} {} ago", years, y_unit, months, m_unit)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_short_relative_age_formatting() {
        let now = Utc::now();

        // 30 mins ago -> now
        assert_eq!(ScoopPackage::format_short_relative_age(now - Duration::minutes(30), now), "now");
        // 4 hours ago -> 4h
        assert_eq!(ScoopPackage::format_short_relative_age(now - Duration::hours(4), now), "4h");
        // 12 days ago -> 12d
        assert_eq!(ScoopPackage::format_short_relative_age(now - Duration::days(12), now), "12d");
        // 70 days ago -> 2mo
        assert_eq!(ScoopPackage::format_short_relative_age(now - Duration::days(70), now), "2mo");
        // 400 days ago -> 1y
        assert_eq!(ScoopPackage::format_short_relative_age(now - Duration::days(400), now), "1y");
    }

    #[test]
    fn test_full_relative_age_formatting() {
        let now = Utc::now();

        // 5 hours ago
        assert_eq!(
            ScoopPackage::format_full_relative_age(now - Duration::hours(5), now),
            "5 hours ago"
        );
        // 1 day ago
        assert_eq!(
            ScoopPackage::format_full_relative_age(now - Duration::days(1), now),
            "1 day ago"
        );
        // 15 days ago
        assert_eq!(
            ScoopPackage::format_full_relative_age(now - Duration::days(15), now),
            "15 days ago"
        );
        // 2 months 5 days ago (65 days)
        assert_eq!(
            ScoopPackage::format_full_relative_age(now - Duration::days(65), now),
            "2 months 5 days ago"
        );
        // 1 year 3 months ago (365 + 95 days)
        assert_eq!(
            ScoopPackage::format_full_relative_age(now - Duration::days(460), now),
            "1 year 3 months ago"
        );
    }

    #[test]
    fn test_install_command_generation() {
        let mut pkg = ScoopPackage {
            id: "test-pkg".to_string(),
            name: "deckcraft".to_string(),
            version: "0.15.1".to_string(),
            description: None,
            homepage: None,
            license: None,
            notes: None,
            metadata: PackageMetadata {
                repository: "https://github.com/ScoopInstaller/Extras".to_string(),
                official_repository: true,
                repository_stars: None,
                file_path: "bucket/deckcraft.json".to_string(),
                committed: None,
                sha: None,
            },
        };

        // When bucket "extras" is already installed
        assert_eq!(
            pkg.install_command_with_bucket_check(|b| b == "extras"),
            "scoop install extras/deckcraft"
        );

        // When bucket "extras" is not yet installed
        assert_eq!(
            pkg.install_command_with_bucket_check(|_| false),
            "scoop bucket add extras; scoop install extras/deckcraft"
        );

        // Main bucket packages never need bucket add
        pkg.metadata.repository = "https://github.com/ScoopInstaller/Main".to_string();
        assert_eq!(
            pkg.install_command_with_bucket_check(|_| false),
            "scoop install deckcraft"
        );

        // Community bucket package
        pkg.metadata.repository = "https://github.com/custom/mybucket".to_string();
        pkg.metadata.official_repository = false;
        assert_eq!(
            pkg.install_command_with_bucket_check(|b| b == "mybucket"),
            "scoop install mybucket/deckcraft"
        );
        assert_eq!(
            pkg.install_command_with_bucket_check(|_| false),
            "scoop bucket add mybucket https://github.com/custom/mybucket; scoop install mybucket/deckcraft"
        );
    }

    #[test]
    fn test_is_bucket_installed_at() {
        let temp_dir = std::env::temp_dir().join("scoopfind_bucket_test");
        let buckets_dir = temp_dir.join("buckets");
        let extras_dir = buckets_dir.join("extras");
        let _ = std::fs::create_dir_all(&extras_dir);

        assert!(is_bucket_installed_at("extras", &temp_dir));
        assert!(!is_bucket_installed_at("nonexistent_bucket", &temp_dir));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
