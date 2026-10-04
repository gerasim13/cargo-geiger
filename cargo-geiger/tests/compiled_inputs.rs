use assert_cmd::prelude::*;
use cargo_geiger_serde::SafetyReport;
use std::{fs, process::Command};

#[test]
fn generated_rust_is_counted_and_resource_inputs_are_not_rust() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"generated-input-proof\"\nversion = \"0.1.0\"\nedition = \"2024\"\n[workspace]\n").unwrap();
    fs::write(dir.path().join("build.rs"), r#"fn main() {
        std::fs::write(std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("generated.rs"), "pub unsafe fn generated() {}\n").unwrap();
    }"#).unwrap();
    fs::write(
        dir.path().join("src/lib.rs"),
        r#"include!(concat!(env!("OUT_DIR"), "/generated.rs"));
        pub const DOC: &str = include_str!("../README.md");
        pub const DATA: &[u8] = include_bytes!("../sample.data");"#,
    )
    .unwrap();
    fs::write(dir.path().join("README.md"), "Documentation, not Rust.")
        .unwrap();
    fs::write(dir.path().join("sample.data"), [0xff, 0x00]).unwrap();
    fs::write(
        dir.path().join("src/uncompiled.rs"),
        "intentionally invalid UI fixture",
    )
    .unwrap();
    for format in ["Ascii", "Json"] {
        let output = Command::cargo_bin("cargo-geiger")
            .unwrap()
            .args(["geiger", "--color=never", "--output-format", format])
            .arg("--manifest-path")
            .arg(dir.path().join("Cargo.toml"))
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!String::from_utf8_lossy(&output.stderr)
            .contains("Failed to parse"));
        if format == "Json" {
            let report: SafetyReport =
                serde_json::from_slice(&output.stdout).unwrap();
            assert!(report.used_but_not_scanned_files.is_empty(), "{:?}", report);
            assert!(report.packages_without_metrics.is_empty(), "{:?}", report);
            assert_eq!(
                report
                    .packages
                    .values()
                    .next()
                    .unwrap()
                    .unsafety
                    .used
                    .functions
                    .unsafe_,
                1
            );
        }
    }
}
