use expect_test::expect_file;

#[test]
fn gen() {
    let mut tests = Vec::with_capacity(24);

    // Find all test rs files
    for res in walkdir::WalkDir::new("tests") {
        let entry = res.unwrap();
        if entry.file_type().is_file() {
            let path = entry.path();
            if path.extension() == Some("rs".as_ref()) {
                // Uniform path to linux style
                let s = entry.into_path().to_str().unwrap().replace("\\", "/");
                tests.push(s);
            }
        }
    }

    expect_file!["tests.txt"].assert_debug_eq(&tests);

    dbg!(tests);
}
