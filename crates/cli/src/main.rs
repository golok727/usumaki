pub mod cli;
pub mod init;
pub mod standalone;
pub mod utils;

use anyhow::{Context, Result};
use uzumaki_runtime::{AppConfig, Application};

use crate::standalone::LaunchMode;

pub static UZUMAKI_SNAPSHOT: Option<&[u8]> = Some(include_bytes!(concat!(
    env!("OUT_DIR"),
    "/UZUMAKI_SNAPSHOT.bin"
)));

fn main() {
    uzumaki_runtime::terminal_colors::enable_ansi();

    #[cfg(target_os = "windows")]
    unsafe {
        std::env::set_var("WGPU_POWER_PREF", "high");
    }

    if let Err(err) = run() {
        utils::print_error(&err);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    uzumaki_runtime::rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .map_err(|_| anyhow::anyhow!("failed to install rustls crypto provider"))?;

    if let Some(config) =
        standalone::detect_and_prepare().context("failed to read embedded standalone payload")?
    {
        return run_app(config);
    }

    match cli::run_cli()? {
        Some(LaunchMode::Dev { config }) | Some(LaunchMode::Standalone { config, .. }) => {
            run_app(config)
        }
        Some(LaunchMode::Headless { config }) => run_headless(config),
        None => Ok(()),
    }
}

fn run_headless(config: AppConfig) -> Result<()> {
    uzumaki_runtime::headless::run_headless(UZUMAKI_SNAPSHOT, config)
}

fn run_app(config: AppConfig) -> Result<()> {
    let mut app = Application::new_with_root(UZUMAKI_SNAPSHOT, config)
        .context("failed to create application")?;

    app.run()
}
