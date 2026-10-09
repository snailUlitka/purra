use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn transforms_stdin_with_inline_unicode_rules() {
    cargo_bin_cmd!("purra")
        .args(["--in-place-preset", "🇺=🇨,🇸=🇦"])
        .write_stdin("flag: 🇺🇸")
        .assert()
        .success()
        .stdout("flag: 🇨🇦")
        .stderr("");
}

#[test]
fn rejects_a_grapheme_that_is_not_one_unicode_scalar() {
    cargo_bin_cmd!("purra")
        .args(["--in-place-preset", "🇺🇸=🇨🇦"])
        .write_stdin("untouched")
        .assert()
        .code(2)
        .stdout("")
        .stderr(predicate::str::contains(
            "source must decode to exactly one Unicode scalar value",
        ));
}

#[test]
fn transforms_stdin_with_scalar_to_text_rules() {
    cargo_bin_cmd!("purra")
        .args(["--in-place-preset", r"…=...,≠=!\="])
        .write_stdin("a…b ≠ c")
        .assert()
        .success()
        .stdout("a...b != c")
        .stderr("");
}

#[test]
fn ascii_preset_includes_ai_typography_and_selected_expansions() {
    cargo_bin_cmd!("purra")
        .arg("--ascii-preset")
        .write_stdin("“A—B…” ﬁ → ⇒ ⇐ ⇔ ≠ ≤ ≥ ≡")
        .assert()
        .success()
        .stdout("\"A-B...\" fi -> ==> <== <==> != <= >= ===")
        .stderr("");
}

#[test]
fn ai_preset_does_not_gain_ascii_expansions() {
    cargo_bin_cmd!("purra")
        .arg("--ai-preset")
        .write_stdin("… → ≠")
        .assert()
        .success()
        .stdout("… → ≠")
        .stderr("");
}

#[test]
fn validates_preset_before_touching_input_or_output() {
    let directory = tempdir().unwrap();
    let missing_input = directory.path().join("missing.txt");
    let output = directory.path().join("output.txt");
    fs::write(&output, "sentinel").unwrap();

    cargo_bin_cmd!("purra")
        .arg("--in-place-preset")
        .arg("a=b,a=c")
        .arg(&missing_input)
        .arg(&output)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("duplicate source character"));

    assert_eq!(fs::read_to_string(output).unwrap(), "sentinel");
}

#[test]
fn transforms_between_files() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.txt");
    let output = directory.path().join("output.txt");
    fs::write(&input, "before—after").unwrap();

    cargo_bin_cmd!("purra")
        .args(["--ai-preset"])
        .arg(&input)
        .arg(&output)
        .assert()
        .success()
        .stdout("")
        .stderr("");

    assert_eq!(fs::read_to_string(input).unwrap(), "before—after");
    assert_eq!(fs::read_to_string(output).unwrap(), "before-after");
}

#[test]
fn dry_run_reports_unicode_scalar_positions_and_exit_one() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("flags.txt");
    fs::write(&input, "é🇺🇸\n🇺").unwrap();

    cargo_bin_cmd!("purra")
        .args([
            "--in-place-preset",
            "🇺=🇨,🇸=🇦",
            "--dry-run",
            "--color",
            "never",
        ])
        .arg(&input)
        .assert()
        .code(1)
        .stdout(
            predicate::str::contains(format!("{}:1:2: 🇺 -> 🇨", input.display()))
                .and(predicate::str::contains(format!(
                    "{}:1:3: 🇸 -> 🇦",
                    input.display()
                )))
                .and(predicate::str::contains(format!(
                    "{}:2:1: 🇺 -> 🇨",
                    input.display()
                ))),
        )
        .stderr("3 finding(s) in 1 input(s)\n");

    assert_eq!(fs::read_to_string(input).unwrap(), "é🇺🇸\n🇺");
}

#[test]
fn dry_run_without_findings_exits_zero() {
    cargo_bin_cmd!("purra")
        .args(["--in-place-preset", "a=b", "--dry-run", "--color", "never"])
        .write_stdin("pure")
        .assert()
        .success()
        .stdout("")
        .stderr("0 finding(s) in 0 input(s)\n");
}

#[test]
fn dry_run_displays_text_replacement_and_counts_the_source_once() {
    cargo_bin_cmd!("purra")
        .args(["--ascii-preset", "--dry-run", "--color", "never"])
        .write_stdin("é…")
        .assert()
        .code(1)
        .stdout("<stdin>:1:2: … -> ...\n")
        .stderr("1 finding(s) in 1 input(s)\n");
}

#[test]
fn forced_directory_mode_is_non_recursive_and_creates_backup() {
    let directory = tempdir().unwrap();
    let top = directory.path().join("top.txt");
    let nested_directory = directory.path().join("nested");
    let nested = nested_directory.join("nested.txt");
    fs::create_dir(&nested_directory).unwrap();
    fs::write(&top, "a").unwrap();
    fs::write(&nested, "a").unwrap();

    cargo_bin_cmd!("purra")
        .args(["--in-place-preset", "a=b", "--force", "--color", "never"])
        .arg(directory.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("updated"));

    assert_eq!(fs::read_to_string(&top).unwrap(), "b");
    assert_eq!(fs::read_to_string(&nested).unwrap(), "a");
    assert_eq!(
        fs::read_to_string(directory.path().join(".top.txt.purra.bak")).unwrap(),
        "a"
    );
}

#[test]
fn recursive_directory_mode_honors_confirmation() {
    let directory = tempdir().unwrap();
    let nested_directory = directory.path().join("nested");
    let nested = nested_directory.join("nested.txt");
    fs::create_dir(&nested_directory).unwrap();
    fs::write(&nested, "a").unwrap();

    cargo_bin_cmd!("purra")
        .args([
            "--in-place-preset",
            "a=b",
            "--recursive",
            "--color",
            "never",
        ])
        .arg(directory.path())
        .write_stdin("y\n")
        .assert()
        .success()
        .stderr(predicate::str::contains("replace").and(predicate::str::contains("updated")));

    assert_eq!(fs::read_to_string(nested).unwrap(), "b");
}

#[test]
fn declining_directory_confirmation_leaves_no_backup() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.txt");
    fs::write(&input, "a").unwrap();

    cargo_bin_cmd!("purra")
        .args(["--in-place-preset", "a=b", "--color", "never"])
        .arg(directory.path())
        .write_stdin("n\n")
        .assert()
        .success()
        .stderr(predicate::str::contains("replace"));

    assert_eq!(fs::read_to_string(input).unwrap(), "a");
    assert!(!directory.path().join(".input.txt.purra.bak").exists());
}

#[test]
fn loads_external_preset_with_comments_and_escapes() {
    let directory = tempdir().unwrap();
    let preset = directory.path().join("custom.preset");
    let input = directory.path().join("input.txt");
    fs::write(&preset, "# normalize\n—=-\n\\s=_\n").unwrap();
    fs::write(&input, "a — b").unwrap();

    cargo_bin_cmd!("purra")
        .arg("--preset")
        .arg(&preset)
        .arg(&input)
        .assert()
        .success()
        .stdout("a_-_b");
}

#[test]
fn ignores_binary_and_warns_for_invalid_utf8() {
    let directory = tempdir().unwrap();
    let binary = directory.path().join("binary.bin");
    let invalid = directory.path().join("invalid.txt");
    fs::write(&binary, b"a\0a").unwrap();
    fs::write(&invalid, [0xff]).unwrap();

    cargo_bin_cmd!("purra")
        .args(["--in-place-preset", "a=b", "--force", "--color", "never"])
        .arg(directory.path())
        .assert()
        .success()
        .stdout("")
        .stderr(
            predicate::str::contains("invalid UTF-8").and(predicate::str::contains("binary").not()),
        );

    assert_eq!(fs::read(&binary).unwrap(), b"a\0a");
    assert_eq!(fs::read(&invalid).unwrap(), [0xff]);
}

#[test]
fn no_backup_recursively_updates_files_and_preserves_permissions_and_existing_backups() {
    let directory = tempdir().unwrap();
    let top = directory.path().join("top.txt");
    let existing_backup = directory.path().join(".top.txt.purra.bak");
    let nested_directory = directory.path().join("nested");
    let nested = nested_directory.join("nested.txt");
    fs::create_dir(&nested_directory).unwrap();
    fs::write(&top, "a").unwrap();
    fs::write(&nested, "aa").unwrap();
    fs::write(&existing_backup, "a sentinel").unwrap();
    fs::set_permissions(&top, fs::Permissions::from_mode(0o640)).unwrap();
    fs::set_permissions(&nested, fs::Permissions::from_mode(0o600)).unwrap();

    cargo_bin_cmd!("purra")
        .args([
            "--in-place-preset",
            "a=b",
            "-rf",
            "--no-backup",
            "--color",
            "never",
        ])
        .arg(directory.path())
        .assert()
        .success()
        .stdout("")
        .stderr(predicate::str::contains("no backup"));

    assert_eq!(fs::read_to_string(&top).unwrap(), "b");
    assert_eq!(fs::read_to_string(&nested).unwrap(), "bb");
    assert_eq!(
        fs::metadata(&top).unwrap().permissions().mode() & 0o777,
        0o640
    );
    assert_eq!(
        fs::metadata(&nested).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(fs::read_to_string(existing_backup).unwrap(), "a sentinel");
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 3);
    assert_eq!(fs::read_dir(nested_directory).unwrap().count(), 1);
}

#[test]
fn same_file_output_creates_unique_backups_by_default_and_can_disable_them() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.txt");
    let alias = directory.path().join(".").join("input.txt");
    fs::write(&input, "a").unwrap();
    fs::set_permissions(&input, fs::Permissions::from_mode(0o640)).unwrap();

    for (rule, original) in [("a=b", "a"), ("b=c", "b")] {
        cargo_bin_cmd!("purra")
            .args(["--in-place-preset", rule, "--color", "never"])
            .arg(&input)
            .arg(&alias)
            .assert()
            .success()
            .stderr(predicate::str::contains("backup:"));
        let backup = if original == "a" {
            ".input.txt.purra.bak"
        } else {
            ".input.txt.purra.bak.1"
        };
        assert_eq!(
            fs::read_to_string(directory.path().join(backup)).unwrap(),
            original
        );
    }

    cargo_bin_cmd!("purra")
        .args([
            "--in-place-preset",
            "c=d",
            "--no-backup",
            "--color",
            "never",
        ])
        .arg(&input)
        .arg(&input)
        .assert()
        .success()
        .stderr(format!("updated {} (no backup)\n", input.display()));

    assert_eq!(fs::read_to_string(&input).unwrap(), "d");
    assert_eq!(
        fs::metadata(&input).unwrap().permissions().mode() & 0o777,
        0o640
    );
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 3);
    assert_eq!(
        fs::read_to_string(directory.path().join(".input.txt.purra.bak")).unwrap(),
        "a"
    );
    assert_eq!(
        fs::read_to_string(directory.path().join(".input.txt.purra.bak.1")).unwrap(),
        "b"
    );
}

#[test]
fn no_backup_is_accepted_for_distinct_output_and_file_to_stdout() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.txt");
    let output = directory.path().join("output.txt");
    fs::write(&input, "…").unwrap();

    cargo_bin_cmd!("purra")
        .args(["--ascii-preset", "--no-backup"])
        .arg(&input)
        .arg(&output)
        .assert()
        .success()
        .stdout("")
        .stderr("");
    cargo_bin_cmd!("purra")
        .args(["--ascii-preset", "--no-backup"])
        .arg(&input)
        .assert()
        .success()
        .stdout("...")
        .stderr("");

    assert_eq!(fs::read_to_string(input).unwrap(), "…");
    assert_eq!(fs::read_to_string(output).unwrap(), "...");
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
}

#[test]
fn no_backup_does_not_write_during_dry_run_decline_or_invalid_preset() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.txt");
    fs::write(&input, "a").unwrap();

    cargo_bin_cmd!("purra")
        .args([
            "--in-place-preset",
            "a=b",
            "--no-backup",
            "--dry-run",
            "--color",
            "never",
        ])
        .arg(directory.path())
        .assert()
        .code(1)
        .stdout(predicate::str::contains("a -> b"));
    cargo_bin_cmd!("purra")
        .args([
            "--in-place-preset",
            "a=b",
            "--no-backup",
            "--color",
            "never",
        ])
        .arg(directory.path())
        .write_stdin("n\n")
        .assert()
        .success();
    cargo_bin_cmd!("purra")
        .args(["--in-place-preset", "a=b,a=c", "--no-backup", "-f"])
        .arg(directory.path())
        .assert()
        .code(2)
        .stderr(predicate::str::contains("duplicate source character"));

    assert_eq!(fs::read_to_string(input).unwrap(), "a");
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
fn unchanged_in_place_files_do_not_create_backups() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.txt");
    fs::write(&input, "clean").unwrap();

    for no_backup in [false, true] {
        let mut command = cargo_bin_cmd!("purra");
        command.args(["--ascii-preset"]);
        if no_backup {
            command.arg("--no-backup");
        }
        command
            .arg(&input)
            .arg(&input)
            .assert()
            .success()
            .stdout("")
            .stderr("");
    }
    assert_eq!(fs::read_to_string(input).unwrap(), "clean");
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
fn no_backup_skips_input_symlinks_and_refuses_output_symlinks() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.txt");
    let target = directory.path().join("target.txt");
    let link = directory.path().join("link.txt");
    fs::write(&input, "a").unwrap();
    fs::write(&target, "sentinel").unwrap();
    symlink(&target, &link).unwrap();

    cargo_bin_cmd!("purra")
        .args([
            "--in-place-preset",
            "a=b",
            "--no-backup",
            "--color",
            "never",
        ])
        .arg(&link)
        .arg(&link)
        .assert()
        .success()
        .stderr(predicate::str::contains("symbolic link"));
    cargo_bin_cmd!("purra")
        .args(["--in-place-preset", "a=b", "--no-backup", "-q"])
        .arg(&input)
        .arg(&link)
        .assert()
        .code(3)
        .stderr(predicate::str::contains(
            "error: refusing to replace symbolic link",
        ));

    assert_eq!(fs::read_to_string(input).unwrap(), "a");
    assert_eq!(fs::read_to_string(target).unwrap(), "sentinel");
    assert!(fs::symlink_metadata(link).unwrap().file_type().is_symlink());
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 3);
}

#[test]
fn quiet_keeps_transformed_stdout_and_dry_run_findings() {
    for flag in ["-q", "--quiet"] {
        cargo_bin_cmd!("purra")
            .args(["--ascii-preset", flag, "--no-backup"])
            .write_stdin("é…")
            .assert()
            .success()
            .stdout("é...")
            .stderr("");
        cargo_bin_cmd!("purra")
            .args([
                "--ascii-preset",
                flag,
                "--no-backup",
                "--dry-run",
                "--color",
                "never",
            ])
            .write_stdin("é…")
            .assert()
            .code(1)
            .stdout("<stdin>:1:2: … -> ...\n")
            .stderr("");
        cargo_bin_cmd!("purra")
            .args(["--ascii-preset", flag, "--dry-run"])
            .write_stdin("clean")
            .assert()
            .success()
            .stdout("")
            .stderr("");
    }
}

#[test]
fn quiet_directory_hides_warnings_updates_and_summaries_but_keeps_findings() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.txt");
    let invalid = directory.path().join("invalid.txt");
    let binary = directory.path().join("binary.bin");
    let link = directory.path().join("link.txt");
    fs::write(&input, "…").unwrap();
    fs::write(&invalid, [0xff]).unwrap();
    fs::write(&binary, b"a\0b").unwrap();
    symlink(&input, &link).unwrap();

    cargo_bin_cmd!("purra")
        .args(["--ascii-preset", "-q", "--dry-run", "--color", "never"])
        .arg(directory.path())
        .assert()
        .code(1)
        .stdout(format!("{}:1:1: … -> ...\n", input.display()))
        .stderr("");
    cargo_bin_cmd!("purra")
        .args(["--ascii-preset", "--quiet", "-f"])
        .arg(directory.path())
        .assert()
        .success()
        .stdout("")
        .stderr("");

    assert_eq!(fs::read_to_string(&input).unwrap(), "...");
    assert_eq!(
        fs::read_to_string(directory.path().join(".input.txt.purra.bak")).unwrap(),
        "…"
    );
    assert_eq!(fs::read(invalid).unwrap(), [0xff]);
    assert_eq!(fs::read(binary).unwrap(), b"a\0b");
}

#[test]
fn quiet_keeps_confirmation_and_invalid_response_messages() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.txt");
    fs::write(&input, "a").unwrap();

    cargo_bin_cmd!("purra")
        .args([
            "--in-place-preset",
            "a=b",
            "-q",
            "--no-backup",
            "--color",
            "never",
        ])
        .arg(directory.path())
        .write_stdin("maybe\nn\n")
        .assert()
        .success()
        .stderr(format!(
            "replace {}? [y/n] please answer y or n\nreplace {}? [y/n] ",
            input.display(),
            input.display()
        ));
    assert_eq!(fs::read_to_string(&input).unwrap(), "a");

    cargo_bin_cmd!("purra")
        .args([
            "--in-place-preset",
            "a=b",
            "--quiet",
            "--no-backup",
            "--color",
            "never",
        ])
        .arg(directory.path())
        .write_stdin("y\n")
        .assert()
        .success()
        .stderr(format!("replace {}? [y/n] ", input.display()));
    assert_eq!(fs::read_to_string(input).unwrap(), "b");
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
fn quiet_keeps_configuration_usage_and_runtime_errors() {
    cargo_bin_cmd!("purra")
        .args(["--in-place-preset", "a=b,a=c", "-q"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "error: duplicate source character",
        ));
    cargo_bin_cmd!("purra")
        .args([
            "--ascii-preset",
            "-q",
            "--dry-run",
            "input.txt",
            "output.txt",
        ])
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "error: --dry-run does not accept an output path",
        ));

    let directory = tempdir().unwrap();
    cargo_bin_cmd!("purra")
        .args(["--ascii-preset", "--quiet"])
        .arg(directory.path().join("missing.txt"))
        .assert()
        .code(3)
        .stderr(predicate::str::contains("error: failed to inspect"));
}

#[test]
fn quiet_conflicts_with_verbose_in_both_orders_and_spellings() {
    for (quiet, verbose) in [("-q", "-v"), ("--quiet", "--verbose"), ("-q", "-vv")] {
        for flags in [[quiet, verbose], [verbose, quiet]] {
            cargo_bin_cmd!("purra")
                .arg("--ascii-preset")
                .args(flags)
                .assert()
                .code(2)
                .stdout("")
                .stderr(predicate::str::contains("cannot be used with"));
        }
    }
    cargo_bin_cmd!("purra")
        .args(["--ascii-preset", "--quite"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("unexpected argument '--quite'"));
}

#[test]
fn verbose_reports_stdin_counts_without_individual_matches() {
    for flag in ["-v", "--verbose"] {
        cargo_bin_cmd!("purra")
            .args(["--ascii-preset", flag, "--color", "never"])
            .write_stdin("é… ⇒")
            .assert()
            .success()
            .stdout("é... ==>")
            .stderr(
                "processed <stdin>: 2 replacement(s)\n\
                 1 processed input(s), 0 skipped input(s), 0 declined input(s); 2 replacement(s) in 1 input(s)\n"
            );
    }
}

#[test]
fn detailed_verbosity_reports_original_positions_and_caps_further_repetitions() {
    for flags in [vec!["-vv"], vec!["--verbose", "--verbose"], vec!["-vvv"]] {
        cargo_bin_cmd!("purra")
            .args(["--ascii-preset", "--color", "never"])
            .args(flags)
            .write_stdin("é…⇒\n…")
            .assert()
            .success()
            .stdout("é...==>\n...")
            .stderr(
                "<stdin>:1:2: … -> ...\n\
                 <stdin>:1:3: ⇒ -> ==>\n\
                 <stdin>:2:1: … -> ...\n\
                 processed <stdin>: 3 replacement(s)\n\
                 1 processed input(s), 0 skipped input(s), 0 declined input(s); 3 replacement(s) in 1 input(s)\n"
            );
    }
}

#[test]
fn verbose_directory_counts_only_applied_replacements_and_reports_every_status() {
    let directory = tempdir().unwrap();
    let changed = directory.path().join("a-changed.txt");
    let clean = directory.path().join("b-clean.txt");
    let declined = directory.path().join("c-declined.txt");
    let binary = directory.path().join("d-binary.bin");
    let invalid = directory.path().join("e-invalid.txt");
    let link = directory.path().join("f-link.txt");
    fs::write(&changed, "……").unwrap();
    fs::write(&clean, "clean").unwrap();
    fs::write(&declined, "…").unwrap();
    fs::write(&binary, b"a\0b").unwrap();
    fs::write(&invalid, [0xff]).unwrap();
    symlink(&changed, &link).unwrap();

    cargo_bin_cmd!("purra")
        .args(["--ascii-preset", "-v", "--no-backup", "--color", "never"])
        .arg(directory.path())
        .write_stdin("y\nn\n")
        .assert()
        .success()
        .stdout("")
        .stderr(
            predicate::str::contains(format!("updated {} (no backup; 2 replacement(s))", changed.display()))
                .and(predicate::str::contains(format!("unchanged {} (no matches)", clean.display())))
                .and(predicate::str::contains(format!("declined {}", declined.display())))
                .and(predicate::str::contains("binary file"))
                .and(predicate::str::contains("invalid UTF-8"))
                .and(predicate::str::contains("symbolic link"))
                .and(predicate::str::contains("3 processed input(s), 3 skipped input(s), 1 declined input(s); 2 replacement(s) in 1 input(s)"))
        );

    assert_eq!(fs::read_to_string(changed).unwrap(), "......");
    assert_eq!(fs::read_to_string(declined).unwrap(), "…");
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 6);
}

#[test]
fn detailed_directory_shows_proposals_before_confirmation_even_when_declined() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.txt");
    fs::write(&input, "é…").unwrap();

    cargo_bin_cmd!("purra")
        .args(["--ascii-preset", "-vv", "--color", "never"])
        .arg(directory.path())
        .write_stdin("n\n")
        .assert()
        .success()
        .stdout("")
        .stderr(format!(
            "{}:1:2: … -> ...\nreplace {}? [y/n] declined {}\n\
             1 processed input(s), 0 skipped input(s), 1 declined input(s); 0 replacement(s) in 0 input(s)\n",
            input.display(), input.display(), input.display()
        ));

    assert_eq!(fs::read_to_string(input).unwrap(), "é…");
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
fn verbose_dry_run_does_not_duplicate_findings_in_stderr() {
    for flag in ["-v", "-vv"] {
        cargo_bin_cmd!("purra")
            .args(["--ascii-preset", flag, "--dry-run", "--color", "never"])
            .write_stdin("é…⇒")
            .assert()
            .code(1)
            .stdout("<stdin>:1:2: … -> ...\n<stdin>:1:3: ⇒ -> ==>\n")
            .stderr(
                "checked <stdin>: 2 finding(s)\n\
                 1 processed input(s), 0 skipped input(s), 0 declined input(s); 2 finding(s) in 1 input(s)\n"
            );
    }
}

#[test]
fn verbose_file_modes_preserve_stdout_and_report_clean_inputs() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.txt");
    let output = directory.path().join("output.txt");
    fs::write(&input, "é…").unwrap();

    cargo_bin_cmd!("purra")
        .args(["--ascii-preset", "-vv", "--color", "never"])
        .arg(&input)
        .assert()
        .success()
        .stdout("é...")
        .stderr(predicate::str::contains(format!(
            "{}:1:2: … -> ...",
            input.display()
        )));
    cargo_bin_cmd!("purra")
        .args(["--ascii-preset", "-v", "--color", "never"])
        .arg(&input)
        .arg(&output)
        .assert()
        .success()
        .stdout("")
        .stderr(predicate::str::contains("1 replacement(s)"));
    cargo_bin_cmd!("purra")
        .args(["--ascii-preset", "-v", "--color", "never"])
        .arg(&output)
        .arg(&output)
        .assert()
        .success()
        .stdout("")
        .stderr(
            predicate::str::contains(format!("unchanged {} (no matches)", output.display()))
                .and(predicate::str::contains("0 replacement(s) in 0 input(s)")),
        );

    assert_eq!(fs::read_to_string(input).unwrap(), "é…");
    assert_eq!(fs::read_to_string(output).unwrap(), "é...");
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
}

#[test]
fn verbose_reports_skipped_stdin_and_quiet_hides_its_warning() {
    cargo_bin_cmd!("purra")
        .args(["--ascii-preset", "-v", "--color", "never"])
        .write_stdin(b"a\0b")
        .assert()
        .success()
        .stdout("")
        .stderr(
            "skipped <stdin>: binary file\n\
             0 processed input(s), 1 skipped input(s), 0 declined input(s); 0 replacement(s) in 0 input(s)\n"
        );
    cargo_bin_cmd!("purra")
        .args(["--ascii-preset", "--quiet"])
        .write_stdin([0xff])
        .assert()
        .success()
        .stdout("")
        .stderr("");
}

#[test]
fn detailed_findings_honor_color_for_stderr_without_styling_transformed_stdout() {
    for (color, styled) in [("never", false), ("always", true)] {
        let assertion = cargo_bin_cmd!("purra")
            .args(["--ascii-preset", "-vv", "--color", color])
            .write_stdin("…")
            .assert()
            .success()
            .stdout("...");
        let stderr = String::from_utf8_lossy(&assertion.get_output().stderr);
        assert_eq!(stderr.contains('\u{1b}'), styled);
    }
}

#[test]
fn help_documents_new_flags() {
    cargo_bin_cmd!("purra")
        .arg("--help")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("--no-backup")
                .and(predicate::str::contains("-q, --quiet"))
                .and(predicate::str::contains("-v, --verbose")),
        );
}

#[test]
fn backup_and_verbosity_controls_respect_directory_exclusions() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("input.txt");
    let ignored = directory.path().join("ignored.tmp");
    let explicit_skip = directory.path().join("invalid.skip");
    fs::write(directory.path().join(".gitignore"), "*.tmp\n").unwrap();
    fs::write(&input, "…").unwrap();
    fs::write(&ignored, "…").unwrap();
    fs::write(&explicit_skip, [0xff]).unwrap();

    cargo_bin_cmd!("purra")
        .args([
            "--ascii-preset",
            "--no-backup",
            "--quiet",
            "--dry-run",
            "--ignore",
            "*.skip",
            "--color",
            "never",
        ])
        .arg(directory.path())
        .assert()
        .code(1)
        .stdout(format!("{}:1:1: … -> ...\n", input.display()))
        .stderr("");
    cargo_bin_cmd!("purra")
        .args([
            "--ascii-preset", "--no-backup", "-fv", "--ignore", "*.skip",
            "--color", "never",
        ])
        .arg(directory.path())
        .assert()
        .success()
        .stdout("")
        .stderr(
            predicate::str::contains("2 processed input(s), 0 skipped input(s), 0 declined input(s); 1 replacement(s) in 1 input(s)")
                .and(predicate::str::contains("ignored.tmp").not())
                .and(predicate::str::contains("invalid.skip").not()),
        );

    assert_eq!(fs::read_to_string(input).unwrap(), "...");
    assert_eq!(fs::read_to_string(ignored).unwrap(), "…");
    assert_eq!(fs::read(explicit_skip).unwrap(), [0xff]);
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 4);
}
