use super::command_prelude::*;
use crate::get_book_dir;
use anyhow::Result;
use mdbook_driver::MDBook;

/// Create clap subcommand arguments.
pub fn make_subcommand() -> Command {
    Command::new("check")
        .about("Checks a book's MathLingua code samples")
        .arg_root_dir()
        .arg(
            Arg::new("chapter")
                .short('c')
                .long("chapter")
                .value_name("chapter"),
        )
}

/// Check command implementation.
pub fn execute(args: &ArgMatches) -> Result<()> {
    let chapter: Option<&str> = args.get_one::<String>("chapter").map(|s| s.as_str());

    let book_dir = get_book_dir(args);
    let mut book = MDBook::load(book_dir)?;

    match chapter {
        Some(_) => book.check_chapter(chapter),
        None => book.check(),
    }?;

    Ok(())
}
