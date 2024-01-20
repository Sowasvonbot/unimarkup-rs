use crate::html::citeproc::CiteprocWrapper;
use crate::log_id::RenderError;
use serde_json::Value;
use unimarkup_commons::lexer::{span::Span, symbol::SymbolKind, token::TokenKind};
use unimarkup_inline::element::substitution::DistinctReference;
use unimarkup_inline::element::{
    base::{EscapedNewline, EscapedPlain, EscapedWhitespace, Newline, Plain},
    formatting::{
        Bold, Highlight, Italic, Math, Overline, Quote, Strikethrough, Subscript, Superscript,
        Underline, Verbatim,
    },
    textbox::{citation::Citation, hyperlink::Hyperlink, TextBox},
    InlineElement,
};
use unimarkup_parser::elements::indents::{BulletList, BulletListEntry};

use crate::render::{Context, OutputFormat, Renderer};

use super::{
    highlight, tag::HtmlTag, Html, HtmlAttribute, HtmlAttributes, HtmlBody, HtmlElement, HtmlHead,
};

#[derive(Debug, Default)]
pub struct HtmlRenderer {
    citation_index: usize,
}

impl Renderer<Html> for HtmlRenderer {
    fn render_paragraph(
        &mut self,
        paragraph: &unimarkup_parser::elements::atomic::Paragraph,
        context: &Context,
    ) -> Result<Html, crate::log_id::RenderError> {
        let inner = self.render_inlines(&paragraph.content, context)?;

        Ok(Html::nested(HtmlTag::P, HtmlAttributes::default(), inner))
    }

    fn render_heading(
        &mut self,
        heading: &unimarkup_parser::elements::atomic::Heading,
        context: &Context,
    ) -> Result<Html, crate::log_id::RenderError> {
        let inner = self.render_inlines(&heading.content, context)?;
        let tag = HtmlTag::from(heading.level);

        let attributes = HtmlAttributes::from(vec![HtmlAttribute {
            name: "id".to_string(),
            value: Some(heading.id.clone()),
        }]);

        Ok(Html::nested(tag, attributes, inner))
    }

    fn render_verbatim_block(
        &mut self,
        verbatim: &unimarkup_parser::elements::enclosed::VerbatimBlock,
        _context: &Context,
    ) -> Result<Html, crate::log_id::RenderError> {
        let inner = Html::with(
            HtmlHead {
                syntax_highlighting_used: true,
                ..Default::default()
            },
            HtmlBody::from(HtmlElement {
                tag: HtmlTag::Code,
                attributes: HtmlAttributes::default(),
                content: Some(
                    highlight::highlight_content(
                        &verbatim.content,
                        verbatim
                            .data_lang
                            .as_ref()
                            .unwrap_or(&highlight::PLAIN_SYNTAX.to_string()),
                    )
                    .unwrap_or(verbatim.content.clone()),
                ),
            }),
        );

        Ok(Html::nested(HtmlTag::Pre, HtmlAttributes::default(), inner))
    }

    fn render_bullet_list(
        &mut self,
        bullet_list: &BulletList,
        context: &Context,
    ) -> Result<Html, crate::log_id::RenderError> {
        let mut entries = Html::new(context);

        for entry in &bullet_list.entries {
            entries.append(self.render_bullet_list_entry(entry, context)?)?;
        }

        Ok(Html::nested(
            HtmlTag::Ul,
            HtmlAttributes::default(),
            entries,
        ))
    }

    fn render_bullet_list_entry(
        &mut self,
        bullet_list_entry: &BulletListEntry,
        context: &Context,
    ) -> Result<Html, crate::log_id::RenderError> {
        let mut entry_heading = self.render_inlines(&bullet_list_entry.heading, context)?;

        if !bullet_list_entry.body.is_empty() {
            entry_heading = Html::nested(HtmlTag::P, HtmlAttributes::default(), entry_heading);
            entry_heading.append(self.render_blocks(&bullet_list_entry.body, context)?)?;
        }

        Ok(Html::nested(
            HtmlTag::Li,
            HtmlAttributes::default(),
            entry_heading,
        ))
    }

    fn render_blankline(
        &mut self,
        _blankline: &Span,
        _context: &Context,
    ) -> Result<Html, crate::log_id::RenderError> {
        let html = Html::with_body(HtmlBody::from(HtmlElement {
            tag: HtmlTag::Span,
            attributes: HtmlAttributes(vec![HtmlAttribute {
                name: "style".to_string(),
                value: Some("white-space: pre-wrap;".to_string()),
            }]),
            content: Some(String::from(TokenKind::Blankline)),
        }));

        Ok(html)
    }

    fn render_textbox(
        &mut self,
        textbox: &TextBox,
        context: &Context,
    ) -> Result<Html, crate::log_id::RenderError> {
        let inner = self.render_nested_inline(textbox.inner(), context)?;

        Ok(Html::nested(
            HtmlTag::Span,
            HtmlAttributes::default(),
            inner,
        ))
    }

    fn render_hyperlink(
        &mut self,
        hyperlink: &Hyperlink,
        context: &Context,
    ) -> Result<Html, crate::log_id::RenderError> {
        let inner = self.render_nested_inline(hyperlink.inner(), context)?;
        let mut attributes = vec![HtmlAttribute {
            name: "href".to_string(),
            value: Some(hyperlink.link().to_string()),
        }];

        if let Some(link_text) = hyperlink.link_text() {
            attributes.push(HtmlAttribute {
                name: "title".to_string(),
                value: Some(link_text.to_string()),
            })
        }

        Ok(Html::nested(HtmlTag::A, HtmlAttributes(attributes), inner))
    }

    fn render_citation(
        &mut self,
        _citation: &Citation,
        context: &Context,
    ) -> Result<Html, crate::log_id::RenderError> {
        let citation = context
            .rendered_citation(self.citation_index)
            .expect("Rendered citation must exist for parsed citation.");
        self.citation_index += 1;

        let html = Html::with_body(HtmlBody::from(HtmlElement {
            tag: HtmlTag::PlainContent,
            attributes: HtmlAttributes::default(),
            content: Some(citation.clone()),
        }));

        Ok(html)
    }

    fn render_distinct_reference(
        &mut self,
        distinct_reference: &DistinctReference,
        context: &Context,
    ) -> Result<Html, RenderError> {
        let mut selected_item: Option<Value> = None;
        for csl_item in context.csl_data.clone().items {
            if csl_item.id.to_string() == distinct_reference.id() {
                selected_item = Some(serde_json::to_value(csl_item).unwrap());
                break;
            }
        }
        let content;
        if let Some(item_value) = selected_item {
            let mut result_value = item_value.clone();
            if distinct_reference.fields().len() == 1 && distinct_reference.fields()[0] == "authors"
            {
                content = match CiteprocWrapper::new() {
                    Ok(mut citeproc) => citeproc
                        .get_author_only(context.doc, distinct_reference.id().to_string())
                        .unwrap_or("########### CITATION ERROR ###########".to_string()),
                    Err(_) => "########### CITATION ERROR ###########".to_string(),
                }
            } else {
                for field in distinct_reference.fields().clone() {
                    result_value = match field.parse::<usize>() {
                        Ok(n) => result_value[n].clone(),
                        Err(_) => result_value[field].clone(),
                    };
                }
                content = if let Some(s) = result_value.as_str() {
                    s.to_string()
                } else {
                    let content_as_string = result_value.to_string();
                    if content_as_string.ends_with(".0") {
                        match content_as_string[..content_as_string.len() - 2].parse::<usize>() {
                            Ok(n) => n.to_string(),
                            Err(_) => content_as_string,
                        }
                    } else if content_as_string == "null" {
                        "########### CITATION ERROR ###########".to_string()
                    } else {
                        content_as_string
                    }
                }
            }
        } else {
            content = "########### CITATION ERROR ###########".to_string()
        }
        let html = Html::with_body(HtmlBody::from(HtmlElement {
            tag: HtmlTag::PlainContent,
            attributes: HtmlAttributes::default(),
            content: Some(content),
        }));
        Ok(html)
    }

    fn render_bibliography(
        &mut self,
        context: &Context,
    ) -> Result<Html, crate::log_id::RenderError> {
        let mut elements: Vec<HtmlElement> = vec![];
        let bibliography_string = if context.get_lang().id.language
            == unimarkup_commons::config::icu_locid::subtags::language!("de")
        {
            "Literaturverzeichnis"
        } else {
            "Bibliography"
        };
        elements.push(HtmlElement {
            tag: HtmlTag::H1,
            attributes: HtmlAttributes::default(),
            content: Some(bibliography_string.to_string()),
        });
        elements.push(HtmlElement {
            tag: HtmlTag::PlainContent,
            attributes: HtmlAttributes::default(),
            content: Some(context.bibliography.clone()),
        });
        let body = HtmlBody::from(elements);
        let html = Html::with_body(body);
        Ok(html)
    }

    fn render_footnotes(&mut self, context: &Context) -> Result<Html, RenderError> {
        let footnotes = context.footnotes.clone();
        if !footnotes.is_empty() {
            let elements: Vec<HtmlElement> = vec![
                HtmlElement {
                    tag: HtmlTag::PlainContent,
                    attributes: HtmlAttributes::default(),
                    content: Some("<hr style=\"width: 25%; margin-left: 0\">".to_string()),
                },
                HtmlElement {
                    tag: HtmlTag::PlainContent,
                    attributes: HtmlAttributes::default(),
                    content: Some(footnotes),
                },
            ];
            let body = HtmlBody::from(elements);
            let html = Html::with_body(body);
            Ok(html)
        } else {
            Ok(Html::default())
        }
    }

    fn render_bold(
        &mut self,
        bold: &Bold,
        context: &Context,
    ) -> Result<Html, crate::log_id::RenderError> {
        let inner = self.render_nested_inline(bold.inner(), context)?;

        Ok(Html::nested(
            HtmlTag::Strong,
            HtmlAttributes::default(),
            inner,
        ))
    }

    fn render_italic(
        &mut self,
        italic: &Italic,
        context: &Context,
    ) -> Result<Html, crate::log_id::RenderError> {
        let inner = self.render_nested_inline(italic.inner(), context)?;

        Ok(Html::nested(HtmlTag::Em, HtmlAttributes::default(), inner))
    }

    fn render_underline(
        &mut self,
        underline: &Underline,
        context: &Context,
    ) -> Result<Html, crate::log_id::RenderError> {
        let inner = self.render_nested_inline(underline.inner(), context)?;
        let mut attributes = HtmlAttributes::default();
        attributes.push(HtmlAttribute {
            name: "style".to_string(),
            value: Some("text-decoration: underline;".to_string()),
        });

        Ok(Html::nested(HtmlTag::Span, attributes, inner))
    }

    fn render_subscript(
        &mut self,
        subscript: &Subscript,
        context: &Context,
    ) -> Result<Html, crate::log_id::RenderError> {
        let inner = self.render_nested_inline(subscript.inner(), context)?;

        Ok(Html::nested(HtmlTag::Sub, HtmlAttributes::default(), inner))
    }

    fn render_superscript(
        &mut self,
        superscript: &Superscript,
        context: &Context,
    ) -> Result<Html, crate::log_id::RenderError> {
        let inner = self.render_nested_inline(superscript.inner(), context)?;

        Ok(Html::nested(HtmlTag::Sup, HtmlAttributes::default(), inner))
    }

    fn render_overline(
        &mut self,
        overline: &Overline,
        context: &Context,
    ) -> Result<Html, crate::log_id::RenderError> {
        let inner = self.render_nested_inline(overline.inner(), context)?;
        let mut attributes = HtmlAttributes::default();
        attributes.push(HtmlAttribute {
            name: "style".to_string(),
            value: Some("text-decoration: overline;".to_string()),
        });

        Ok(Html::nested(HtmlTag::Span, attributes, inner))
    }

    fn render_strikethrough(
        &mut self,
        strikethrough: &Strikethrough,
        context: &Context,
    ) -> Result<Html, crate::log_id::RenderError> {
        let inner = self.render_nested_inline(strikethrough.inner(), context)?;
        let mut attributes = HtmlAttributes::default();
        attributes.push(HtmlAttribute {
            name: "style".to_string(),
            value: Some("text-decoration: line-through;".to_string()),
        });

        Ok(Html::nested(HtmlTag::Span, attributes, inner))
    }

    fn render_highlight(
        &mut self,
        highlight: &Highlight,
        context: &Context,
    ) -> Result<Html, crate::log_id::RenderError> {
        let inner = self.render_nested_inline(highlight.inner(), context)?;

        Ok(Html::nested(
            HtmlTag::Mark,
            HtmlAttributes::default(),
            inner,
        ))
    }

    fn render_quote(
        &mut self,
        quote: &Quote,
        context: &Context,
    ) -> Result<Html, crate::log_id::RenderError> {
        let inner = self.render_nested_inline(quote.inner(), context)?;

        Ok(Html::nested(HtmlTag::Q, HtmlAttributes::default(), inner))
    }

    fn render_inline_verbatim(
        &mut self,
        verbatim: &Verbatim,
        _context: &Context,
    ) -> Result<Html, crate::log_id::RenderError> {
        let html = Html::with_body(HtmlBody::from(HtmlElement {
            tag: HtmlTag::Code,
            attributes: HtmlAttributes::default(),
            content: Some(verbatim.inner().as_unimarkup()),
        }));

        Ok(html)
    }

    fn render_inline_math(
        &mut self,
        math: &Math,
        context: &Context,
    ) -> Result<Html, crate::log_id::RenderError> {
        // TODO: use proper math rendering once supported
        let inner = self.render_nested_inline(math.inner(), context)?;

        Ok(Html::nested(
            HtmlTag::Span,
            HtmlAttributes::default(),
            inner,
        ))
    }

    fn render_plain(
        &mut self,
        plain: &Plain,
        _context: &Context,
    ) -> Result<Html, crate::log_id::RenderError> {
        let html = Html::with_body(HtmlBody::from(HtmlElement {
            tag: HtmlTag::PlainContent,
            attributes: HtmlAttributes::default(),
            content: Some(plain.content().clone()),
        }));

        Ok(html)
    }

    fn render_newline(
        &mut self,
        _newline: &Newline,
        _context: &Context,
    ) -> Result<Html, crate::log_id::RenderError> {
        let html = Html::with_body(HtmlBody::from(HtmlElement {
            tag: HtmlTag::PlainContent,
            attributes: HtmlAttributes::default(),
            content: Some(SymbolKind::Whitespace.as_str().to_string()),
        }));

        Ok(html)
    }

    fn render_implicit_newline(
        &mut self,
        _implicit_newline: &Newline,
        _context: &Context,
    ) -> Result<Html, crate::log_id::RenderError> {
        let html = Html::with_body(HtmlBody::from(HtmlElement {
            tag: HtmlTag::Br,
            attributes: HtmlAttributes::default(),
            content: None,
        }));

        Ok(html)
    }

    fn render_escaped_newline(
        &mut self,
        _escaped_newline: &EscapedNewline,
        _context: &Context,
    ) -> Result<Html, crate::log_id::RenderError> {
        let html = Html::with_body(HtmlBody::from(HtmlElement {
            tag: HtmlTag::Br,
            attributes: HtmlAttributes::default(),
            content: None,
        }));

        Ok(html)
    }

    fn render_escaped_whitespace(
        &mut self,
        _escaped_whitespace: &EscapedWhitespace,
        _context: &Context,
    ) -> Result<Html, crate::log_id::RenderError> {
        let html = Html::with_body(HtmlBody::from(HtmlElement {
            tag: HtmlTag::Span,
            attributes: HtmlAttributes(vec![HtmlAttribute {
                name: "style".to_string(),
                value: Some("white-space: pre-wrap;".to_string()),
            }]),
            content: Some(SymbolKind::Whitespace.as_str().to_string()),
        }));

        Ok(html)
    }

    fn render_escaped_plain(
        &mut self,
        escaped_plain: &EscapedPlain,
        _context: &Context,
    ) -> Result<Html, crate::log_id::RenderError> {
        let html = Html::with_body(HtmlBody::from(HtmlElement {
            tag: HtmlTag::PlainContent,
            attributes: HtmlAttributes::default(),
            content: Some(escaped_plain.content().clone()),
        }));

        Ok(html)
    }
}
