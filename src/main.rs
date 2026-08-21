use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anstream::{AutoStream, ColorChoice as StreamColorChoice};
use anstyle::{AnsiColor, Style};
use clap::{ArgAction, ArgGroup, Parser, ValueEnum};
use purra::files::{
    FileError, SkipReason, TextFile, collect_directory_files, read_text, replace_in_place,
    write_atomic,
};
use purra::{Engine, Finding, Preset, PresetError, PresetLoadError};
use thiserror::Error;

const EXIT_FINDINGS: u8 = 1;
const EXIT_CONFIGURATION: u8 = 2;
const EXIT_RUNTIME: u8 = 3;

#[derive(Debug, Parser)]
#[command(
    name = "purra",
    version,
    about = "Replace configured Unicode characters in text",
    group = ArgGroup::new("preset-source")
        .required(true)
        .multiple(false)
        .args(["ai_preset", "preset", "inline_preset"])
)]
struct Cli {
    /// Use the built-in AI typography preset.
    #[arg(long, action = ArgAction::SetTrue)]
    ai_preset: bool,

    /// Load one K=V rule per line from a preset file.
    #[arg(long, value_name = "FILE")]
    preset: Option<PathBuf>,

    /// Use comma-separated K=V rules supplied on the command line.
    #[arg(
        long = "in-place-preset",
        alias = "inline-preset",
        value_name = "RULES"
    )]
    inline_preset: Option<String>,

    /// Recurse into subdirectories when the input is a directory.
    #[arg(short = 'r', long)]
    recursive: bool,

    /// Replace changed directory entries without y/n confirmation.
    #[arg(short = 'f', long)]
    force: bool,

    /// Report findings without writing transformed data.
    #[arg(long)]
    dry_run: bool,

    /// Control colors in dry-run diagnostics.
    #[arg(long, value_enum, default_value_t = ColorMode::Auto)]
    color: ColorMode,

    /// Zero paths reads stdin, one file writes stdout, two paths mean input/output,
    /// and one directory is processed in place.
    #[arg(value_name = "PATH", num_args = 0..=2)]
    paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ColorMode {
    Auto,
    Always,
    Never,
}

#[derive(Debug, Error)]
enum CliError {
    #[error("{0}")]
    Usage(String),
    #[error(transparent)]
    Preset(#[from] PresetError),
    #[error(transparent)]
    PresetLoad(#[from] PresetLoadError),
    #[error(transparent)]
    File(#[from] FileError),
    #[error(transparent)]
    Io(#[from] io::Error),
}

impl CliError {
    const fn exit_code(&self) -> u8 {
        match self {
            Self::Usage(_) | Self::Preset(_) | Self::PresetLoad(_) => EXIT_CONFIGURATION,
            Self::File(_) | Self::Io(_) => EXIT_RUNTIME,
        }
    }
}

#[derive(Debug, Default)]
struct Summary {
    findings: usize,
    affected_inputs: usize,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let color = cli.color;
    match run(cli) {
        Ok(code) => code,
        Err(error) => {
            let mut diagnostics = stderr_stream(color);
            let _ = writeln!(diagnostics, "error: {error}");
            ExitCode::from(error.exit_code())
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode, CliError> {
    let engine = load_engine(&cli)?;
    validate_path_options(&cli)?;

    match cli.paths.as_slice() {
        [] => run_stdin(&engine, &cli),
        [input] if is_directory(input)? => run_directory(&engine, input, &cli),
        [input] => run_file_to_stdout(&engine, input, &cli),
        [input, output] => run_file_to_file(&engine, input, output, &cli),
        _ => unreachable!("clap limits paths to at most two"),
    }
}

fn load_engine(cli: &Cli) -> Result<Engine, CliError> {
    let preset = if cli.ai_preset {
        Preset::ai()
    } else if let Some(path) = &cli.preset {
        Preset::load(path)?
    } else if let Some(inline) = &cli.inline_preset {
        Preset::parse_inline(inline)?
    } else {
        unreachable!("clap requires exactly one preset source")
    };
    Ok(preset.engine())
}

fn validate_path_options(cli: &Cli) -> Result<(), CliError> {
    if cli.paths.len() == 2 && cli.dry_run {
        return Err(CliError::Usage(
            "--dry-run does not accept an output path".to_owned(),
        ));
    }
    if (cli.recursive || cli.force) && cli.paths.len() != 1 {
        return Err(CliError::Usage(
            "--recursive and --force require one directory input".to_owned(),
        ));
    }
    Ok(())
}

fn run_stdin(engine: &Engine, cli: &Cli) -> Result<ExitCode, CliError> {
    if cli.recursive || cli.force {
        return Err(CliError::Usage(
            "--recursive and --force cannot be used with stdin".to_owned(),
        ));
    }

    let mut bytes = Vec::new();
    io::stdin().read_to_end(&mut bytes)?;
    if bytes.contains(&0) {
        return Ok(ExitCode::SUCCESS);
    }
    let Ok(input) = String::from_utf8(bytes) else {
        warn_skip(Path::new("<stdin>"), SkipReason::InvalidUtf8, cli.color)?;
        return Ok(ExitCode::SUCCESS);
    };

    if cli.dry_run {
        let findings = engine.find(&input);
        print_findings(Path::new("<stdin>"), &findings, cli.color)?;
        print_summary(findings.len(), usize::from(!findings.is_empty()), cli.color)?;
        return Ok(findings_exit(findings.len()));
    }

    let replacement = engine.replace(&input);
    io::stdout().write_all(replacement.text.as_bytes())?;
    Ok(ExitCode::SUCCESS)
}

fn run_file_to_stdout(engine: &Engine, input: &Path, cli: &Cli) -> Result<ExitCode, CliError> {
    if cli.recursive || cli.force {
        return Err(CliError::Usage(
            "--recursive and --force require a directory input".to_owned(),
        ));
    }

    let loaded = read_text(input)?;
    let TextFile::Text(text) = loaded else {
        if let TextFile::Skipped(reason) = loaded {
            warn_skip(input, reason, cli.color)?;
        }
        return Ok(ExitCode::SUCCESS);
    };

    if cli.dry_run {
        let findings = engine.find(&text);
        print_findings(input, &findings, cli.color)?;
        print_summary(findings.len(), usize::from(!findings.is_empty()), cli.color)?;
        return Ok(findings_exit(findings.len()));
    }

    let replacement = engine.replace(&text);
    io::stdout().write_all(replacement.text.as_bytes())?;
    Ok(ExitCode::SUCCESS)
}

fn run_file_to_file(
    engine: &Engine,
    input: &Path,
    output: &Path,
    cli: &Cli,
) -> Result<ExitCode, CliError> {
    if cli.recursive || cli.force {
        return Err(CliError::Usage(
            "--recursive and --force require a directory input".to_owned(),
        ));
    }

    let loaded = read_text(input)?;
    let TextFile::Text(text) = loaded else {
        if let TextFile::Skipped(reason) = loaded {
            warn_skip(input, reason, cli.color)?;
        }
        return Ok(ExitCode::SUCCESS);
    };
    let replacement = engine.replace(&text);

    if same_path(input, output)? {
        if replacement.changed() {
            let backup = replace_in_place(input, &replacement.text)?;
            report_update(input, &backup, cli.color)?;
        }
    } else {
        write_atomic(output, &replacement.text, Some(input))?;
    }
    Ok(ExitCode::SUCCESS)
}

fn run_directory(engine: &Engine, directory: &Path, cli: &Cli) -> Result<ExitCode, CliError> {
    let paths = collect_directory_files(directory, cli.recursive)?;
    let mut summary = Summary::default();

    for path in paths {
        let loaded = read_text(&path)?;
        let TextFile::Text(text) = loaded else {
            if let TextFile::Skipped(reason) = loaded {
                warn_skip(&path, reason, cli.color)?;
            }
            continue;
        };

        if cli.dry_run {
            let findings = engine.find(&text);
            if !findings.is_empty() {
                summary.affected_inputs += 1;
                summary.findings += findings.len();
                print_findings(&path, &findings, cli.color)?;
            }
            continue;
        }

        let replacement = engine.replace(&text);
        if !replacement.changed() || (!cli.force && !confirm(&path, cli.color)?) {
            continue;
        }
        let backup = replace_in_place(&path, &replacement.text)?;
        report_update(&path, &backup, cli.color)?;
    }

    if cli.dry_run {
        print_summary(summary.findings, summary.affected_inputs, cli.color)?;
        Ok(findings_exit(summary.findings))
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

fn is_directory(path: &Path) -> Result<bool, CliError> {
    fs::symlink_metadata(path)
        .map(|metadata| metadata.is_dir())
        .map_err(|source| {
            FileError::Io {
                operation: "inspect",
                path: path.to_path_buf(),
                source,
            }
            .into()
        })
}

fn same_path(left: &Path, right: &Path) -> Result<bool, CliError> {
    if left == right {
        return Ok(true);
    }
    if !right.exists() {
        return Ok(false);
    }
    if fs::symlink_metadata(right)?.file_type().is_symlink() {
        return Ok(false);
    }
    Ok(fs::canonicalize(left)? == fs::canonicalize(right)?)
}

fn findings_exit(count: usize) -> ExitCode {
    if count == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(EXIT_FINDINGS)
    }
}

fn print_findings(path: &Path, findings: &[Finding], color: ColorMode) -> io::Result<()> {
    let mut output = stdout_stream(color);
    let path_style = Style::new().fg_color(Some(AnsiColor::Magenta.into()));
    let position_style = Style::new().fg_color(Some(AnsiColor::Green.into()));
    let finding_style = Style::new().fg_color(Some(AnsiColor::Red.into())).bold();

    for finding in findings {
        writeln!(
            output,
            "{}{path}{}:{}{}:{}{}: {}{}{} -> {}",
            path_style.render(),
            path_style.render_reset(),
            position_style.render(),
            finding.line,
            finding.column,
            position_style.render_reset(),
            finding_style.render(),
            display_character(finding.original),
            finding_style.render_reset(),
            display_character(finding.replacement),
            path = path.display(),
        )?;
    }
    Ok(())
}

fn print_summary(findings: usize, affected_inputs: usize, color: ColorMode) -> io::Result<()> {
    let mut diagnostics = stderr_stream(color);
    writeln!(
        diagnostics,
        "{findings} finding(s) in {affected_inputs} input(s)"
    )
}

fn warn_skip(path: &Path, reason: SkipReason, color: ColorMode) -> io::Result<()> {
    if reason == SkipReason::Binary {
        return Ok(());
    }
    let reason = match reason {
        SkipReason::Binary => unreachable!(),
        SkipReason::InvalidUtf8 => "invalid UTF-8",
        SkipReason::SymbolicLink => "symbolic link",
        SkipReason::UnsupportedFileType => "unsupported file type",
    };
    let mut diagnostics = stderr_stream(color);
    writeln!(diagnostics, "warning: skipped {}: {reason}", path.display())
}

fn report_update(path: &Path, backup: &Path, color: ColorMode) -> io::Result<()> {
    let mut diagnostics = stderr_stream(color);
    writeln!(
        diagnostics,
        "updated {} (backup: {})",
        path.display(),
        backup.display()
    )
}

fn confirm(path: &Path, color: ColorMode) -> io::Result<bool> {
    let mut diagnostics = stderr_stream(color);
    loop {
        write!(diagnostics, "replace {}? [y/n] ", path.display())?;
        diagnostics.flush()?;

        let mut response = String::new();
        if io::stdin().read_line(&mut response)? == 0 {
            writeln!(diagnostics)?;
            return Ok(false);
        }
        match response.trim().to_ascii_lowercase().as_str() {
            "y" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            _ => writeln!(diagnostics, "please answer y or n")?,
        }
    }
}

fn display_character(character: char) -> String {
    match character {
        ' ' => "<space>".to_owned(),
        character if character.is_control() => character.escape_default().to_string(),
        character => character.to_string(),
    }
}

fn stream_choice(color: ColorMode, is_terminal: bool) -> StreamColorChoice {
    match color {
        ColorMode::Always => StreamColorChoice::Always,
        ColorMode::Never => StreamColorChoice::Never,
        ColorMode::Auto if is_terminal => StreamColorChoice::Always,
        ColorMode::Auto => StreamColorChoice::Never,
    }
}

fn stdout_stream(color: ColorMode) -> AutoStream<io::Stdout> {
    AutoStream::new(
        io::stdout(),
        stream_choice(color, io::stdout().is_terminal()),
    )
}

fn stderr_stream(color: ColorMode) -> AutoStream<io::Stderr> {
    AutoStream::new(
        io::stderr(),
        stream_choice(color, io::stderr().is_terminal()),
    )
}
