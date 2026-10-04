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

    /// Generates the standard Scoop command to add bucket and install the package
    pub fn install_command(&self) -> String {
        let bucket = self.bucket_name();
        if self.metadata.official_repository {
            if bucket == "main" {
                format!("scoop install {}", self.name)
            } else {
                format!("scoop bucket add {}; scoop install {}/{}", bucket, bucket, self.name)
            }
        } else {
            format!(
                "scoop bucket add {} {}; scoop install {}/{}",
                bucket, self.metadata.repository, bucket, self.name
            )
        }
    }
}
