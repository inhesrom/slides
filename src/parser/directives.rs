use super::SpeakerNote;

#[derive(Debug, Clone)]
pub enum LayoutDirective {
    Split { ratios: Vec<f32> },
    Grid { cols: u32, rows: u32 },
    Stack,
}

/// Extract `:::notes ... :::` blocks, returning (notes, content_without_notes).
pub fn extract_notes(content: &str) -> (Vec<SpeakerNote>, String) {
    let mut notes = Vec::new();
    let mut cleaned = String::new();
    let mut in_notes = false;
    let mut note_buf = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == ":::notes" {
            in_notes = true;
            note_buf.clear();
        } else if in_notes && trimmed == ":::" {
            in_notes = false;
            notes.push(SpeakerNote {
                text: note_buf.trim().to_string(),
            });
        } else if in_notes {
            note_buf.push_str(line);
            note_buf.push('\n');
        } else {
            cleaned.push_str(line);
            cleaned.push('\n');
        }
    }

    (notes, cleaned)
}

/// Consume characters until the matching closing `]` is found.
fn collect_bracketed_text(chars: &mut impl Iterator<Item = char>) -> Option<String> {
    let mut text = String::new();
    let mut depth = 1;
    for c in chars {
        match c {
            '[' => {
                depth += 1;
                text.push(c);
            }
            ']' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                text.push(c);
            }
            _ => text.push(c),
        }
    }
    if text.is_empty() { None } else { Some(text) }
}

/// Extract inline `^[...]` speaker notes from a single line.
fn extract_inline_notes_from_line(line: &str) -> (String, Vec<SpeakerNote>) {
    let mut notes = Vec::new();
    let mut clean_line = String::new();
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '^' && chars.peek() == Some(&'[') {
            chars.next();
            if let Some(text) = collect_bracketed_text(&mut chars) {
                notes.push(SpeakerNote { text });
            }
        } else {
            clean_line.push(c);
        }
    }
    (clean_line, notes)
}

/// Extract all inline `^[...]` speaker notes, returning (notes, cleaned_content).
pub fn extract_inline_notes(content: &str) -> (Vec<SpeakerNote>, String) {
    let mut all_notes = Vec::new();
    let mut cleaned = String::new();

    for line in content.lines() {
        let (clean_line, notes) = extract_inline_notes_from_line(line);
        all_notes.extend(notes);
        cleaned.push_str(&clean_line);
        cleaned.push('\n');
    }

    (all_notes, cleaned)
}

/// Scan lines to find a layout directive block's boundaries and type.
fn find_layout_directive(lines: &[&str]) -> Option<(usize, usize, LayoutDirective)> {
    let mut start = None;
    let mut directive = None;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with(":::split") {
            directive = Some(parse_split(trimmed));
            start = Some(i);
        } else if trimmed.starts_with(":::grid") {
            directive = Some(parse_grid(trimmed));
            start = Some(i);
        } else if trimmed == ":::stack" {
            directive = Some(LayoutDirective::Stack);
            start = Some(i);
        } else if start.is_some() && trimmed == ":::" {
            return Some((start.unwrap(), i, directive.unwrap()));
        }
    }
    None
}

/// Extract layout directives (`:::split`, `:::grid`, `:::stack`) and their regions.
/// Returns (layout_directive, regions) where regions are the content split by `+++`.
pub fn extract_layout(content: &str) -> (Option<LayoutDirective>, Vec<String>) {
    let lines: Vec<&str> = content.lines().collect();

    let Some((start, end, directive)) = find_layout_directive(&lines) else {
        return (None, Vec::new());
    };

    let inner_text = lines[start + 1..end].join("\n");
    let regions: Vec<String> = inner_text
        .split("\n+++\n")
        .map(|s| s.trim().to_string())
        .collect();

    (Some(directive), regions)
}

/// Strip a directive prefix from a line and return the remaining args.
fn directive_args<'a>(line: &'a str, prefix: &str) -> &'a str {
    line.trim().strip_prefix(prefix).unwrap_or("").trim()
}

/// Parse a `:::split [ratio]` directive line.
fn parse_split(line: &str) -> LayoutDirective {
    let args = directive_args(line, ":::split");
    if args.is_empty() {
        return LayoutDirective::Split {
            ratios: vec![50.0, 50.0],
        };
    }

    let ratios: Vec<f32> = args
        .split('/')
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    if ratios.len() >= 2 {
        LayoutDirective::Split { ratios }
    } else {
        LayoutDirective::Split {
            ratios: vec![50.0, 50.0],
        }
    }
}

/// Parse a `:::grid [cols]x[rows]` directive line.
fn parse_grid(line: &str) -> LayoutDirective {
    let args = directive_args(line, ":::grid");
    if let Some((cols, rows)) = args.split_once('x') {
        let cols = cols.trim().parse().unwrap_or(2);
        let rows = rows.trim().parse().unwrap_or(2);
        LayoutDirective::Grid { cols, rows }
    } else {
        LayoutDirective::Grid { cols: 2, rows: 1 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- extract_notes tests ---

    #[test]
    fn test_extract_block_notes() {
        let input = "Some content\n\n:::notes\nSpeaker note here\n:::\n\nMore content";
        let (notes, cleaned) = extract_notes(input);
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].text, "Speaker note here");
        assert!(!cleaned.contains(":::notes"));
        assert!(!cleaned.contains("Speaker note here"));
        assert!(cleaned.contains("Some content"));
        assert!(cleaned.contains("More content"));
    }

    #[test]
    fn test_extract_multiline_notes() {
        let input = ":::notes\nLine 1\nLine 2\nLine 3\n:::";
        let (notes, _cleaned) = extract_notes(input);
        assert_eq!(notes.len(), 1);
        assert!(notes[0].text.contains("Line 1"));
        assert!(notes[0].text.contains("Line 3"));
    }

    #[test]
    fn test_no_notes() {
        let input = "Just regular content\nNo notes here";
        let (notes, cleaned) = extract_notes(input);
        assert!(notes.is_empty());
        assert!(cleaned.contains("Just regular content"));
    }

    #[test]
    fn test_multiple_note_blocks() {
        let input = ":::notes\nNote 1\n:::\n\nContent\n\n:::notes\nNote 2\n:::";
        let (notes, _) = extract_notes(input);
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].text, "Note 1");
        assert_eq!(notes[1].text, "Note 2");
    }

    // --- extract_inline_notes tests ---

    #[test]
    fn test_inline_note_basic() {
        let input = "Some text ^[This is a note] more text";
        let (notes, cleaned) = extract_inline_notes(input);
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].text, "This is a note");
        assert!(cleaned.contains("Some text  more text"));
        assert!(!cleaned.contains("^["));
    }

    #[test]
    fn test_inline_note_at_end() {
        let input = "Text ^[End note]";
        let (notes, cleaned) = extract_inline_notes(input);
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].text, "End note");
        assert_eq!(cleaned.trim(), "Text");
    }

    #[test]
    fn test_multiple_inline_notes() {
        let input = "Text ^[Note 1] middle ^[Note 2] end";
        let (notes, cleaned) = extract_inline_notes(input);
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].text, "Note 1");
        assert_eq!(notes[1].text, "Note 2");
        assert!(!cleaned.contains("^["));
    }

    #[test]
    fn test_no_inline_notes() {
        let input = "Just regular text with a ^ caret";
        let (notes, cleaned) = extract_inline_notes(input);
        assert!(notes.is_empty());
        assert!(cleaned.contains("^"));
    }

    #[test]
    fn test_nested_brackets_in_note() {
        let input = "Text ^[Note with [brackets] inside]";
        let (notes, _) = extract_inline_notes(input);
        assert_eq!(notes.len(), 1);
        assert!(notes[0].text.contains("[brackets]"));
    }

    // --- extract_layout tests ---

    #[test]
    fn test_split_default() {
        let input = ":::split\nLeft content\n+++\nRight content\n:::";
        let (layout, regions) = extract_layout(input);
        assert!(layout.is_some());
        if let Some(LayoutDirective::Split { ratios }) = layout {
            assert_eq!(ratios, vec![50.0, 50.0]);
        } else {
            panic!("Expected Split directive");
        }
        assert_eq!(regions.len(), 2);
        assert!(regions[0].contains("Left content"));
        assert!(regions[1].contains("Right content"));
    }

    #[test]
    fn test_split_custom_ratio() {
        let input = ":::split 60/40\nLeft\n+++\nRight\n:::";
        let (layout, regions) = extract_layout(input);
        if let Some(LayoutDirective::Split { ratios }) = layout {
            assert_eq!(ratios, vec![60.0, 40.0]);
        } else {
            panic!("Expected Split directive");
        }
        assert_eq!(regions.len(), 2);
    }

    #[test]
    fn test_split_three_way() {
        let input = ":::split 33/34/33\nA\n+++\nB\n+++\nC\n:::";
        let (layout, regions) = extract_layout(input);
        if let Some(LayoutDirective::Split { ratios }) = layout {
            assert_eq!(ratios, vec![33.0, 34.0, 33.0]);
        } else {
            panic!("Expected Split directive");
        }
        assert_eq!(regions.len(), 3);
    }

    #[test]
    fn test_grid_layout() {
        let input = ":::grid 2x2\nA\n+++\nB\n+++\nC\n+++\nD\n:::";
        let (layout, regions) = extract_layout(input);
        if let Some(LayoutDirective::Grid { cols, rows }) = layout {
            assert_eq!(cols, 2);
            assert_eq!(rows, 2);
        } else {
            panic!("Expected Grid directive");
        }
        assert_eq!(regions.len(), 4);
    }

    #[test]
    fn test_grid_3x1() {
        let input = ":::grid 3x1\nA\n+++\nB\n+++\nC\n:::";
        let (layout, _) = extract_layout(input);
        if let Some(LayoutDirective::Grid { cols, rows }) = layout {
            assert_eq!(cols, 3);
            assert_eq!(rows, 1);
        } else {
            panic!("Expected Grid directive");
        }
    }

    #[test]
    fn test_stack_layout() {
        let input = ":::stack\nTop\n+++\nBottom\n:::";
        let (layout, regions) = extract_layout(input);
        assert!(matches!(layout, Some(LayoutDirective::Stack)));
        assert_eq!(regions.len(), 2);
    }

    #[test]
    fn test_no_layout() {
        let input = "Just regular markdown\n\nNo layout here";
        let (layout, regions) = extract_layout(input);
        assert!(layout.is_none());
        assert!(regions.is_empty());
    }

    #[test]
    fn test_layout_with_markdown_content() {
        let input = ":::split 50/50\n## Left Title\n\nSome **bold** text\n+++\n## Right Title\n\n- list item\n:::";
        let (layout, regions) = extract_layout(input);
        assert!(layout.is_some());
        assert_eq!(regions.len(), 2);
        assert!(regions[0].contains("## Left Title"));
        assert!(regions[1].contains("- list item"));
    }

    // --- parse_split edge cases ---

    #[test]
    fn test_parse_split_invalid_ratio() {
        let input = ":::split abc\nA\n+++\nB\n:::";
        let (layout, _) = extract_layout(input);
        if let Some(LayoutDirective::Split { ratios }) = layout {
            // Falls back to 50/50 when parse fails
            assert_eq!(ratios, vec![50.0, 50.0]);
        } else {
            panic!("Expected Split directive");
        }
    }

    // --- directive_args tests ---

    #[test]
    fn test_directive_args_basic() {
        assert_eq!(directive_args(":::split 60/40", ":::split"), "60/40");
    }

    #[test]
    fn test_directive_args_no_args() {
        assert_eq!(directive_args(":::split", ":::split"), "");
    }

    #[test]
    fn test_directive_args_wrong_prefix() {
        assert_eq!(directive_args(":::grid 2x2", ":::split"), "");
    }
}
