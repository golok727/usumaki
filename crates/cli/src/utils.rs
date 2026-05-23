use anyhow::Error;
use uzumaki_runtime::terminal_colors;

pub fn brand(text: impl AsRef<str>) -> String {
    format!("{}", terminal_colors::brand(text.as_ref()))
}

pub fn muted(text: impl AsRef<str>) -> String {
    format!("{}", terminal_colors::dimmed_gray(text.as_ref()))
}

pub fn success(text: impl AsRef<str>) -> String {
    format!("{}", terminal_colors::green_bold(text.as_ref()))
}

pub fn bold(text: impl AsRef<str>) -> String {
    format!("{}", terminal_colors::bold(text.as_ref()))
}

pub fn cyan(text: impl AsRef<str>) -> String {
    format!("{}", terminal_colors::cyan_bold(text.as_ref()))
}

pub fn yellow(text: impl AsRef<str>) -> String {
    format!("{}", terminal_colors::yellow(text.as_ref()))
}

pub fn purple(text: impl AsRef<str>) -> String {
    format!("{}", terminal_colors::purple_bold(text.as_ref()))
}

pub fn teal(text: impl AsRef<str>) -> String {
    format!("{}", terminal_colors::teal_bold(text.as_ref()))
}

pub fn warning(text: impl AsRef<str>) -> String {
    format!("{}", terminal_colors::yellow_bold(text.as_ref()))
}

pub fn danger(text: impl AsRef<str>) -> String {
    format!("{}", terminal_colors::white_bold_on_red(text.as_ref()))
}

pub fn print_status(action: &str, message: impl AsRef<str>) {
    println!(
        "{} {} {}",
        brand("uzumaki"),
        muted(action),
        message.as_ref()
    );
}

pub fn print_warning(action: &str, message: impl AsRef<str>) {
    eprintln!(
        "{} {} {}",
        brand("uzumaki"),
        muted(action),
        warning(message)
    );
}

pub fn print_error(err: &Error) {
    eprintln!("{} {}", danger("error"), err);

    let mut chain = err.chain();
    let _ = chain.next();

    let mut has_chain = false;
    for cause in chain {
        if !has_chain {
            eprintln!("{}", muted("caused by:"));
            has_chain = true;
        }
        eprintln!("  - {cause}");
    }
}

pub fn print_help_command(
    colorized_name: impl AsRef<str>,
    sample: Option<impl AsRef<str>>,
    description: impl AsRef<str>,
) {
    match sample {
        Some(sample) => println!(
            "  {:<10} {:<22} {}",
            colorized_name.as_ref(),
            muted(sample.as_ref()),
            description.as_ref()
        ),
        None => println!(
            "  {:<10} {:<22} {}",
            colorized_name.as_ref(),
            "",
            description.as_ref()
        ),
    }
}
