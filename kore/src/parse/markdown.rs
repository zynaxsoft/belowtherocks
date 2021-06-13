use pulldown_cmark::{html, Options, Parser};
use std::io::Write;

#[allow(unused_imports)]
use color_eyre::{eyre::Report, eyre::WrapErr, Result, Section};
#[allow(unused_imports)]
use tide::log::{debug, error, info, trace, warn};

fn options() -> Options {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options
}

pub fn parse_md<W: Write>(input: &str, output: W) -> Result<()> {
    let parser = Parser::new_ext(input, options());
    // let iter = parser.map(|event| {
    //     match &event {
    //         e => {println!("Event {:?}", e);},
    //     }
    //     event
    // });
    html::write_html(output, parser)?;
    Ok(())
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn parse_simple_md() {
        let markdown_input = "Hello world, this is a ~~complicated~~ *very simple* example.";
        let mut output = Vec::new();
        parse_md(markdown_input, &mut output).unwrap();
        let html = String::from_utf8_lossy(&output);
        let expected_html =
            "<p>Hello world, this is a <del>complicated</del> <em>very simple</em> example.</p>\n";
        assert_eq!(expected_html, html);
    }
}
