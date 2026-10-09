use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anstream::{AutoStream, ColorChoice as StreamColorChoice};
use anstyle::{AnsiColor, Style};
use clap::{ArgAction, ArgGroup, Parser, ValueEnum};
use purra::files::{
    DirectoryError, DirectoryOptions, FileError, SkipReason, TextFile,
    collect_directory_files_with_options, read_text, replace_in_place, write_atomic,
};
use purra::{TextEngine, TextFinding, TextPreset, TextPresetError, TextPresetLoadError};
use thiserror::Error;

const EXIT_FINDINGS: u8 = 1;
const EXIT_CONFIGURATION: u8 = 2;
const EXIT_RUNTIME: u8 = 3;

#[derive(Debug, Parser)]
#[command(
    name = "purra",
    version,
    about = "Replace configured Unicode scalar values with text",
    group = ArgGroup::new("preset-source")
        .required(true)
        .multiple(false)
        .args(["ai_preset", "ascii_preset", "preset", "inline_preset"])
)]
struct Cli {
    /// Use the built-in AI typography preset.
    #[arg(long, action = ArgAction::SetTrue)]
    ai_preset: bool,

    /// Use the AI preset plus selected ASCII text expansions.
    #[arg(long, action = ArgAction::SetTrue)]
    ascii_preset: bool,

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

    /// Exclude a gitignore-style glob relative to the input directory (repeatable).
    /// Leading / anchors to the scan root; ! negation is not accepted.
    #[arg(long = "ignore", value_name = "GLOB", action = ArgAction::Append)]
    ignores: Vec<String>,

    /// Disable root and nested .gitignore rules; explicit --ignore still applies.
    #[arg(long)]
    no_gitignore: bool,

    /// Replace changed directory entries without y/n confirmation.
    #[arg(short = 'f', long)]
    force: bool,

    /// Report findings without writing transformed data.
    #[arg(long)]
    dry_run: bool,

    /// Disable backups for in-place writes; accepted in all input modes.
    #[arg(long)]
    no_backup: bool,

    /// Hide warnings and logs, keeping dry-run findings, errors, and prompts.
    #[arg(short = 'q', long, conflicts_with = "verbose")]
    quiet: bool,

    /// Report each input and totals; repeat to show individual matches.
    #[arg(short = 'v', long, action = ArgAction::Count, conflicts_with = "quiet")]
    verbose: u8,

    /// Control colors in findings and diagnostics.
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
    Preset(#[from] TextPresetError),
    #[error(transparent)]
    PresetLoad(#[from] TextPresetLoadError),
    #[error(transparent)]
    File(#[from] FileError),
    #[error(transparent)]
    Directory(#[from] DirectoryError),
    #[error(transparent)]
    Io(#[from] io::Error),
}

impl CliError {
    const fn exit_code(&self) -> u8 {
        match self {
            Self::Usage(_) | Self::Preset(_) | Self::PresetLoad(_) => EXIT_CONFIGURATION,
            Self::File(_) | Self::Io(_) => EXIT_RUNTIME,
            Self::Directory(DirectoryError::File(_)) => EXIT_RUNTIME,
            Self::Directory(_) => EXIT_CONFIGURATION,
        }
    }
}

#[derive(Debug, Default)]
struct Summary {
    findings: usize,
    affected_inputs: usize,
    processed_inputs: usize,
    skipped_inputs: usize,
    declined_inputs: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Verbosity {
    Quiet,
    Normal,
    Verbose,
    Detailed,
}

struct Diagnostics {
    color: ColorMode,
    verbosity: Verbosity,
    summary: Summary,
}

impl Diagnostics {
    fn new(cli: &Cli) -> Self {
        Self {
            color: cli.color,
            verbosity: if cli.quiet {
                Verbosity::Quiet
            } else {
                match cli.verbose {
                    0 => Verbosity::Normal,
                    1 => Verbosity::Verbose,
                    _ => Verbosity::Detailed,
                }
            },
            summary: Summary::default(),
        }
    }

    fn record_processed(&mut self, count: usize) {
        self.summary.processed_inputs += 1;
        self.summary.findings += count;
        self.summary.affected_inputs += usize::from(count != 0);
    }

    fn checked(&mut self, path: &Path, findings: &[TextFinding<'_>]) -> io::Result<()> {
        self.record_processed(findings.len());
        print_findings(stdout_stream(self.color), path, findings)?;
        if self.verbosity >= Verbosity::Verbose {
            writeln!(
                stderr_stream(self.color),
                "checked {}: {} finding(s)",
                path.display(),
                findings.len()
            )?;
        }
        Ok(())
    }

    fn transformed(&mut self, path: &Path, count: usize) -> io::Result<()> {
        self.record_processed(count);
        if self.verbosity >= Verbosity::Verbose {
            let mut diagnostics = stderr_stream(self.color);
            if count == 0 {
                writeln!(diagnostics, "unchanged {} (no matches)", path.display())?;
            } else {
                writeln!(
                    diagnostics,
                    "processed {}: {count} replacement(s)",
                    path.display()
                )?;
            }
        }
        Ok(())
    }

    fn updated(&mut self, path: &Path, backup: Option<&Path>, count: usize) -> io::Result<()> {
        self.record_processed(count);
        if self.verbosity == Verbosity::Quiet {
            return Ok(());
        }
        let mut diagnostics = stderr_stream(self.color);
        write!(diagnostics, "updated {} (", path.display())?;
        match backup {
            Some(backup) => write!(diagnostics, "backup: {}", backup.display())?,
            None => write!(diagnostics, "no backup")?,
        }
        if self.verbosity >= Verbosity::Verbose {
            write!(diagnostics, "; {count} replacement(s)")?;
        }
        writeln!(diagnostics, ")")
    }

    fn declined(&mut self, path: &Path) -> io::Result<()> {
        self.summary.processed_inputs += 1;
        self.summary.declined_inputs += 1;
        if self.verbosity >= Verbosity::Verbose {
            writeln!(stderr_stream(self.color), "declined {}", path.display())?;
        }
        Ok(())
    }

    fn skipped(&mut self, path: &Path, reason: SkipReason) -> io::Result<()> {
        self.summary.skipped_inputs += 1;
        if self.verbosity == Verbosity::Quiet
            || (reason == SkipReason::Binary && self.verbosity == Verbosity::Normal)
        {
            return Ok(());
        }
        let reason_text = match reason {
            SkipReason::Binary => "binary file",
            SkipReason::InvalidUtf8 => "invalid UTF-8",
            SkipReason::SymbolicLink => "symbolic link",
            SkipReason::UnsupportedFileType => "unsupported file type",
        };
        let prefix = if reason == SkipReason::Binary {
            ""
        } else {
            "warning: "
        };
        writeln!(
            stderr_stream(self.color),
            "{prefix}skipped {}: {reason_text}",
            path.display()
        )
    }

    fn details(&self, engine: &TextEngine, path: &Path, text: &str) -> io::Result<()> {
        if self.verbosity == Verbosity::Detailed {
            print_findings(stderr_stream(self.color), path, &engine.find(text))?;
        }
        Ok(())
    }

    fn dry_summary(&self) -> io::Result<()> {
        if self.verbosity == Verbosity::Normal {
            writeln!(
                stderr_stream(self.color),
                "{} finding(s) in {} input(s)",
                self.summary.findings,
                self.summary.affected_inputs
            )?;
        }
        Ok(())
    }

    fn finish(&self, dry_run: bool) -> io::Result<()> {
        if self.verbosity >= Verbosity::Verbose {
            let Summary {
                findings,
                affected_inputs,
                processed_inputs,
                skipped_inputs,
                declined_inputs,
            } = self.summary;
            let kind = if dry_run {
                "finding(s)"
            } else {
                "replacement(s)"
            };
            writeln!(
                stderr_stream(self.color),
                "{processed_inputs} processed input(s), {skipped_inputs} skipped input(s), \
                 {declined_inputs} declined input(s); {findings} {kind} in {affected_inputs} input(s)"
            )?;
        }
        Ok(())
    }
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
    let mut diagnostics = Diagnostics::new(&cli);

    let code = match cli.paths.as_slice() {
        [] => run_stdin(&engine, &cli, &mut diagnostics),
        [input] if is_directory(input)? => run_directory(&engine, input, &cli, &mut diagnostics),
        [input] => run_file_to_stdout(&engine, input, &cli, &mut diagnostics),
        [input, output] => run_file_to_file(&engine, input, output, &cli, &mut diagnostics),
        _ => unreachable!("clap limits paths to at most two"),
    }?;
    diagnostics.finish(cli.dry_run)?;
    Ok(code)
}

fn load_engine(cli: &Cli) -> Result<TextEngine, CliError> {
    let preset = if cli.ai_preset {
        TextPreset::ai()
    } else if cli.ascii_preset {
        TextPreset::ascii()
    } else if let Some(path) = &cli.preset {
        TextPreset::load(path)?
    } else if let Some(inline) = &cli.inline_preset {
        TextPreset::parse_inline(inline)?
    } else {
        unreachable!("clap requires exactly one preset source")
    };
    Ok(preset.engine())
}

fn validate_path_options(cli: &Cli) -> Result<(), CliError> {
    if (!cli.ignores.is_empty() || cli.no_gitignore)
        && (cli.paths.len() != 1 || !is_directory(&cli.paths[0])?)
    {
        return Err(CliError::Usage(
            "--ignore and --no-gitignore require one directory input".to_owned(),
        ));
    }
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

fn run_stdin(
    engine: &TextEngine,
    cli: &Cli,
    diagnostics: &mut Diagnostics,
) -> Result<ExitCode, CliError> {
    if cli.recursive || cli.force {
        return Err(CliError::Usage(
            "--recursive and --force cannot be used with stdin".to_owned(),
        ));
    }

    let mut bytes = Vec::new();
    io::stdin().read_to_end(&mut bytes)?;
    let path = Path::new("<stdin>");
    if bytes.contains(&0) {
        diagnostics.skipped(path, SkipReason::Binary)?;
        return Ok(ExitCode::SUCCESS);
    }
    let Ok(input) = String::from_utf8(bytes) else {
        diagnostics.skipped(path, SkipReason::InvalidUtf8)?;
        return Ok(ExitCode::SUCCESS);
    };

    if cli.dry_run {
        let findings = engine.find(&input);
        diagnostics.checked(path, &findings)?;
        diagnostics.dry_summary()?;
        return Ok(findings_exit(findings.len()));
    }

    let replacement = engine.replace(&input);
    diagnostics.details(engine, path, &input)?;
    io::stdout().write_all(replacement.text.as_bytes())?;
    diagnostics.transformed(path, replacement.count)?;
    Ok(ExitCode::SUCCESS)
}

fn run_file_to_stdout(
    engine: &TextEngine,
    input: &Path,
    cli: &Cli,
    diagnostics: &mut Diagnostics,
) -> Result<ExitCode, CliError> {
    if cli.recursive || cli.force {
        return Err(CliError::Usage(
            "--recursive and --force require a directory input".to_owned(),
        ));
    }

    let loaded = read_text(input)?;
    let TextFile::Text(text) = loaded else {
        if let TextFile::Skipped(reason) = loaded {
            diagnostics.skipped(input, reason)?;
        }
        return Ok(ExitCode::SUCCESS);
    };

    if cli.dry_run {
        let findings = engine.find(&text);
        diagnostics.checked(input, &findings)?;
        diagnostics.dry_summary()?;
        return Ok(findings_exit(findings.len()));
    }

    let replacement = engine.replace(&text);
    diagnostics.details(engine, input, &text)?;
    io::stdout().write_all(replacement.text.as_bytes())?;
    diagnostics.transformed(input, replacement.count)?;
    Ok(ExitCode::SUCCESS)
}

fn run_file_to_file(
    engine: &TextEngine,
    input: &Path,
    output: &Path,
    cli: &Cli,
    diagnostics: &mut Diagnostics,
) -> Result<ExitCode, CliError> {
    if cli.recursive || cli.force {
        return Err(CliError::Usage(
            "--recursive and --force require a directory input".to_owned(),
        ));
    }

    let loaded = read_text(input)?;
    let TextFile::Text(text) = loaded else {
        if let TextFile::Skipped(reason) = loaded {
            diagnostics.skipped(input, reason)?;
        }
        return Ok(ExitCode::SUCCESS);
    };
    let replacement = engine.replace(&text);
    diagnostics.details(engine, input, &text)?;

    if same_path(input, output)? {
        if replacement.changed() {
            let backup = write_in_place(input, &replacement.text, cli.no_backup)?;
            diagnostics.updated(input, backup.as_deref(), replacement.count)?;
        } else {
            diagnostics.transformed(input, 0)?;
        }
    } else {
        write_atomic(output, &replacement.text, Some(input))?;
        diagnostics.transformed(input, replacement.count)?;
    }
    Ok(ExitCode::SUCCESS)
}

fn run_directory(
    engine: &TextEngine,
    directory: &Path,
    cli: &Cli,
    diagnostics: &mut Diagnostics,
) -> Result<ExitCode, CliError> {
    let options = DirectoryOptions {
        recursive: cli.recursive,
        respect_gitignore: !cli.no_gitignore,
        ignores: cli.ignores.clone(),
    };
    let paths = collect_directory_files_with_options(directory, &options)?;

    for path in paths {
        let loaded = read_text(&path)?;
        let TextFile::Text(text) = loaded else {
            if let TextFile::Skipped(reason) = loaded {
                diagnostics.skipped(&path, reason)?;
            }
            continue;
        };

        if cli.dry_run {
            let findings = engine.find(&text);
            diagnostics.checked(&path, &findings)?;
            continue;
        }

        let replacement = engine.replace(&text);
        if !replacement.changed() {
            diagnostics.transformed(&path, 0)?;
            continue;
        }
        diagnostics.details(engine, &path, &text)?;
        if !cli.force && !confirm(&path, cli.color)? {
            diagnostics.declined(&path)?;
            continue;
        }
        let backup = write_in_place(&path, &replacement.text, cli.no_backup)?;
        diagnostics.updated(&path, backup.as_deref(), replacement.count)?;
    }

    if cli.dry_run {
        diagnostics.dry_summary()?;
        Ok(findings_exit(diagnostics.summary.findings))
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

fn write_in_place(path: &Path, text: &str, no_backup: bool) -> Result<Option<PathBuf>, FileError> {
    if no_backup {
        write_atomic(path, text, None)?;
        Ok(None)
    } else {
        replace_in_place(path, text).map(Some)
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

fn print_findings(
    mut output: impl Write,
    path: &Path,
    findings: &[TextFinding<'_>],
) -> io::Result<()> {
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
            display_text(finding.replacement),
            path = path.display(),
        )?;
    }
    Ok(())
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

fn display_text(text: &str) -> String {
    text.chars().map(display_character).collect()
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
