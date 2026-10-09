use std::fs;
use std::io;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use purra::files::{
    DirectoryError, DirectoryOptions, FileError, collect_directory_files,
    collect_directory_files_with_options,
};
use tempfile::{TempDir, tempdir};

fn fixture(files: &[(&str, &str)]) -> TempDir {
    let directory = tempdir().unwrap();
    for (name, contents) in files {
        let path = directory.path().join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }
    directory
}

fn discover(root: &Path, options: &DirectoryOptions) -> Vec<PathBuf> {
    collect_directory_files_with_options(root, options)
        .unwrap()
        .into_iter()
        .map(|path| path.strip_prefix(root).unwrap().to_path_buf())
        .collect()
}

fn names(names: &[&str]) -> Vec<PathBuf> {
    names.iter().map(PathBuf::from).collect()
}

#[test]
fn default_options_filter_flat_scans_without_changing_legacy_discovery() {
    let directory = fixture(&[
        (".gitignore", "ignored.txt\n"),
        (".hidden.txt", "a"),
        (".top.txt.purra.bak.2", "a"),
        ("ignored.txt", "a"),
        ("top.txt", "a"),
        ("nested/.gitignore", "[z-a]\n"),
        ("nested/child.txt", "a"),
    ]);
    let options = DirectoryOptions::default();
    assert!(!options.recursive);
    assert!(options.respect_gitignore);
    assert!(options.ignores.is_empty());
    assert_eq!(
        discover(directory.path(), &options),
        names(&[".gitignore", ".hidden.txt", "top.txt"])
    );
    let legacy = collect_directory_files(directory.path(), false).unwrap();
    assert!(legacy.contains(&directory.path().join("ignored.txt")));
    assert!(!legacy.contains(&directory.path().join(".top.txt.purra.bak.2")));
}

#[test]
fn nested_gitignore_overrides_ancestors_without_leaking_into_siblings() {
    let directory = fixture(&[
        (".gitignore", "*.tmp\n!root.tmp\nblocked/\n"),
        ("blocked/.gitignore", "!keep.tmp\n[z-a]\n"),
        ("blocked/keep.tmp", "a"),
        ("left/.gitignore", "!keep.tmp\nlocal.txt\n"),
        ("left/deep/.gitignore", "keep.tmp\n"),
        ("left/deep/keep.tmp", "a"),
        ("left/keep.tmp", "a"),
        ("left/local.txt", "a"),
        ("left/other.tmp", "a"),
        ("right/keep.tmp", "a"),
        ("right/local.txt", "a"),
        ("root.tmp", "a"),
        ("skip.tmp", "a"),
    ]);
    assert_eq!(
        discover(
            directory.path(),
            &DirectoryOptions {
                recursive: true,
                ..DirectoryOptions::default()
            }
        ),
        names(&[
            ".gitignore",
            "left/.gitignore",
            "left/deep/.gitignore",
            "left/keep.tmp",
            "right/local.txt",
            "root.tmp",
        ])
    );
}

#[test]
fn explicit_globs_are_root_relative_and_independent_of_existing_paths() {
    let directory = fixture(&[
        ("assets/root.txt", "a"),
        ("nested/assets/child.txt", "a"),
        ("nested/report.pdf", "a"),
        ("notes.txt", "a"),
        ("report.pdf", "a"),
    ]);
    for (patterns, expected) in [
        (
            vec!["/assets/", "*.pdf", "future/"],
            vec!["nested/assets/child.txt", "notes.txt"],
        ),
        (vec!["assets/", "*.pdf"], vec!["notes.txt"]),
        (
            vec!["nested/assets/child.txt", "/report.pdf"],
            vec!["assets/root.txt", "nested/report.pdf", "notes.txt"],
        ),
    ] {
        assert_eq!(
            discover(
                directory.path(),
                &DirectoryOptions {
                    recursive: true,
                    ignores: patterns.into_iter().map(str::to_owned).collect(),
                    ..DirectoryOptions::default()
                }
            ),
            names(&expected)
        );
    }
}

#[test]
fn comments_escaping_and_rule_order_follow_gitignore_syntax() {
    let directory = fixture(&[
        (
            ".gitignore",
            "# comment\n\n\\#literal.txt\n*.tmp\n!keep.tmp\nlast.tmp\n!last.tmp\n",
        ),
        ("#literal.txt", "a"),
        ("!literal.txt", "a"),
        ("data[1].txt", "a"),
        ("keep.tmp", "a"),
        ("last.tmp", "a"),
        ("skip.tmp", "a"),
    ]);
    assert_eq!(
        discover(
            directory.path(),
            &DirectoryOptions {
                ignores: vec![r"\!literal.txt".into(), r"data\[1\].txt".into()],
                ..DirectoryOptions::default()
            }
        ),
        names(&[".gitignore", "keep.tmp", "last.tmp"])
    );
}

#[test]
fn explicit_exclusions_win_over_gitignore_negation_and_prune_directories() {
    let directory = fixture(&[
        (".gitignore", "*.pdf\n!keep.pdf\n!blocked/\n"),
        ("blocked/.gitignore", "[z-a]\n"),
        ("blocked/input.txt", "a"),
        ("keep.pdf", "a"),
        ("visible.txt", "a"),
    ]);
    for patterns in [["*.pdf", "blocked/"], ["blocked/", "*.pdf"]] {
        assert_eq!(
            discover(
                directory.path(),
                &DirectoryOptions {
                    recursive: true,
                    ignores: patterns.into_iter().map(str::to_owned).collect(),
                    ..DirectoryOptions::default()
                }
            ),
            names(&[".gitignore", "visible.txt"])
        );
    }
}

#[test]
fn only_gitignore_inside_the_scan_tree_is_loaded() {
    let directory = fixture(&[
        (".gitignore", "*.txt\n"),
        ("tree/.ignore", "*.txt\n"),
        ("tree/.git/info/exclude", "*.txt\n"),
        ("tree/input.txt", "a"),
    ]);
    let root = directory.path().join("tree");
    assert_eq!(
        discover(
            &root,
            &DirectoryOptions {
                recursive: true,
                ..DirectoryOptions::default()
            }
        ),
        names(&[".git/info/exclude", ".ignore", "input.txt"])
    );
}

#[test]
fn symbolic_links_including_gitignore_are_not_followed() {
    let directory = fixture(&[
        ("outside/invalid.txt", "[z-a]\n"),
        ("outside/nested/input.txt", "a"),
        ("tree/input.txt", "a"),
    ]);
    let root = directory.path().join("tree");
    symlink(
        directory.path().join("outside/invalid.txt"),
        root.join(".gitignore"),
    )
    .unwrap();
    symlink(directory.path().join("outside/nested"), root.join("link")).unwrap();
    assert_eq!(
        discover(
            &root,
            &DirectoryOptions {
                recursive: true,
                ..DirectoryOptions::default()
            }
        ),
        names(&[".gitignore", "input.txt", "link"])
    );
    assert!(
        collect_directory_files_with_options(root.join("link"), &DirectoryOptions::default())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn malformed_rules_report_the_source_and_line() {
    let directory = fixture(&[(".gitignore", "valid.txt\n[z-a]\n"), ("valid.txt", "a")]);
    let error =
        collect_directory_files_with_options(directory.path(), &DirectoryOptions::default())
            .unwrap_err();
    assert!(matches!(error, DirectoryError::InvalidPattern { .. }));
    assert!(error.to_string().contains(".gitignore:2"));

    for pattern in ["[z-a]", "!valid.txt"] {
        let error = collect_directory_files_with_options(
            directory.path(),
            &DirectoryOptions {
                ignores: vec![pattern.into()],
                ..DirectoryOptions::default()
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("--ignore"));
    }
}

#[test]
fn unreadable_ignore_contents_are_file_errors() {
    let directory = fixture(&[("input.txt", "a")]);
    let ignore = directory.path().join(".gitignore");
    fs::write(&ignore, [0xff]).unwrap();
    let error =
        collect_directory_files_with_options(directory.path(), &DirectoryOptions::default())
            .unwrap_err();
    assert!(matches!(
        error,
        DirectoryError::File(FileError::Io { source, .. })
            if source.kind() == io::ErrorKind::InvalidData
    ));
    fs::remove_file(&ignore).unwrap();
    fs::create_dir(&ignore).unwrap();
    assert!(matches!(
        collect_directory_files_with_options(directory.path(), &DirectoryOptions::default()),
        Err(DirectoryError::File(FileError::Io { .. }))
    ));
}

#[test]
fn disabled_gitignore_does_not_load_any_rules_but_keeps_explicit_exclusions() {
    let directory = fixture(&[
        (".gitignore", "[z-a]\n"),
        ("nested/.gitignore", "[z-a]\n"),
        ("nested/input.txt", "a"),
        ("nested/skip.pdf", "a"),
    ]);
    assert_eq!(
        discover(
            directory.path(),
            &DirectoryOptions {
                recursive: true,
                respect_gitignore: false,
                ignores: vec!["*.pdf".into()],
            }
        ),
        names(&[".gitignore", "nested/.gitignore", "nested/input.txt"])
    );
}

#[test]
fn cli_uses_the_same_filters_for_dry_run_and_writes_without_skip_warnings() {
    let directory = fixture(&[
        (".gitignore", "ignored.txt\n"),
        ("ignored.txt", "a"),
        ("nested/input.txt", "a"),
        ("nested/report.pdf", "a"),
        (".git/info", "a"),
    ]);
    fs::write(directory.path().join("invalid.txt"), [0xff]).unwrap();
    symlink("missing", directory.path().join("link.txt")).unwrap();
    let exclusions = [
        "--ignore",
        "*.pdf",
        "--ignore",
        ".git/",
        "--ignore",
        "invalid.txt",
        "--ignore",
        "link.txt",
    ];
    cargo_bin_cmd!("purra")
        .args([
            "--in-place-preset",
            "a=b",
            "-r",
            "--dry-run",
            "--color",
            "never",
        ])
        .args(exclusions)
        .arg(directory.path())
        .assert()
        .code(1)
        .stdout(format!(
            "{}:1:1: a -> b\n",
            directory.path().join("nested/input.txt").display()
        ))
        .stderr("1 finding(s) in 1 input(s)\n");
    cargo_bin_cmd!("purra")
        .args(["--in-place-preset", "a=b", "-r", "-f", "--color", "never"])
        .args(exclusions)
        .arg(directory.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("updated").and(predicate::str::contains("warning").not()));
    for name in ["ignored.txt", "nested/report.pdf", ".git/info"] {
        assert_eq!(
            fs::read_to_string(directory.path().join(name)).unwrap(),
            "a"
        );
        let path = directory.path().join(name);
        let backup = path.parent().unwrap().join(format!(
            ".{}.purra.bak",
            path.file_name().unwrap().to_str().unwrap()
        ));
        assert!(!backup.exists());
    }
    assert_eq!(
        fs::read_to_string(directory.path().join("nested/input.txt")).unwrap(),
        "b"
    );
}

#[test]
fn exclusions_do_not_prompt_and_an_empty_dry_run_exits_zero() {
    let directory = fixture(&[(".gitignore", "*.txt\n"), ("input.txt", "a")]);
    cargo_bin_cmd!("purra")
        .args(["--in-place-preset", "a=b", "--color", "never"])
        .arg(directory.path())
        .assert()
        .success()
        .stdout("")
        .stderr("");
    cargo_bin_cmd!("purra")
        .args(["--in-place-preset", "a=b", "--dry-run", "--color", "never"])
        .arg(directory.path())
        .assert()
        .success()
        .stdout("")
        .stderr("0 finding(s) in 0 input(s)\n");
    assert!(!directory.path().join(".input.txt.purra.bak").exists());
}

#[test]
fn cli_globs_are_relative_to_the_scan_root_with_dot_and_named_paths() {
    let directory = fixture(&[
        ("tree/.gitignore", "ignored.txt\n"),
        ("tree/ignored.txt", "a"),
        ("tree/assets/input.txt", "a"),
        ("tree/nested/assets/input.txt", "a"),
    ]);
    for (cwd, root) in [
        (directory.path().to_path_buf(), "./tree"),
        (directory.path().join("tree"), "."),
    ] {
        cargo_bin_cmd!("purra")
            .current_dir(cwd)
            .args([
                "--in-place-preset",
                "a=b",
                "-r",
                "--dry-run",
                "--color",
                "never",
                "--ignore",
                "/assets/",
                root,
            ])
            .assert()
            .code(1)
            .stdout(
                predicate::str::contains("nested/assets/input.txt:1:1")
                    .and(predicate::str::contains("ignored.txt").not()),
            )
            .stderr("1 finding(s) in 1 input(s)\n");
    }
}

#[test]
fn cli_configuration_errors_leave_all_inputs_and_backups_untouched() {
    for (ignore_path, contents, args, message) in [
        (".gitignore", "keep.txt\n[z-a]\n", vec![], ".gitignore:2"),
        ("nested/.gitignore", "[z-a]\n", vec![], ".gitignore:1"),
        (
            ".gitignore",
            "",
            vec!["--ignore", "keep.txt", "--ignore", "[z-a]"],
            "--ignore",
        ),
        (".gitignore", "", vec!["--ignore", "!keep.txt"], "negation"),
    ] {
        let directory = fixture(&[
            (ignore_path, contents),
            ("keep.txt", "a"),
            ("nested/input.txt", "a"),
        ]);
        cargo_bin_cmd!("purra")
            .args(["--in-place-preset", "a=b", "-r", "-f", "--color", "never"])
            .args(args)
            .arg(directory.path())
            .assert()
            .code(2)
            .stdout("")
            .stderr(
                predicate::str::contains(message).and(predicate::str::contains("updated").not()),
            );
        for name in ["keep.txt", "nested/input.txt"] {
            assert_eq!(
                fs::read_to_string(directory.path().join(name)).unwrap(),
                "a"
            );
        }
        assert!(
            walkdir::WalkDir::new(directory.path())
                .into_iter()
                .all(|entry| {
                    !entry
                        .unwrap()
                        .file_name()
                        .to_string_lossy()
                        .contains(".purra.bak")
                })
        );
    }
}

#[test]
fn cli_ignore_io_errors_exit_three_without_changes() {
    let directory = fixture(&[("input.txt", "a")]);
    fs::create_dir(directory.path().join(".gitignore")).unwrap();
    cargo_bin_cmd!("purra")
        .args(["--in-place-preset", "a=b", "-f"])
        .arg(directory.path())
        .assert()
        .code(3)
        .stderr(predicate::str::contains("read ignore file"));
    assert_eq!(
        fs::read_to_string(directory.path().join("input.txt")).unwrap(),
        "a"
    );
    assert!(!directory.path().join(".input.txt.purra.bak").exists());
}

#[test]
fn cli_no_gitignore_bypasses_invalid_rules_and_still_applies_ignore() {
    let directory = fixture(&[
        (".gitignore", "[z-a]\n"),
        ("nested/.gitignore", "[z-a]\n"),
        ("nested/input.txt", "a"),
        ("nested/report.pdf", "a"),
    ]);
    cargo_bin_cmd!("purra")
        .args([
            "--in-place-preset",
            "a=b",
            "-r",
            "--dry-run",
            "--no-gitignore",
            "--ignore",
            "*.pdf",
            "--color",
            "never",
        ])
        .arg(directory.path())
        .assert()
        .code(1)
        .stdout(
            predicate::str::contains("nested/input.txt:1:1")
                .and(predicate::str::contains("report.pdf").not()),
        )
        .stderr("3 finding(s) in 3 input(s)\n");
}

#[test]
fn directory_filter_flags_reject_stdin_explicit_files_and_output_paths() {
    let directory = fixture(&[("input.txt", "a"), ("output.txt", "sentinel")]);
    for flags in [vec!["--ignore", "*.txt"], vec!["--no-gitignore"]] {
        for paths in [vec![], vec!["input.txt"], vec!["input.txt", "output.txt"]] {
            cargo_bin_cmd!("purra")
                .current_dir(directory.path())
                .args(["--in-place-preset", "a=b"])
                .args(&flags)
                .args(paths)
                .write_stdin("a")
                .assert()
                .code(2)
                .stdout("")
                .stderr(predicate::str::contains("require one directory input"));
        }
    }
    assert_eq!(
        fs::read_to_string(directory.path().join("output.txt")).unwrap(),
        "sentinel"
    );
}

#[test]
fn invalid_preset_is_rejected_before_directory_rules() {
    let directory = fixture(&[(".gitignore", "[z-a]\n"), ("input.txt", "a")]);
    cargo_bin_cmd!("purra")
        .args(["--in-place-preset", "a=b,a=c", "--ignore", "[z-a]"])
        .arg(directory.path())
        .assert()
        .code(2)
        .stderr(predicate::str::contains("duplicate source character"));
}
