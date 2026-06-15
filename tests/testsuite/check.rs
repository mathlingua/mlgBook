//! Tests for the `mdbook check` command.

use crate::prelude::*;
use std::env;

#[test]
fn check_runs_mlg_check_on_mlg_blocks() {
    let mut test = BookTest::init(|_| {});
    test.change_file(
        "src/chapter_1.md",
        r#"```mlg
Defines: x is \set
```

```mlg,ignore
Defines: ignored is \set
```

```mlg-view
Text: $x$
```
"#,
    )
    .rust_program(
        "bin/mlg",
        r#"
use std::env;
use std::fs;
use std::io::Write;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let log = env::var("MLG_FAKE_LOG").expect("MLG_FAKE_LOG should be set");
    let cwd = env::current_dir().expect("current_dir should be available");
    let source = fs::read_to_string(cwd.join(&args[1])).expect("source should be readable");
    let mut output = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log)
        .expect("log should be writable");
    writeln!(output, "{}", args.join(" ")).unwrap();
    write!(output, "{source}").unwrap();
}
"#,
    );

    let log = test.dir.join("mlg-log.txt");
    let path = fake_mlg_path(&test);
    test.run("check", |cmd| {
        cmd.env("PATH", path.clone())
            .env("MLG_FAKE_LOG", log.to_string_lossy())
            .expect_stdout(str![[""]])
            .expect_stderr(str![[r#"
 INFO Checking 1 MathLingua block(s) in chapter 'Chapter 1': "chapter_1.md"

"#]]);
    });

    let log = read_to_string(log);
    test.assert.eq(
        log,
        str![[r#"
check block.mlg
Defines: x is /set

"#]],
    );
}

fn fake_mlg_path(test: &BookTest) -> String {
    let existing_path = env::var_os("PATH").unwrap_or_default();
    let paths = std::iter::once(test.dir.join("bin")).chain(env::split_paths(&existing_path));
    env::join_paths(paths)
        .unwrap()
        .to_string_lossy()
        .into_owned()
}
