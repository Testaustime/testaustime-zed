use std::{
    fs,
    path::{Path, PathBuf},
};

use testaustime_shared::TestaustimeSettings;
use zed_extension_api::{self as zed, Command, LanguageServerId, Result, Worktree, serde_json};

struct TestaustimeExtension {
    cached_binary_path: Option<PathBuf>,
}

fn executable_name(binary: &str) -> String {
    match zed::current_platform() {
        (zed::Os::Windows, _) => format!("{binary}.exe"),
        _ => binary.to_string(),
    }
}

impl TestaustimeExtension {
    fn target_triple(&self) -> Result<String> {
        let (platform, arch) = zed::current_platform();

        let arch = match arch {
            zed::Architecture::Aarch64 => "aarch64",
            zed::Architecture::X8664 => "x86_64",
            _ => return Err(format!("unsupported architecture: {arch:?}")),
        };

        let os = match platform {
            zed::Os::Mac => "apple-darwin",
            zed::Os::Linux => "unknown-linux-gnu",
            zed::Os::Windows => "pc-windows-msvc",
        };

        Ok(format!("testaustime-ls-{arch}-{os}"))
    }

    fn download(&self, language_server_id: &LanguageServerId) -> Result<PathBuf> {
        let release = zed::latest_github_release(
            "testaustime/testaustime-zed",
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        let target_triple = self.target_triple()?;
        let asset_name = format!("{target_triple}.zip");

        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .ok_or_else(|| format!("no asset found matching {asset_name:?}"))?;

        let version_dir = format!("testaustime-ls-{}", release.version);
        let binary_path = Path::new(&version_dir).join(executable_name("testaustime-ls"));

        if !fs::metadata(&binary_path).is_ok_and(|stat| stat.is_file()) {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            zed::download_file(
                &asset.download_url,
                &version_dir,
                zed::DownloadedFileType::Zip,
            )
            .map_err(|err| format!("failed to download file: {err}"))?;

            // remove old versions
            let entries = fs::read_dir(".")
                .map_err(|err| format!("failed to list working directory {err}"))?;

            for entry in entries {
                let entry = entry.map_err(|err| format!("failed to load directory entry {err}"))?;
                if let Some(file_name) = entry.file_name().to_str()
                    && file_name.starts_with("testaustime-ls")
                    && file_name != version_dir
                {
                    fs::remove_dir_all(entry.path()).ok();
                }
            }
        }

        zed::make_file_executable(binary_path.to_str().unwrap())?;

        Ok(binary_path)
    }

    fn language_server_binary_path(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<PathBuf> {
        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        // check $PATH
        if let Some(path) = worktree.which(&executable_name("testaustime-ls")) {
            return Ok(path.into());
        }

        // check cache
        if let Some(path) = &self.cached_binary_path
            && fs::metadata(path).is_ok_and(|stat| stat.is_file())
        {
            return Ok(path.clone());
        }

        // download
        let binary_path = self.download(language_server_id)?;
        self.cached_binary_path = Some(binary_path.clone());

        Ok(binary_path)
    }
}

impl zed::Extension for TestaustimeExtension {
    fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Command> {
        let binary_path = self.language_server_binary_path(language_server_id, worktree)?;

        Ok(Command {
            command: binary_path.to_str().unwrap().to_owned(),
            args: vec![],
            env: worktree.shell_env(),
        })
    }

    fn language_server_initialization_options(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Option<serde_json::Value>> {
        let settings_json = zed::settings::LspSettings::for_worktree("testaustime", worktree)
            .ok()
            .and_then(|s| s.settings)
            .unwrap_or_default();

        let settings = TestaustimeSettings::from_json(&settings_json);
        Ok(Some(settings.to_init_options()))
    }
}

zed::register_extension!(TestaustimeExtension);
