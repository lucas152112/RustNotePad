use thiserror::Error;

/// Tokens recognised by the header/footer parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateToken {
    FileName,
    FilePath,
    PageNumber,
    PageCount,
    Date,
    Time,
    Encoding,
}

/// Template segments per alignment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateSegment {
    Literal(String),
    Token(TemplateToken),
}

/// Parsed representation of a header/footer template.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HeaderFooterTemplate {
    pub left: Vec<TemplateSegment>,
    pub center: Vec<TemplateSegment>,
    pub right: Vec<TemplateSegment>,
}

#[derive(Clone, Copy)]
enum Alignment {
    Left,
    Center,
    Right,
}

impl HeaderFooterTemplate {
    pub fn parse(input: &str) -> Result<Self, TemplateError> {
        let mut alignment = Alignment::Left;
        let mut left = Vec::new();
        let mut center = Vec::new();
        let mut right = Vec::new();
        let mut buffer = String::new();

        let mut chars = input.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch != '&' {
                buffer.push(ch);
                continue;
            }

            let Some(next) = chars.peek().copied() else {
                buffer.push('&');
                break;
            };

            match next {
                '&' => {
                    buffer.push('&');
                    chars.next();
                }
                'l' | 'L' => {
                    flush_buffer(&mut buffer, alignment, &mut left, &mut center, &mut right);
                    alignment = Alignment::Left;
                    chars.next();
                }
                'c' | 'C' => {
                    flush_buffer(&mut buffer, alignment, &mut left, &mut center, &mut right);
                    alignment = Alignment::Center;
                    chars.next();
                }
                'r' | 'R' => {
                    flush_buffer(&mut buffer, alignment, &mut left, &mut center, &mut right);
                    alignment = Alignment::Right;
                    chars.next();
                }
                'f' => {
                    flush_buffer(&mut buffer, alignment, &mut left, &mut center, &mut right);
                    push_segment(
                        TemplateSegment::Token(TemplateToken::FileName),
                        alignment,
                        &mut left,
                        &mut center,
                        &mut right,
                    );
                    chars.next();
                }
                'F' => {
                    flush_buffer(&mut buffer, alignment, &mut left, &mut center, &mut right);
                    push_segment(
                        TemplateSegment::Token(TemplateToken::FilePath),
                        alignment,
                        &mut left,
                        &mut center,
                        &mut right,
                    );
                    chars.next();
                }
                'p' => {
                    flush_buffer(&mut buffer, alignment, &mut left, &mut center, &mut right);
                    push_segment(
                        TemplateSegment::Token(TemplateToken::PageNumber),
                        alignment,
                        &mut left,
                        &mut center,
                        &mut right,
                    );
                    chars.next();
                }
                'P' => {
                    flush_buffer(&mut buffer, alignment, &mut left, &mut center, &mut right);
                    push_segment(
                        TemplateSegment::Token(TemplateToken::PageCount),
                        alignment,
                        &mut left,
                        &mut center,
                        &mut right,
                    );
                    chars.next();
                }
                'd' | 'D' => {
                    flush_buffer(&mut buffer, alignment, &mut left, &mut center, &mut right);
                    push_segment(
                        TemplateSegment::Token(TemplateToken::Date),
                        alignment,
                        &mut left,
                        &mut center,
                        &mut right,
                    );
                    chars.next();
                }
                't' | 'T' => {
                    flush_buffer(&mut buffer, alignment, &mut left, &mut center, &mut right);
                    push_segment(
                        TemplateSegment::Token(TemplateToken::Time),
                        alignment,
                        &mut left,
                        &mut center,
                        &mut right,
                    );
                    chars.next();
                }
                'o' | 'O' => {
                    flush_buffer(&mut buffer, alignment, &mut left, &mut center, &mut right);
                    push_segment(
                        TemplateSegment::Token(TemplateToken::Encoding),
                        alignment,
                        &mut left,
                        &mut center,
                        &mut right,
                    );
                    chars.next();
                }
                other => return Err(TemplateError::UnknownToken(other)),
            }
        }

        flush_buffer(&mut buffer, alignment, &mut left, &mut center, &mut right);

        Ok(Self {
            left,
            center,
            right,
        })
    }

    pub fn render(&self, context: &HeaderFooterContext<'_>) -> RenderedHeaderFooter {
        RenderedHeaderFooter {
            left: render_segments(&self.left, context),
            center: render_segments(&self.center, context),
            right: render_segments(&self.right, context),
        }
    }
}

fn flush_buffer(
    buffer: &mut String,
    alignment: Alignment,
    left: &mut Vec<TemplateSegment>,
    center: &mut Vec<TemplateSegment>,
    right: &mut Vec<TemplateSegment>,
) {
    if buffer.is_empty() {
        return;
    }
    let literal = TemplateSegment::Literal(buffer.clone());
    push_segment(literal, alignment, left, center, right);
    buffer.clear();
}

fn push_segment(
    segment: TemplateSegment,
    alignment: Alignment,
    left: &mut Vec<TemplateSegment>,
    center: &mut Vec<TemplateSegment>,
    right: &mut Vec<TemplateSegment>,
) {
    match alignment {
        Alignment::Left => left.push(segment),
        Alignment::Center => center.push(segment),
        Alignment::Right => right.push(segment),
    }
}

fn render_segments(segments: &[TemplateSegment], context: &HeaderFooterContext<'_>) -> String {
    let mut output = String::new();
    for segment in segments {
        match segment {
            TemplateSegment::Literal(text) => output.push_str(text),
            TemplateSegment::Token(token) => token.append_to(&mut output, context),
        }
    }
    output
}

/// Runtime context for header/footer rendering.
#[derive(Debug, Clone, Default)]
pub struct HeaderFooterContext<'a> {
    pub file_name: Option<&'a str>,
    pub file_path: Option<&'a str>,
    pub page_number: u32,
    pub page_count: Option<u32>,
    pub date: Option<&'a str>,
    pub time: Option<&'a str>,
    pub encoding: Option<&'a str>,
}

/// Rendered header/footer strings for each alignment slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedHeaderFooter {
    pub left: String,
    pub center: String,
    pub right: String,
}

/// Errors raised while parsing header/footer templates.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TemplateError {
    #[error("unknown header/footer token '&{0}'")]
    UnknownToken(char),
}

impl TemplateToken {
    fn append_to<'a>(&self, buffer: &mut String, context: &HeaderFooterContext<'a>) {
        match self {
            TemplateToken::FileName => {
                if let Some(value) = context.file_name {
                    buffer.push_str(value);
                }
            }
            TemplateToken::FilePath => {
                if let Some(value) = context.file_path {
                    buffer.push_str(value);
                }
            }
            TemplateToken::PageNumber => {
                use std::fmt::Write;
                let _ = write!(buffer, "{}", context.page_number);
            }
            TemplateToken::PageCount => {
                if let Some(total) = context.page_count {
                    use std::fmt::Write;
                    let _ = write!(buffer, "{}", total);
                }
            }
            TemplateToken::Date => {
                if let Some(value) = context.date {
                    buffer.push_str(value);
                }
            }
            TemplateToken::Time => {
                if let Some(value) = context.time {
                    buffer.push_str(value);
                }
            }
            TemplateToken::Encoding => {
                if let Some(value) = context.encoding {
                    buffer.push_str(value);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_alignment_and_tokens() {
        let template = HeaderFooterTemplate::parse("&lLeft &f&cCenter &p&rPage &P").unwrap();
        assert_eq!(
            template.left,
            vec![
                TemplateSegment::Literal("Left ".into()),
                TemplateSegment::Token(TemplateToken::FileName)
            ]
        );
        assert_eq!(
            template.center,
            vec![
                TemplateSegment::Literal("Center ".into()),
                TemplateSegment::Token(TemplateToken::PageNumber)
            ]
        );
        assert_eq!(
            template.right,
            vec![
                TemplateSegment::Literal("Page ".into()),
                TemplateSegment::Token(TemplateToken::PageCount)
            ]
        );
    }

    #[test]
    fn render_template() {
        let template = HeaderFooterTemplate::parse("&l&f - && &p&r&P").unwrap();
        let context = HeaderFooterContext {
            file_name: Some("main.rs"),
            file_path: Some("/tmp/main.rs"),
            page_number: 3,
            page_count: Some(10),
            date: Some("2024-03-01"),
            time: Some("20:20"),
            encoding: Some("UTF-8"),
        };
        let rendered = template.render(&context);
        assert_eq!(rendered.left, "main.rs - & 3");
        assert_eq!(rendered.right, "10");
        assert!(rendered.center.is_empty());
    }

    #[test]
    fn unknown_token() {
        match HeaderFooterTemplate::parse("&x") {
            Err(TemplateError::UnknownToken('x')) => {}
            other => panic!("unexpected result: {:?}", other),
        }
    }
}
