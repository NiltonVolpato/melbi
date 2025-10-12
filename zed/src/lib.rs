use zed_extension_api::{self as zed, Result};

struct RhizomeExtension {
    cached_binary_path: Option<String>,
}

impl zed::Extension for RhizomeExtension {
    fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        // Option 1: Use bundled binary
        let command = zed::Command {
            command: self.language_server_binary_path(language_server_id, worktree)?,
            args: vec![],
            env: Default::default(),
        };
        Ok(command)
    }

    fn language_server_initialization_options(
        &mut self,
        _language_server_id: &zed::LanguageServerId,
        _worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        Ok(None)
    }

    fn language_server_workspace_configuration(
        &mut self,
        _language_server_id: &zed::LanguageServerId,
        _worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        Ok(None)
    }
}

impl RhizomeExtension {
    fn language_server_binary_path(
        &mut self,
        _language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<String> {
        if let Some(path) = &self.cached_binary_path {
            if std::fs::metadata(path).map_or(false, |stat| stat.is_file()) {
                return Ok(path.clone());
            }
        }

        // Option 1: Look for locally built binary (development)
        if let Some(path) = worktree.which("rhizome-lsp") {
            self.cached_binary_path = Some(path.clone());
            return Ok(path);
        }

        // TODO: Option 2: Download from GitHub releases
        Err("rhizome-lsp binary not found".into())
    }
}

zed::register_extension!(RhizomeExtension);
