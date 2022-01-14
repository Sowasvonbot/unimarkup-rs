use unimarkup_rs::frontend::parser;
use unimarkup_rs::middleend::ContentIrLine;
use unimarkup_rs::um_elements::types;
use unimarkup_rs::um_error::UmError;

use super::umblock_tests::*;

#[test]
fn valid_paragraph_with_heading() -> Result<(), UmError> {
    //paragraph1.um
    let um_blocks =
        parser::parse_unimarkup(&mut get_config("tests/test_files/frontend/paragraph1.um"))?;
    loop_through_ir_lines(&um_blocks, paragraph1_expected_result());

    Ok(())
}
#[test]
fn valid_paragraph_with_multi_line_heading() -> Result<(), UmError> {
    //paragraph2.um
    let um_blocks =
        parser::parse_unimarkup(&mut get_config("tests/test_files/frontend/paragraph2.um"))?;
    loop_through_ir_lines(&um_blocks, paragraph2_expected_result());

    Ok(())
}
#[test]
fn valid_paragraphs_with_sub_heading() -> Result<(), UmError> {
    //paragraph3.um
    let um_blocks =
        parser::parse_unimarkup(&mut get_config("tests/test_files/frontend/paragraph3.um"))?;
    loop_through_ir_lines(&um_blocks, paragraph3_expected_result());

    Ok(())
}

pub fn paragraph1_expected_result() -> Vec<ContentIrLine> {
    let mut blocks_vector: Vec<ContentIrLine> = Vec::new();
    blocks_vector.push(ContentIrLine::new(
        "head1",
        1,
        format!("heading{delim}level{delim}1", delim = types::DELIMITER),
        "head1",
        "",
        "{}",
        "",
    ));

    blocks_vector.push(ContentIrLine::new(
        "paragraph-3",
        3,
        "paragraph",
        "paragraph 1",
        "",
        "{}",
        "",
    ));

    blocks_vector.reverse();
    blocks_vector
}

pub fn paragraph2_expected_result() -> Vec<ContentIrLine> {
    let mut blocks_vector: Vec<ContentIrLine> = Vec::new();
    blocks_vector.push(ContentIrLine::new(
        "multi-line-header",
        1,
        format!("heading{delim}level{delim}1", delim = types::DELIMITER),
        "multi\nline header",
        "",
        "{}",
        "",
    ));

    blocks_vector.push(ContentIrLine::new(
        "paragraph-4",
        4,
        "paragraph",
        "paragraph",
        "",
        "{}",
        "",
    ));

    blocks_vector.reverse();
    blocks_vector
}

pub fn paragraph3_expected_result() -> Vec<ContentIrLine> {
    let mut blocks_vector: Vec<ContentIrLine> = Vec::new();
    blocks_vector.push(ContentIrLine::new(
        "head2",
        1,
        format!("heading{delim}level{delim}1", delim = types::DELIMITER),
        "head2",
        "",
        "{}",
        "",
    ));

    blocks_vector.push(ContentIrLine::new(
        "paragraph-3",
        3,
        "paragraph",
        "paragraph1",
        "",
        "{}",
        "",
    ));

    blocks_vector.push(ContentIrLine::new(
        "subhead2",
        5,
        format!("heading{delim}level{delim}2", delim = types::DELIMITER),
        "subhead2",
        "",
        "{}",
        "",
    ));

    blocks_vector.push(ContentIrLine::new(
        "paragraph-7",
        7,
        "paragraph",
        "paragraph2",
        "",
        "{}",
        "",
    ));

    blocks_vector.reverse();
    blocks_vector
}
