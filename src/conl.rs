use std::fs;
use zed_extension_api::{self as zed, GithubReleaseOptions, LanguageServerId, Result};

struct ConlExtension {
    cached_binary_path: Option<String>,
}

impl ConlExtension {
    fn language_server_binary_path(
        &mut self,
        language_server_id: &LanguageServerId,
        _worktree: &zed::Worktree,
    ) -> Result<String> {
        if let Some(path) = &self.cached_binary_path {
            return Ok(path.clone());
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        let release = zed::latest_github_release(
            "ConradIrwin/conl-lsp",
            GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        let (platform, arch) = zed::current_platform();
        let asset_name = match (platform, arch) {
            (zed::Os::Mac, zed::Architecture::Aarch64) => "conl-lsp-darwin-arm64",
            (zed::Os::Mac, zed::Architecture::X8664) => "conl-lsp-darwin-amd64",
            (zed::Os::Linux, zed::Architecture::Aarch64) => "conl-lsp-linux-arm64",
            (zed::Os::Linux, zed::Architecture::X8664) => "conl-lsp-linux-amd64",
            (zed::Os::Windows, zed::Architecture::Aarch64) => "conl-lsp-windows-arm64",
            (zed::Os::Windows, zed::Architecture::X8664) => "conl-lsp-windows-amd64",
            _ => return Err(format!("Unsupported platform: {:?} {:?}", platform, arch)),
        };

        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .ok_or_else(|| format!("No asset found for {}", asset_name))?;

        let version_dir = format!("conl-lsp-{}", release.version);
        std::fs::create_dir_all(&version_dir)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
        let binary_path = format!("{}/{}", version_dir, asset_name);

        if !fs::metadata(&binary_path)
            .map(|stat| stat.is_file())
            .unwrap_or(false)
        {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            zed::download_file(
                &asset.download_url,
                &binary_path,
                zed::DownloadedFileType::Uncompressed,
            )
            .map_err(|e| format!("Failed to download file: {}", e))?;

            zed::make_file_executable(&binary_path)?;

            let downloaded = fs::metadata(&binary_path)
                .map(|stat| stat.is_file())
                .unwrap_or(false);
            if downloaded {
                zed::set_language_server_installation_status(
                    language_server_id,
                    &zed::LanguageServerInstallationStatus::None,
                );
            } else {
                zed::set_language_server_installation_status(
                    language_server_id,
                    &zed::LanguageServerInstallationStatus::Failed(
                        "Failed to download language server".to_string(),
                    ),
                );
                return Err("Failed to download language server".to_string());
            }
        }

        self.cached_binary_path = Some(binary_path.clone());
        Ok(binary_path)
    }
}

impl zed::Extension for ConlExtension {
    fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let binary_path = self.language_server_binary_path(language_server_id, worktree)?;

        Ok(zed::Command {
            command: binary_path,
            args: vec![],
            env: Default::default(),
        })
    }
}

zed::register_extension!(ConlExtension);
