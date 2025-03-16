use expect_test::expect_file;
use std::path::Path;

use snafu::{OptionExt, ResultExt};
type Result<T, E = snafu::Whatever> = std::result::Result<T, E>;

#[test]
#[snafu::report]
fn gen() -> Result<()> {
    let mut tests = Vec::with_capacity(24);

    // Find all test rs files
    for res in walkdir::WalkDir::new("tests") {
        let entry = res.with_whatever_context(|_| "Failed to get an entry from tests dir")?;
        if entry.file_type().is_file() {
            let path = entry.path();
            if path.extension() == Some("rs".as_ref()) {
                // Uniform path to linux style
                let s = path
                    .to_str()
                    .with_whatever_context(|| format!("{path:?} fails to be a &str"))?
                    .replace("\\", "/");
                tests.push(s);
            }
        }
    }
    tests.sort();

    expect_file!["tests.txt"].assert_debug_eq(&tests);

    dbg!(&tests);

    let mut outputs = Vec::new();

    // Analyze each test based on ullbc
    for test in &tests {
        let stem = Path::new(test).file_stem().unwrap().to_str().unwrap();
        println!("gen_ullbc for {test}");
        gen_ullbc(test, stem)?;
        println!("analyze {test}");
        let output = analyze(stem)?;
        println!(
            "analyze {test} output (empty={}):\n{output}\n",
            output.is_empty()
        );

        // Skip empty analysis
        let out_file = format!("{stem}.out");
        if !output.is_empty() {
            expect_file![out_file].assert_eq(&output);
            outputs.push((test, output));
        } else if std::fs::exists(Path::new("diagnostics").join(&out_file)).unwrap() {
            // detect if unexpected out file exists for empty analysis
            snafu::whatever!("{out_file} shouldn't exist, because {test} is analyzed as ok.");
        }
    }

    println!("\n\n\n");
    for (test, output) in &outputs {
        let content = std::fs::read_to_string(test)
            .with_whatever_context(|_| format!("Failed to read the content of {test}"))?;
        let meta = parse_meta(&content)?;
        println!("\x1B[42m\x1B[30m{test} | {meta:?}:\x1B[0m\n{output}");
    }

    Ok(())
}

fn ullbc_file(file_stem: &str) -> String {
    format!("diagnostics/{file_stem}.ullbc")
}

// Generate insertion_sort.ullbc
// e.g. charon --ullbc --no-merge-goto-chains --no-cargo --input tests/panic_safety/insertion_sort.rs
fn gen_ullbc(file: &str, file_stem: &str) -> Result<()> {
    duct::cmd!(
        "charon",
        "--ullbc",
        "--no-merge-goto-chains",
        "--no-cargo",
        "--input",
        file,
        "--dest-file",
        ullbc_file(file_stem)
    )
    .run()
    .with_whatever_context(|_| {
        let ullbc = ullbc_file(file_stem);
        format!(
            "Failed to run `\
             charon --ullbc --no-merge-goto-chains --no-cargo --input {file} --dest-file {ullbc}\
        `"
        )
    })?;
    Ok(())
}

// Analyze with rudra
// e.g. cargo-charon-rudra --file insertion_sort.ullbc
fn analyze(file_stem: &str) -> Result<String> {
    duct::cmd!("cargo-charon-rudra", "--file", ullbc_file(file_stem))
        .stdout_stderr_swap()
        .read()
        .with_whatever_context(|_| {
            let ullbc = ullbc_file(file_stem);
            format!("Failed to run `cargo-charon-rudra --file {ullbc}`")
        })
}

// /*!
// ```rudra-test
// test_type = "normal"
// expected_analyzers = ["SendSyncVariance"]
// ```
// !*/
fn parse_meta(content: &str) -> Result<Meta> {
    const HEAD: &str = "rudra-test";
    const TAIL: &str = "```\n";

    let pos = |s: &str| {
        content
            .find(s)
            .with_whatever_context(|| format!("{s:?} is not found in {content:?}"))
    };
    let pos_head = pos(HEAD)? + HEAD.len();
    let pos_tail = pos(TAIL)?;
    toml::from_str(&content[pos_head..pos_tail])
        .with_whatever_context(|_| "Failed to parse as Meta from the test")
}

#[allow(dead_code)]
#[derive(Debug, serde::Deserialize)]
struct Meta {
    test_type: String,
    expected_analyzers: Vec<String>,
}
