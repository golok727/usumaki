use anyhow::{Result, bail};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::utils;

const TMPL_PACKAGE_JSON: &str = include_str!("../template/package.json");
const TMPL_TSCONFIG: &str = include_str!("../template/tsconfig.json");
const TMPL_CONFIG: &str = include_str!("../template/uzumaki.config.json");
const TMPL_INDEX_TSX: &str = include_str!("../template/src/index.tsx");
const TMPL_LOGO_SVG: &[u8] = include_bytes!("../template/assets/logo.svg");
const TMPL_REACT_SVG: &[u8] = include_bytes!("../template/assets/react.svg");
const TMPL_BUILD_TS: &str = include_str!("../template/build.ts");
const TMPL_APP_TSX: &str = include_str!("../template/src/app.tsx");
const TMPL_THEME_TSX: &str = include_str!("../template/src/theme.ts");

struct TemplateEntry {
    path: &'static str,
    content: TemplateContent,
}

enum TemplateContent {
    Text(&'static str),
    Bytes(&'static [u8]),
}

const TEMPLATE_ENTRIES: &[TemplateEntry] = &[
    TemplateEntry {
        path: "package.json",
        content: TemplateContent::Text(TMPL_PACKAGE_JSON),
    },
    TemplateEntry {
        path: "tsconfig.json",
        content: TemplateContent::Text(TMPL_TSCONFIG),
    },
    TemplateEntry {
        path: "uzumaki.config.json",
        content: TemplateContent::Text(TMPL_CONFIG),
    },
    TemplateEntry {
        path: "build.ts",
        content: TemplateContent::Text(TMPL_BUILD_TS),
    },
    TemplateEntry {
        path: "src/index.tsx",
        content: TemplateContent::Text(TMPL_INDEX_TSX),
    },
    TemplateEntry {
        path: "src/theme.ts",
        content: TemplateContent::Text(TMPL_THEME_TSX),
    },
    TemplateEntry {
        path: "src/app.tsx",
        content: TemplateContent::Text(TMPL_APP_TSX),
    },
    TemplateEntry {
        path: "assets/logo.svg",
        content: TemplateContent::Bytes(TMPL_LOGO_SVG),
    },
    TemplateEntry {
        path: "assets/react.svg",
        content: TemplateContent::Bytes(TMPL_REACT_SVG),
    },
];

fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_lowercase()
}

fn apply_vars(template: &str, vars: &[(&str, &str)]) -> String {
    let mut out = template.to_string();
    for (key, value) in vars {
        out = out.replace(&format!("{{{{{key}}}}}"), value);
    }
    out
}

fn write_template_entry(base: &Path, entry: &TemplateEntry, vars: &[(&str, &str)]) -> Result<()> {
    let dest = base.join(entry.path);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    match entry.content {
        TemplateContent::Text(template) => fs::write(&dest, apply_vars(template, vars))?,
        TemplateContent::Bytes(bytes) => fs::write(&dest, bytes)?,
    }
    Ok(())
}

pub fn cmd_init(target_dir: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir()?;

    if cwd.join("package.json").is_file() {
        bail!(
            "a project already exists in this folder: found package.json in {}",
            cwd.display()
        );
    }

    let default_name = cwd
        .file_name()
        .map(|name| sanitize_name(&name.to_string_lossy()))
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "my-app".to_string());

    let project_name = match target_dir {
        Some(name) => {
            let name = sanitize_name(name);
            if name.is_empty() {
                bail!("project name cannot be empty");
            }
            name
        }
        None => {
            let name = prompt_with_default("Project name", &default_name)?;
            let name = sanitize_name(&name);
            if name.is_empty() {
                bail!("project name cannot be empty");
            }
            name
        }
    };

    let identifier = prompt_with_default("Bundle identifier", &default_identifier(&project_name))?;

    scaffold_project_here(&cwd, &project_name, &identifier, "init")
}

pub fn cmd_create(name: &str) -> Result<()> {
    let project_name = sanitize_name(name);
    if project_name.is_empty() {
        bail!("project name cannot be empty");
    }
    let identifier = prompt_with_default("Bundle identifier", &default_identifier(&project_name))?;
    scaffold_project(Some(name), false, &identifier, "create")
}

pub fn cmd_create_interactive() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let default_name = cwd
        .file_name()
        .map(|name| sanitize_name(&name.to_string_lossy()))
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "my-app".to_string());

    println!("{}", utils::brand("Create a new Uzumaki project"));
    println!();
    println!(
        "{}",
        utils::muted("Press enter to initialize the current directory.")
    );
    println!();

    let name = prompt_with_default("Project name", &default_name)?;
    let name = sanitize_name(&name);

    if name.is_empty() || name == default_name {
        let identifier = prompt_with_default("Bundle identifier", &default_identifier(&name))?;
        return scaffold_project_here(&cwd, &name, &identifier, "create");
    }

    let identifier = prompt_with_default("Bundle identifier", &default_identifier(&name))?;
    scaffold_project(Some(&name), false, &identifier, "create")
}

fn scaffold_project(
    target_dir: Option<&str>,
    allow_current_dir: bool,
    identifier: &str,
    action: &str,
) -> Result<()> {
    let cwd = std::env::current_dir()?;
    if !allow_current_dir && target_dir.is_none() {
        bail!("project name is required");
    }
    let (project_dir, dir_display) = resolve_project_dir(&cwd, target_dir);
    let project_name = derive_project_name(&project_dir)?;

    if project_dir.is_dir() {
        if project_dir.join("package.json").is_file() {
            if allow_current_dir && target_dir.is_none() {
                bail!(
                    "a project already exists in this folder: found package.json in {}",
                    project_dir.display()
                );
            }
            bail!(
                "target folder already contains package.json: {}",
                project_dir.display()
            );
        }

        let has_entries = fs::read_dir(&project_dir)?.next().is_some();
        if (!allow_current_dir || target_dir.is_some()) && has_entries {
            bail!("directory {} is not empty", project_dir.display());
        }
    }

    let vars: Vec<(&str, &str)> = vec![("PROJECT_NAME", &project_name), ("IDENTIFIER", identifier)];

    utils::print_status(
        action,
        format!("creating project {}", dir_display.display()),
    );

    for entry in TEMPLATE_ENTRIES {
        if allow_current_dir && target_dir.is_none() && project_dir.join(entry.path).exists() {
            bail!(
                "cannot initialize here because {} already exists",
                project_dir.join(entry.path).display()
            );
        }
        write_template_entry(&project_dir, entry, &vars)?;
    }

    let rel = project_dir.strip_prefix(&cwd).unwrap_or(&project_dir);

    for entry in TEMPLATE_ENTRIES {
        println!(
            "  {} {}/{}",
            utils::success("created"),
            rel.display(),
            entry.path
        );
    }

    println!();
    println!("{}", utils::brand("Next steps"));
    if target_dir.is_some() {
        println!("  cd {}", rel.display());
    }
    println!("  pnpm install");
    println!("  pnpm dev");
    println!();

    Ok(())
}

fn scaffold_project_here(
    project_dir: &Path,
    project_name: &str,
    identifier: &str,
    action: &str,
) -> Result<()> {
    if project_dir.join("package.json").is_file() {
        bail!(
            "a project already exists in this folder: found package.json in {}",
            project_dir.display()
        );
    }

    utils::print_status(
        action,
        format!("creating project {}", project_dir.display()),
    );

    let vars: Vec<(&str, &str)> = vec![("PROJECT_NAME", project_name), ("IDENTIFIER", identifier)];

    for entry in TEMPLATE_ENTRIES {
        let dest = project_dir.join(entry.path);
        if dest.exists() {
            bail!(
                "cannot initialize here because {} already exists",
                dest.display()
            );
        }
        write_template_entry(project_dir, entry, &vars)?;
    }

    for entry in TEMPLATE_ENTRIES {
        println!(
            "  {} ./{}",
            utils::success("created"),
            entry.path.replace('\\', "/")
        );
    }

    println!();
    println!("{}", utils::brand("Next steps"));
    println!("  pnpm install");
    println!("  pnpm dev");
    println!();

    Ok(())
}

fn derive_project_name(project_dir: &Path) -> Result<String> {
    let raw_name = project_dir
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "my-app".to_string());
    let project_name = sanitize_name(&raw_name);

    if project_name.is_empty() {
        bail!("could not derive a valid project name from '{}'", raw_name);
    }

    Ok(project_name)
}

fn default_identifier(project_name: &str) -> String {
    format!("com.example.{}", project_name.replace('-', "_"))
}

fn resolve_project_dir(cwd: &Path, target_dir: Option<&str>) -> (PathBuf, PathBuf) {
    match target_dir {
        Some(target) => {
            let path = cwd.join(target);
            let display = path.strip_prefix(cwd).unwrap_or(&path).to_path_buf();
            (path, display)
        }
        None => (cwd.to_path_buf(), PathBuf::from(".")),
    }
}

fn prompt_with_default(label: &str, default: &str) -> Result<String> {
    print!(
        "{} {} {} ",
        utils::teal("?"),
        label,
        utils::muted(format!("({default})"))
    );
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let trimmed = input.trim();

    if trimmed.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(trimmed.to_string())
    }
}
