use std::fs;

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
