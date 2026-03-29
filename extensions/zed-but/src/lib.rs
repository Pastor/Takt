use zed_extension_api::{self as zed, LanguageServerId, Result};

struct BuTExtension;

impl zed::Extension for BuTExtension {
    fn new() -> Self {
        BuTExtension
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        _worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        Ok(zed::Command {
            command: "/Users/pastor/.cargo/bin/but-lsp".to_string(),
            args: vec![],
            env: vec![],
        })
    }
}

zed::register_extension!(BuTExtension);
