use once_cell::sync::Lazy;
use std::fmt;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicBool, Ordering};
use termcolor::Ansi;
use termcolor::Color::Ansi256;
use termcolor::Color::Blue;
use termcolor::Color::Cyan;
use termcolor::Color::Green;
use termcolor::Color::Red;
use termcolor::Color::Rgb;
use termcolor::Color::White;
use termcolor::Color::Yellow;
use termcolor::ColorSpec;
use termcolor::WriteColor;

#[cfg(windows)]
use termcolor::BufferWriter;
#[cfg(windows)]
use termcolor::ColorChoice;

static FORCE_COLOR: Lazy<bool> = Lazy::new(|| {
    std::env::var_os("FORCE_COLOR")
        .map(|v| !v.is_empty())
        .unwrap_or(false)
});

static USE_COLOR: Lazy<AtomicBool> = Lazy::new(|| {
    if *FORCE_COLOR {
        return AtomicBool::new(true);
    }

    let no_color = std::env::var_os("NO_COLOR")
        .map(|v| !v.is_empty())
        .unwrap_or(false);
    AtomicBool::new(!no_color)
});

pub fn use_color() -> bool {
    USE_COLOR.load(Ordering::Relaxed)
}

pub fn set_use_color(use_color: bool) {
    USE_COLOR.store(use_color, Ordering::Relaxed);
}

pub fn enable_ansi() {
    #[cfg(windows)]
    {
        BufferWriter::stdout(ColorChoice::AlwaysAnsi);
    }
}

struct StdFmtStdIoWriter<'a>(&'a mut dyn fmt::Write);

impl std::io::Write for StdFmtStdIoWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let s = std::str::from_utf8(buf).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "failed to convert bytes to utf-8",
            )
        })?;

        self.0
            .write_str(s)
            .map_err(|_| std::io::Error::other("failed to write formatted output"))?;

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct StdIoStdFmtWriter<'a>(&'a mut dyn std::io::Write);

impl fmt::Write for StdIoStdFmtWriter<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.write_all(s.as_bytes()).map_err(|_| fmt::Error)?;
        Ok(())
    }
}

pub struct Style<I: fmt::Display> {
    colorspec: ColorSpec,
    inner: I,
}

impl<I: fmt::Display> fmt::Display for Style<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !use_color() {
            return fmt::Display::fmt(&self.inner, f);
        }

        let mut ansi_writer = Ansi::new(StdFmtStdIoWriter(f));
        ansi_writer
            .set_color(&self.colorspec)
            .map_err(|_| fmt::Error)?;
        write!(StdIoStdFmtWriter(&mut ansi_writer), "{}", self.inner)?;
        ansi_writer.reset().map_err(|_| fmt::Error)?;
        Ok(())
    }
}

fn style<S: fmt::Display>(s: S, colorspec: ColorSpec) -> Style<S> {
    Style {
        colorspec,
        inner: s,
    }
}

pub fn red_bold<S: fmt::Display>(s: S) -> Style<S> {
    let mut style_spec = ColorSpec::new();
    style_spec.set_fg(Some(Red)).set_bold(true);
    style(s, style_spec)
}

pub fn green_bold<S: fmt::Display>(s: S) -> Style<S> {
    let mut style_spec = ColorSpec::new();
    style_spec.set_fg(Some(Green)).set_bold(true);
    style(s, style_spec)
}

pub fn yellow_bold<S: fmt::Display>(s: S) -> Style<S> {
    let mut style_spec = ColorSpec::new();
    style_spec.set_fg(Some(Yellow)).set_bold(true);
    style(s, style_spec)
}

pub fn cyan_bold<S: fmt::Display>(s: S) -> Style<S> {
    let mut style_spec = ColorSpec::new();
    style_spec.set_fg(Some(Cyan)).set_bold(true);
    style(s, style_spec)
}

pub fn yellow<S: fmt::Display>(s: S) -> Style<S> {
    let mut style_spec = ColorSpec::new();
    style_spec.set_fg(Some(Yellow));
    style(s, style_spec)
}

pub fn purple_bold<S: fmt::Display>(s: S) -> Style<S> {
    let mut style_spec = ColorSpec::new();
    style_spec.set_fg(Some(Ansi256(141))).set_bold(true);
    style(s, style_spec)
}

pub fn teal_bold<S: fmt::Display>(s: S) -> Style<S> {
    let mut style_spec = ColorSpec::new();
    style_spec.set_fg(Some(Rgb(78, 201, 176))).set_bold(true);
    style(s, style_spec)
}

pub fn brand<S: fmt::Display>(s: S) -> Style<S> {
    let mut style_spec = ColorSpec::new();
    style_spec
        .set_fg(Some(Blue))
        .set_intense(true)
        .set_bold(true);
    style(s, style_spec)
}

pub fn gray<S: fmt::Display>(s: S) -> Style<S> {
    let mut style_spec = ColorSpec::new();
    style_spec.set_fg(Some(Ansi256(245)));
    style(s, style_spec)
}

pub fn bold<S: fmt::Display>(s: S) -> Style<S> {
    let mut style_spec = ColorSpec::new();
    style_spec.set_bold(true);
    style(s, style_spec)
}

pub fn dimmed_gray<S: fmt::Display>(s: S) -> Style<S> {
    let mut style_spec = ColorSpec::new();
    style_spec.set_fg(Some(Ansi256(245))).set_dimmed(true);
    style(s, style_spec)
}

pub fn white_bold_on_red<S: fmt::Display>(s: S) -> Style<S> {
    let mut style_spec = ColorSpec::new();
    style_spec
        .set_fg(Some(White))
        .set_bg(Some(Red))
        .set_bold(true);
    style(s, style_spec)
}
