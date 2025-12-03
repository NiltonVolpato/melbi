use nu_ansi_term::{Color, Style};
use reedline::StyledText;
use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent};
use miette::Result;

#[derive(Debug)]
struct PaletteItem<'a> {
    name: &'a str,
    fg: Color,
}

const PALETTE: &[PaletteItem] = &[
    PaletteItem { name: "", fg: Color::White },
    PaletteItem { name: "keyword", fg: Color::Magenta },
    PaletteItem { name: "operator", fg: Color::White },
    PaletteItem { name: "constant", fg: Color::Cyan },
    PaletteItem { name: "number", fg: Color::Cyan },
    PaletteItem { name: "string", fg: Color::Green },
    PaletteItem { name: "comment", fg: Color::DarkGray },
    PaletteItem { name: "function", fg: Color::Blue },
    PaletteItem { name: "type", fg: Color::Yellow },
    PaletteItem { name: "variable", fg: Color::Red },
    PaletteItem { name: "property", fg: Color::Red },
    PaletteItem { name: "punctuation", fg: Color::White },
    PaletteItem { name: "embedded", fg: Color::White },
];

const HIGHLIGHTS_QUERY: &str = include_str!("../../zed/languages/melbi/highlights.scm");

pub struct Highlighter {
    config: HighlightConfiguration,
}

impl Highlighter {
    pub fn new() -> Result<Self> {
        let highlight_names = PALETTE.iter().map(|item| item.name).collect::<Vec<_>>();
        
        let mut config = HighlightConfiguration::new(
            tree_sitter_melbi::LANGUAGE.into(),
            "melbi",
            HIGHLIGHTS_QUERY,
            "",
            "",
        )
        .map_err(|e| miette::miette!(e))?;
        config.configure(&highlight_names);
        Ok(Self { config })
    }
}

impl reedline::Highlighter for Highlighter {
    fn highlight(&self, line: &str, _: usize) -> StyledText {
        let mut output = StyledText::new();

        let mut highlighter = tree_sitter_highlight::Highlighter::new();
        let Ok(highlights) = highlighter.highlight(&self.config, line.as_bytes(), None, |_| None)
        else {
            let style = Style::new().fg(PALETTE[0].fg);
            output.push((style, line.to_string()));
            return output;
        };

        let mut curr_fg = PALETTE[0].fg;
        let mut curr_end = 0;

        for event in highlights {
            match event {
                Ok(HighlightEvent::HighlightStart(highlight)) => {
                    if let Some(item) = PALETTE.get(highlight.0) {
                     curr_fg = item.fg;
                    }
                }
                Ok(HighlightEvent::Source { start, end }) => {
                    let style = Style::new().fg(curr_fg);
                    let text = line[start..end].to_string();
                    output.push((style, text));
                    curr_end = end;
                }
                Ok(HighlightEvent::HighlightEnd) => {
                    curr_fg = PALETTE[0].fg;
                }
                Err(_) => {
                    let style = Style::new().fg(PALETTE[0].fg);
                    let text = line.get(curr_end..).unwrap_or_default().to_string();
                    output.push((style, text));
                    break;
                }
            }
        }

        output
    }
}
