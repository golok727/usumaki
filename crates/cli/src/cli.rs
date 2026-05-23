use anyhow::{Context, Result, bail};
use clap::builder::styling::AnsiColor;
use clap::{ColorChoice, Command, CommandFactory, FromArgMatches, Parser, Subcommand};
use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

use crate::standalone;
use crate::utils;
use uzumaki_runtime::AppConfig;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const GITHUB_REPO: &str = "golok727/uzumaki";
const CONFIG_FILE_NAME: &str = "uzumaki.config.json";
#[derive(Debug, serde::Deserialize)]
pub struct UzumakiConfig {
    #[serde(rename = "productName")]
    pub product_name: String,
    pub version: String,
    pub identifier: String,
    #[serde(default, rename = "jsxImportSource")]
    pub jsx_import_source: Option<String>,
    #[serde(default, rename = "beforeBuildCommand")]
    pub before_build_command: Option<String>,
    #[serde(default)]
    pub bundle: BundleConfig,
}

#[derive(Debug, Default, serde::Deserialize)]
pub struct BundleConfig {
    /// Files / globs to copy next to the packed exe under `resources/`.
    /// Resolved relative to the config file's directory.
    #[serde(default)]
    pub resources: Vec<String>,
    #[serde(default)]
    pub js: BundleJsConfig,
    #[serde(rename = "outputDir")]
    pub output_dir: Option<String>,
    #[serde(rename = "binName")]
    pub bin_name: Option<String>,
    #[serde(rename = "baseBinary")]
    pub base_binary: Option<String>,
}

#[derive(Debug, Default, serde::Deserialize)]
pub struct BundleJsConfig {
    #[serde(rename = "rootDir")]
    pub root_dir: Option<String>,
    pub entry: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_parse_should_accept_bundle_without_before_build_command() {
        let config: UzumakiConfig = serde_json::from_str(
            r#"{
                "productName": "demo",
                "version": "0.1.0",
                "identifier": "com.example.demo",
                "bundle": {
                    "js": {
                        "rootDir": "./dist",
                        "entry": "index.js"
                    },
                    "outputDir": "./target"
                }
            }"#,
        )
        .expect("config should parse");

        assert!(config.before_build_command.is_none());
        assert_eq!(config.bundle.js.root_dir.as_deref(), Some("./dist"));
        assert_eq!(config.bundle.js.entry.as_deref(), Some("index.js"));
        assert_eq!(config.bundle.output_dir.as_deref(), Some("./target"));
    }
}

fn find_config(start: &Path) -> Option<PathBuf> {
    let mut dir = start.to_path_buf();
    loop {
        let candidate = dir.join(CONFIG_FILE_NAME);
        if candidate.is_file() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn load_config(path: &Path) -> Result<UzumakiConfig> {
    let raw = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
}

#[derive(Parser)]
#[command(
    name = "uzumaki",
    about = "Desktop UI runtime",
    version = VERSION,
)]
pub struct Cli {
    /// Entry point file to run in dev mode
    pub entry: Option<String>,
    /// Extra arguments passed to the runtime when using the bare entry form
    #[arg(
        trailing_var_arg = true,
        allow_hyphen_values = true,
        requires = "entry"
    )]
    pub args: Vec<String>,
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run a JS/TS file in the uzumaki runtime
    Dev {
        /// Entry point file
        entry: String,
        /// Extra arguments passed to the runtime
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    // Run as JS/TS file in headless mode
    Run {
        entry: String,
        /// Extra arguments passed to the runtime
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Build and package an app using uzumaki.config.json
    Build {
        /// Path to config file
        #[arg(long)]
        config: Option<String>,
        /// Skip the build step
        #[arg(long)]
        no_build: bool,
    },
    /// Initialize a new Uzumaki project
    Init {
        /// Initialize the current directory as a new Uzumaki project.
        #[arg(hide = true)]
        name: Option<String>,
    },
    /// Create a new Uzumaki project in a new directory
    Create {
        /// Project name / directory to create
        name: Option<String>,
    },
    /// Upgrade to the latest version
    Upgrade {
        /// Specific version to install (e.g. 0.2.0)
        #[arg(long)]
        version: Option<String>,
    },
}

fn clap_styles() -> clap::builder::Styles {
    clap::builder::Styles::styled()
        .header(AnsiColor::BrightCyan.on_default().bold())
        .usage(AnsiColor::White.on_default().bold())
        .literal(AnsiColor::Green.on_default())
        .placeholder(AnsiColor::Green.on_default())
}

fn long_version() -> String {
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "release".to_string());
    let target = std::env::var("TARGET")
        .unwrap_or_else(|_| format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS));

    format!(
        "{} ({}, {})\nv8 {}\ntypescript {}",
        VERSION,
        profile,
        target,
        uzumaki_runtime::deno_core::v8::VERSION_STRING,
        uzumaki_runtime::TS_VERSION,
    )
}

pub fn clap_root() -> Command {
    let long_version: &'static str = Box::leak(long_version().into_boxed_str());

    Cli::command()
        .bin_name("uzumaki")
        .styles(clap_styles())
        .color(ColorChoice::Auto)
        .term_width(100)
        .next_line_help(false)
        .disable_help_subcommand(true)
        .long_version(long_version)
}

pub fn run_cli() -> Result<Option<standalone::LaunchMode>> {
    if should_print_root_help() {
        print_root_help();
        println!();
        return Ok(None);
    }

    let cmd = clap_root();
    let matches = if is_root_invocation(&cmd) {
        cmd.get_matches()
    } else {
        let raw_args: Vec<String> = std::env::args().collect();
        let mut patched = vec![raw_args[0].clone(), "dev".to_string()];
        patched.extend_from_slice(&raw_args[1..]);
        cmd.get_matches_from(patched)
    };
    let cli = Cli::from_arg_matches(&matches)?;

    match (cli.command, cli.entry) {
        (Some(Commands::Dev { entry, args }), None) => Ok(Some(resolve_run(&entry, args, false)?)),
        (Some(Commands::Run { entry, args }), None) => Ok(Some(resolve_run(&entry, args, true)?)),
        (Some(Commands::Build { config, no_build }), None) => {
            cmd_build(config.as_deref(), no_build)?;
            Ok(None)
        }
        (Some(Commands::Init { name }), None) => {
            crate::init::cmd_init(name.as_deref())?;
            Ok(None)
        }
        (Some(Commands::Create { name }), None) => {
            match name {
                Some(name) => crate::init::cmd_create(&name)?,
                None => crate::init::cmd_create_interactive()?,
            }
            Ok(None)
        }
        (Some(Commands::Upgrade { version }), None) => {
            cmd_upgrade(version.as_deref())?;
            Ok(None)
        }
        (None, Some(entry)) => Ok(Some(resolve_run(&entry, cli.args, false)?)),
        (None, None) => {
            print_root_help();
            println!();
            Ok(None)
        }
        (Some(_), Some(_)) => {
            unreachable!("clap should not parse a subcommand and bare entry together")
        }
    }
}

fn is_root_invocation(cmd: &Command) -> bool {
    let Some(arg1) = std::env::args().nth(1) else {
        return true;
    };
    if arg1.starts_with('-') {
        return true;
    }
    cmd.get_subcommands()
        .any(|sc| sc.get_name() == arg1 || sc.get_all_aliases().any(|a| a == arg1))
}

fn should_print_root_help() -> bool {
    let args: Vec<String> = std::env::args().skip(1).collect();
    args.is_empty()
        || matches!(args.as_slice(), [flag] if flag == "--help" || flag == "-h" || flag == "help")
}

fn print_root_help() {
    println!(
        "{} is a desktop UI runtime for Javascript / TypeScript. (v{})",
        utils::brand("Uzumaki"),
        VERSION
    );
    println!();
    println!("Usage: uzumaki [ENTRY] [ARGS]... [COMMAND]");
    println!();
    println!("{}", utils::bold("Commands:"));

    utils::print_help_command(
        utils::purple("dev"),
        Some("./app.tsx"),
        "Run a file in the interactive runtime",
    );
    utils::print_help_command(
        utils::purple("run"),
        Some("./script.ts"),
        "Run a file in headless mode",
    );

    println!();

    utils::print_help_command(
        utils::yellow("build"),
        Some(""),
        "Build and package an app using uzumaki.config.json",
    );

    println!();

    utils::print_help_command(
        utils::teal("init"),
        Some(""),
        "Initialize a project in the current directory",
    );
    utils::print_help_command(
        utils::teal("create"),
        Some("[name]"),
        "Create a new project",
    );

    println!();

    utils::print_help_command(
        utils::cyan("upgrade"),
        Some(""),
        "Upgrade to the latest version",
    );

    println!();
    println!("{}", utils::bold("Options:"));
    println!("  {:<10} Print help text", utils::muted("-h, --help"));
    println!("  {:<10} Print version", utils::muted("-V, --version"));
    println!();
    println!("  uzumaki --help           Print help text for the root command.");
    println!(
        "  uzumaki {} --help Print help text for a command.",
        utils::muted("<command>")
    );
    println!();
    println!(
        "{} https://github.com/golok727/uzumaki",
        utils::muted("GitHub:")
    );
}

fn resolve_run(entry: &str, args: Vec<String>, headless: bool) -> Result<standalone::LaunchMode> {
    let cwd = std::env::current_dir()?;
    let entry_path = strip_unc_prefix(
        fs::canonicalize(cwd.join(entry))
            .with_context(|| format!("entry point not found: {entry}"))?,
    );
    let app_root = entry_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or(cwd.clone());

    // Locate the config to derive identifier and the resource root for dev
    // mode. In dev, resources are read straight from the project tree, so the
    // resource root is the config file's directory (or app_root as a fallback).
    let (identifier, resource_root, jsx_import_source) = match find_config(&app_root) {
        Some(config_path) => {
            let config_dir = config_path
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| app_root.clone());
            let (identifier, jsx_import_source) = load_config(&config_path)
                .map(|c| (c.identifier, c.jsx_import_source))
                .unwrap_or_else(|_| ("com.uzumaki.app".to_string(), None));
            (identifier, config_dir, jsx_import_source)
        }
        None => ("com.uzumaki.app".to_string(), app_root.clone(), None),
    };

    let config = AppConfig {
        entry: entry_path,
        app_root,
        args,
        identifier,
        resource_root,
        jsx_import_source,
    };

    Ok(if headless {
        standalone::LaunchMode::Headless { config }
    } else {
        standalone::LaunchMode::Dev { config }
    })
}

/// On Windows, `fs::canonicalize` returns a `\\?\C:\...` extended-length path.
/// Strip the prefix when it's safe (regular drive paths) so user-visible strings
/// and `Uz.path.resource(...)` outputs look like normal `C:\...` paths.
fn strip_unc_prefix(path: PathBuf) -> PathBuf {
    if cfg!(windows) {
        let s = path.to_string_lossy();
        if let Some(rest) = s.strip_prefix(r"\\?\")
            && !rest.starts_with(r"UNC\")
        {
            return PathBuf::from(rest);
        }
    }
    path
}

fn cmd_build(config_path: Option<&str>, no_build: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;

    let config_file = match config_path {
        Some(p) => {
            let p = cwd.join(p);
            if !p.is_file() {
                bail!("config file not found: {}", p.display());
            }
            p
        }
        None => find_config(&cwd).ok_or_else(|| {
            anyhow::anyhow!("could not find {CONFIG_FILE_NAME} from {}", cwd.display())
        })?,
    };

    let config_dir = config_file.parent().unwrap().to_path_buf();
    let config = load_config(&config_file)?;

    if !no_build && let Some(ref cmd) = config.before_build_command {
        utils::print_status("build", cmd);
        let status = run_shell_command(cmd, &config_dir)?;
        if !status.success() {
            bail!("build command failed with exit code {}", status);
        }
    }

    // Package
    let js_dist = config
        .bundle
        .js
        .root_dir
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("missing bundle.js.rootDir in config"))?;
    let entry = config
        .bundle
        .js
        .entry
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("missing bundle.js.entry in config"))?;
    let output_dir_raw = config
        .bundle
        .output_dir
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("missing bundle.outputDir in config"))?;

    let js_dist_path = resolve_from(&config_dir, js_dist);
    let output_dir = resolve_from(&config_dir, output_dir_raw);
    let bin_name = config
        .bundle
        .bin_name
        .clone()
        .unwrap_or_else(|| config.product_name.clone());
    let output_path = normalize_output_extension(&output_dir.join(&bin_name));
    let base_binary = match &config.bundle.base_binary {
        Some(b) => resolve_from(&config_dir, b),
        None => std::env::current_exe()?,
    };

    utils::print_status("package", format!("{js_dist} -> {}", output_path.display()));

    let final_output = standalone::pack::pack_app(&standalone::pack::PackOptions {
        dist_dir: js_dist_path,
        entry_rel: entry.to_string(),
        output: output_path.clone(),
        app_name: bin_name,
        base_binary,
        identifier: config.identifier,
        version: config.version,
        product_name: config.product_name,
    })?;

    if !config.bundle.resources.is_empty() {
        let resources_dir = resources_dir_for(&final_output)?;
        copy_bundle_resources(&config_dir, &config.bundle.resources, &resources_dir)?;
    }

    Ok(())
}

/// Where bundle resources land relative to the packed artifact.
/// - macOS: `<App>.app/Contents/Resources/` (already created by the bundler).
/// - Windows / Linux: `<exe_dir>/resources/` (created on demand).
fn resources_dir_for(final_output: &Path) -> Result<PathBuf> {
    if cfg!(target_os = "macos") {
        Ok(final_output.join("Contents").join("Resources"))
    } else {
        final_output
            .parent()
            .map(|p| p.join("resources"))
            .ok_or_else(|| anyhow::anyhow!("output path has no parent"))
    }
}

fn copy_bundle_resources(base: &Path, patterns: &[String], resources_dir: &Path) -> Result<()> {
    fs::create_dir_all(resources_dir)
        .with_context(|| format!("creating {}", resources_dir.display()))?;

    for pattern in patterns {
        let mut matched = false;
        let abs_pattern = if Path::new(pattern).is_absolute() {
            pattern.clone()
        } else {
            base.join(pattern).to_string_lossy().into_owned()
        };

        for entry in
            glob::glob(&abs_pattern).with_context(|| format!("invalid glob pattern: {pattern}"))?
        {
            let path = entry.with_context(|| format!("globbing {pattern}"))?;
            matched = true;
            let rel = path.strip_prefix(base).unwrap_or(&path);
            let dest = resources_dir.join(rel);
            copy_path(&path, &dest)?;
        }

        if !matched {
            utils::print_warning("bundle resource", format!("{pattern} matched nothing"));
        }
    }
    Ok(())
}

fn copy_path(src: &Path, dest: &Path) -> Result<()> {
    let meta = fs::metadata(src).with_context(|| format!("stat {}", src.display()))?;
    if meta.is_dir() {
        fs::create_dir_all(dest).with_context(|| format!("creating {}", dest.display()))?;
        for entry in fs::read_dir(src).with_context(|| format!("reading {}", src.display()))? {
            let entry = entry?;
            let name = entry.file_name();
            copy_path(&entry.path(), &dest.join(&name))?;
        }
    } else {
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
        fs::copy(src, dest)
            .with_context(|| format!("copying {} -> {}", src.display(), dest.display()))?;
    }
    Ok(())
}

fn cmd_upgrade(target_version: Option<&str>) -> Result<()> {
    utils::print_status("upgrade", "checking for updates...");

    let version_tag = match target_version {
        Some(v) => {
            if v.starts_with('v') {
                v.to_string()
            } else {
                format!("v{v}")
            }
        }
        None => {
            // Fetch latest release tag from GitHub API
            let url = format!("https://api.github.com/repos/{GITHUB_REPO}/releases/latest");
            let body: String = ureq::get(&url)
                .header("Accept", "application/vnd.github+json")
                .header("User-Agent", "uzumaki-updater")
                .call()
                .context("failed to fetch latest release")?
                .body_mut()
                .read_to_string()
                .context("failed to read response body")?;
            let release: serde_json::Value =
                serde_json::from_str(&body).context("invalid JSON from GitHub API")?;
            release["tag_name"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("no tag_name in latest release"))?
                .to_string()
        }
    };

    let version_num = version_tag.strip_prefix('v').unwrap_or(&version_tag);

    if version_num == VERSION {
        utils::print_status(
            "upgrade",
            format!("{} (v{VERSION})", utils::success("already up to date")),
        );
        return Ok(());
    }

    let asset_name = get_asset_name();
    let download_url =
        format!("https://github.com/{GITHUB_REPO}/releases/download/{version_tag}/{asset_name}");

    utils::print_status(
        "upgrade",
        format!("downloading v{VERSION} -> v{version_num}"),
    );

    let mut response = ureq::get(&download_url)
        .header("User-Agent", "uzumaki-updater")
        .call()
        .with_context(|| format!("failed to download {download_url}"))?;

    let total = response
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);

    let mut body_bytes = Vec::with_capacity(total as usize);
    let mut reader = response.body_mut().as_reader();
    let mut downloaded: u64 = 0;
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = reader
            .read(&mut buf)
            .context("failed to read download body")?;
        if n == 0 {
            break;
        }
        body_bytes.extend_from_slice(&buf[..n]);
        downloaded += n as u64;
        if total > 0 {
            let pct = (downloaded as f64 / total as f64 * 100.0) as u8;
            let filled = (pct as usize) / 2;
            eprint!(
                "\r{} {} [{:█<filled$}{:·<empty$}] {pct}%",
                utils::brand("uzumaki"),
                utils::muted("upgrade"),
                "",
                "",
                filled = filled,
                empty = 50 - filled,
                pct = pct,
            );
        }
    }
    if total > 0 {
        eprintln!();
    }

    let binary_bytes = extract_binary_from_zip(&body_bytes, &get_binary_name())?;
    let current_exe = std::env::current_exe()?;
    replace_exe(&current_exe, &binary_bytes)?;

    utils::print_status(
        "upgrade",
        format!("{} v{version_num}", utils::success("updated to")),
    );

    Ok(())
}

fn extract_binary_from_zip(zip_bytes: &[u8], binary_name: &str) -> Result<Vec<u8>> {
    let reader = std::io::Cursor::new(zip_bytes);
    let mut archive = zip::ZipArchive::new(reader).context("invalid zip archive")?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();
        if name == binary_name || name.ends_with(&format!("/{binary_name}")) {
            let mut bytes = Vec::with_capacity(file.size() as usize);
            std::io::Read::read_to_end(&mut file, &mut bytes)?;
            return Ok(bytes);
        }
    }

    bail!("binary '{binary_name}' not found in zip archive")
}

fn replace_exe(current_exe: &Path, new_bytes: &[u8]) -> Result<()> {
    let dir = current_exe.parent().unwrap();
    let tmp_file = tempfile::NamedTempFile::new_in(dir)?;
    fs::write(tmp_file.path(), new_bytes)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(tmp_file.path(), fs::Permissions::from_mode(0o755))?;
    }

    let backup_path = current_exe.with_extension("old");
    let _ = fs::remove_file(&backup_path);

    fs::rename(current_exe, &backup_path)
        .with_context(|| format!("failed to move current exe to {}", backup_path.display()))?;

    if let Err(e) = fs::rename(tmp_file.path(), current_exe) {
        // Rollback
        let _ = fs::rename(&backup_path, current_exe);
        return Err(e).context("failed to place new binary");
    }

    tmp_file.into_temp_path().keep()?;

    // Clean up backup
    let _ = fs::remove_file(&backup_path);

    Ok(())
}

fn resolve_from(base: &Path, value: &str) -> PathBuf {
    let p = Path::new(value);
    let joined = if p.is_absolute() {
        p.to_path_buf()
    } else {
        base.join(p)
    };
    normalize_path(&joined)
}

fn normalize_path(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

fn normalize_output_extension(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if cfg!(target_os = "windows") {
        if s.ends_with(".exe") {
            path.to_path_buf()
        } else {
            PathBuf::from(format!("{s}.exe"))
        }
    } else if cfg!(target_os = "macos") {
        let cleaned = s.trim_end_matches(".exe").trim_end_matches(".app");
        PathBuf::from(cleaned.to_string())
    } else {
        // Linux – strip .exe if present
        let cleaned = s.trim_end_matches(".exe");
        PathBuf::from(cleaned.to_string())
    }
}

fn run_shell_command(command: &str, cwd: &Path) -> Result<std::process::ExitStatus> {
    let status = if cfg!(target_os = "windows") {
        ProcessCommand::new("cmd.exe")
            .args(["/d", "/s", "/c", command])
            .current_dir(cwd)
            .status()
    } else {
        ProcessCommand::new("sh")
            .args(["-lc", command])
            .current_dir(cwd)
            .status()
    };
    status.with_context(|| format!("failed to run: {command}"))
}

fn get_binary_name() -> String {
    if cfg!(target_os = "windows") {
        "uzumaki.exe".to_string()
    } else {
        "uzumaki".to_string()
    }
}

fn get_asset_name() -> String {
    let os = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x64"
    };

    format!("uzumaki-{os}-{arch}.zip")
}
