use clap::StructOpt;
use unimarkup_core::{
    backend::{self, BackendError, Render},
    config::Config,
    elements::{HeadingBlock, HeadingLevel},
    error::UmError,
    middleend::{self, AsIrLines, ContentIrLine},
};

use super::super::middleend::ir_test_setup;

#[test]
fn run() -> Result<(), UmError> {
    let mut connection = ir_test_setup::setup_test_ir();

    let block = HeadingBlock {
        id: "some-id".into(),
        level: HeadingLevel::Level1,
        content: "This is a heading".into(),
        attributes: "{}".into(),
        line_nr: 0,
    };

    let lines: Vec<ContentIrLine> = block.as_ir_lines();

    {
        let transaction = ir_test_setup::get_test_transaction(&mut connection);
        middleend::write_ir_lines(&lines, &transaction)?;

        transaction.commit().unwrap();
    }

    let cfg: Config = Config::parse_from(vec!["unimarkup", "--output-formats=html", "in_file.um"]);

    #[allow(clippy::redundant_clone)]
    let mut out_path = cfg.um_file.clone();
    out_path.set_extension("html");

    backend::run(&mut connection, &cfg)?;

    let output = std::fs::read_to_string(&out_path);

    match output {
        Ok(content) => {
            assert_eq!(block.render_html().expect("Block is checked"), content);
        }
        _ => {
            return Err(BackendError::new(format!(
                "Could not write file to {}",
                out_path.to_str().unwrap()
            ))
            .into())
        }
    }

    assert!(out_path.exists());

    if out_path.exists() {
        let _ = std::fs::remove_file(out_path);
    }

    Ok(())
}
