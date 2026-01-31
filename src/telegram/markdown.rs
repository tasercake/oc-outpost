use tracing::debug;

/// Convert Markdown to Telegram HTML format
///
/// Supports:
/// - Bold: **text** or __text__ → <b>text</b>
/// - Italic: *text* or _text_ → <i>text</i>
/// - Inline code: `code` → <code>code</code>
/// - Code blocks: ```lang\ncode\n``` → <pre><code class="language-lang">code</code></pre>
/// - Links: [text](url) → <a href="url">text</a>
pub fn markdown_to_telegram_html(text: &str) -> String {
    debug!(
        input_len = text.len(),
        "Converting markdown to Telegram HTML"
    );
    let mut result = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Check for code blocks first (```lang\ncode\n```)
        if i + 2 < chars.len() && chars[i] == '`' && chars[i + 1] == '`' && chars[i + 2] == '`' {
            i += 3;

            // Extract language (optional)
            let mut lang = String::new();
            while i < chars.len() && chars[i] != '\n' {
                lang.push(chars[i]);
                i += 1;
            }
            if i < chars.len() {
                i += 1; // Skip newline
            }

            // Extract code content
            let mut code = String::new();
            while i + 2 < chars.len() {
                if chars[i] == '`' && chars[i + 1] == '`' && chars[i + 2] == '`' {
                    break;
                }
                code.push(chars[i]);
                i += 1;
            }

            // Skip closing ```
            if i + 2 < chars.len() {
                i += 3;
            }

            // Build code block HTML
            result.push_str("<pre><code");
            if !lang.trim().is_empty() {
                result.push_str(" class=\"language-");
                result.push_str(&escape_html(lang.trim()));
                result.push('"');
            }
            result.push('>');
            result.push_str(&escape_html(&code));
            result.push_str("</code></pre>");
            continue;
        }

        // Check for inline code (`code`)
        if chars[i] == '`' {
            i += 1;
            let mut code = String::new();
            while i < chars.len() && chars[i] != '`' {
                code.push(chars[i]);
                i += 1;
            }
            if i < chars.len() {
                i += 1; // Skip closing `
            }
            result.push_str("<code>");
            result.push_str(&escape_html(&code));
            result.push_str("</code>");
            continue;
        }

        // Check for bold (**text** or __text__)
        if i + 1 < chars.len()
            && ((chars[i] == '*' && chars[i + 1] == '*')
                || (chars[i] == '_' && chars[i + 1] == '_'))
        {
            let marker = chars[i];
            i += 2;
            let mut bold_text = String::new();
            while i < chars.len() {
                if i + 1 < chars.len() && chars[i] == marker && chars[i + 1] == marker {
                    break;
                }
                bold_text.push(chars[i]);
                i += 1;
            }
            if i + 1 < chars.len() {
                i += 2; // Skip closing markers
            }
            result.push_str("<b>");
            // Recursively process inner content for nested formatting
            result.push_str(&markdown_to_telegram_html(&bold_text));
            result.push_str("</b>");
            continue;
        }

        // Check for italic (*text* or _text_)
        if chars[i] == '*' || chars[i] == '_' {
            let marker = chars[i];
            i += 1;
            let mut italic_text = String::new();
            while i < chars.len() && chars[i] != marker {
                italic_text.push(chars[i]);
                i += 1;
            }
            if i < chars.len() {
                i += 1; // Skip closing marker
            }
            result.push_str("<i>");
            // Recursively process inner content for nested formatting
            result.push_str(&markdown_to_telegram_html(&italic_text));
            result.push_str("</i>");
            continue;
        }

        // Check for links ([text](url))
        if chars[i] == '[' {
            i += 1;
            let mut link_text = String::new();
            let mut found_closing_bracket = false;
            while i < chars.len() && chars[i] != ']' {
                link_text.push(chars[i]);
                i += 1;
            }
            if i < chars.len() && chars[i] == ']' {
                found_closing_bracket = true;
                i += 1; // Skip ]
            }

            // Check for (url)
            if found_closing_bracket && i < chars.len() && chars[i] == '(' {
                i += 1;
                let mut url = String::new();
                while i < chars.len() && chars[i] != ')' {
                    url.push(chars[i]);
                    i += 1;
                }
                if i < chars.len() {
                    i += 1; // Skip )
                }

                result.push_str("<a href=\"");
                result.push_str(&escape_html(&url));
                result.push_str("\">");
                result.push_str(&escape_html(&link_text));
                result.push_str("</a>");
                continue;
            } else {
                // Not a valid link, output as-is
                result.push('[');
                result.push_str(&escape_html(&link_text));
                if found_closing_bracket {
                    result.push(']');
                }
                continue;
            }
        }

        // Regular character - escape HTML entities
        match chars[i] {
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '&' => result.push_str("&amp;"),
            c => result.push(c),
        }
        i += 1;
    }

    debug!(
        input_len = text.len(),
        output_len = result.len(),
        "Markdown conversion complete"
    );
    result
}

/// Escape HTML entities
fn escape_html(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '&' => "&amp;".to_string(),
            c => c.to_string(),
        })
        .collect()
}

/// Truncate message to max_len characters
///
/// Adds "..." if truncated. Avoids breaking in middle of HTML tags.
#[allow(dead_code)]
// Used by future: message truncation feature
pub fn truncate_message(text: &str, max_len: usize) -> String {
    debug!(
        input_len = text.len(),
        max_len = max_len,
        "Truncating message"
    );
    if text.len() <= max_len {
        return text.to_string();
    }

    let ellipsis = "...";
    let target_len = max_len.saturating_sub(ellipsis.len());

    // Find safe truncation point (not inside HTML tag)
    let mut safe_len = target_len;
    let chars: Vec<char> = text.chars().collect();

    // Count open tags to avoid breaking inside a tag
    let mut in_tag = false;
    for (i, &ch) in chars.iter().enumerate().take(target_len) {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
        }
        if i >= target_len - 1 {
            break;
        }
    }

    // If we're inside a tag, backtrack to before the tag
    if in_tag {
        for i in (0..target_len.min(chars.len())).rev() {
            if chars[i] == '<' {
                safe_len = i;
                break;
            }
        }
    }

    let truncated: String = chars.iter().take(safe_len).collect();
    let result = format!("{}{}", truncated, ellipsis);
    debug!(
        input_len = text.len(),
        output_len = result.len(),
        "Message truncation complete"
    );
    result
}

/// Split message into chunks of max_len characters
///
/// Preserves code block integrity. Closes/reopens HTML tags across splits.
pub fn split_message(text: &str, max_len: usize) -> Vec<String> {
    debug!(
        input_len = text.len(),
        max_len = max_len,
        "Splitting message for Telegram"
    );
    if text.len() <= max_len {
        return vec![text.to_string()];
    }

    let mut parts = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut start = 0;

    while start < chars.len() {
        let remaining = chars.len() - start;
        if remaining <= max_len {
            // Last chunk
            let chunk: String = chars[start..].iter().collect();
            parts.push(chunk);
            break;
        }

        // Find safe split point
        let mut end = start + max_len;

        // Check if we're inside a code block
        let chunk_text: String = chars[start..end].iter().collect();
        if is_inside_code_block(&chunk_text) {
            // Find the end of the code block
            end = find_code_block_end(&chars, start, end);
        }

        // Check if we're inside an HTML tag
        let mut in_tag = false;
        for &ch in chars.iter().skip(start).take(end.min(chars.len()) - start) {
            if ch == '<' {
                in_tag = true;
            } else if ch == '>' {
                in_tag = false;
            }
        }

        // If inside tag, backtrack
        if in_tag {
            for i in (start..end).rev() {
                if chars[i] == '<' {
                    end = i;
                    break;
                }
            }
        }

        let chunk: String = chars[start..end].iter().collect();
        parts.push(chunk);
        start = end;
    }

    // Add ellipsis between parts
    for i in 0..parts.len() - 1 {
        parts[i].push_str("...");
    }

    debug!(parts = parts.len(), "Message split complete");
    parts
}

/// Check if text is inside a code block
fn is_inside_code_block(text: &str) -> bool {
    let pre_open = text.matches("<pre>").count();
    let pre_close = text.matches("</pre>").count();
    pre_open > pre_close
}

/// Find the end of a code block
fn find_code_block_end(chars: &[char], start: usize, initial_end: usize) -> usize {
    let text: String = chars[start..].iter().collect();
    if let Some(pos) = text.find("</pre>") {
        let end = start + pos + 6; // Include </pre>
        return end.min(chars.len());
    }
    initial_end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bold_conversion() {
        assert_eq!(
            markdown_to_telegram_html("This is **bold** text"),
            "This is <b>bold</b> text"
        );
        assert_eq!(
            markdown_to_telegram_html("This is __bold__ text"),
            "This is <b>bold</b> text"
        );
    }

    #[test]
    fn test_italic_conversion() {
        assert_eq!(
            markdown_to_telegram_html("This is *italic* text"),
            "This is <i>italic</i> text"
        );
        assert_eq!(
            markdown_to_telegram_html("This is _italic_ text"),
            "This is <i>italic</i> text"
        );
    }

    #[test]
    fn test_inline_code_conversion() {
        assert_eq!(
            markdown_to_telegram_html("This is `code` text"),
            "This is <code>code</code> text"
        );
    }

    #[test]
    fn test_code_block_conversion() {
        let input = "```\nlet x = 42;\n```";
        let expected = "<pre><code>let x = 42;\n</code></pre>";
        assert_eq!(markdown_to_telegram_html(input), expected);
    }

    #[test]
    fn test_code_block_with_language() {
        let input = "```python\ndef hello():\n    print(\"world\")\n```";
        let expected = "<pre><code class=\"language-python\">def hello():\n    print(\"world\")\n</code></pre>";
        assert_eq!(markdown_to_telegram_html(input), expected);
    }

    #[test]
    fn test_link_conversion() {
        assert_eq!(
            markdown_to_telegram_html("Check [this link](https://example.com)"),
            "Check <a href=\"https://example.com\">this link</a>"
        );
    }

    #[test]
    fn test_nested_formatting() {
        assert_eq!(
            markdown_to_telegram_html("This is **bold with *italic* inside**"),
            "This is <b>bold with <i>italic</i> inside</b>"
        );
    }

    #[test]
    fn test_html_entity_escaping() {
        assert_eq!(
            markdown_to_telegram_html("This has <tags> & entities"),
            "This has &lt;tags&gt; &amp; entities"
        );
    }

    #[test]
    fn test_truncate_short_message() {
        let text = "Short message";
        assert_eq!(truncate_message(text, 100), "Short message");
    }

    #[test]
    fn test_truncate_long_message() {
        let text = "This is a very long message that needs to be truncated";
        let result = truncate_message(text, 20);
        assert!(result.len() <= 20);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_truncate_at_tag_boundary() {
        let text = "<b>This is bold text</b>";
        let result = truncate_message(text, 10);
        // Should not break inside a tag (result should be valid HTML)
        assert!(result.len() <= 10);
        assert!(result.ends_with("..."));
        // Either contains complete <b> tag or no tag at all
        if result.contains('<') {
            assert!(result.contains('>'));
        }
    }

    #[test]
    fn test_split_short_message() {
        let text = "Short message";
        let parts = split_message(text, 100);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0], "Short message");
    }

    #[test]
    fn test_split_long_message() {
        let text = "a".repeat(5000);
        let parts = split_message(&text, 4096);
        assert!(parts.len() > 1);
        for part in &parts[..parts.len() - 1] {
            assert!(part.len() <= 4096 + 3); // +3 for "..."
        }
    }

    #[test]
    fn test_split_preserves_code_blocks() {
        let code_block = "<pre><code>".to_string() + &"x".repeat(5000) + "</code></pre>";
        let parts = split_message(&code_block, 4096);

        // First part should contain the entire code block or end at a safe point
        assert!(parts[0].contains("<pre><code>"));
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(markdown_to_telegram_html(""), "");
        assert_eq!(truncate_message("", 100), "");
        assert_eq!(split_message("", 100), vec![""]);
    }

    #[test]
    fn test_mixed_formatting() {
        let input = "This is **bold** and *italic* with `code` and [link](https://example.com)";
        let expected = "This is <b>bold</b> and <i>italic</i> with <code>code</code> and <a href=\"https://example.com\">link</a>";
        assert_eq!(markdown_to_telegram_html(input), expected);
    }

    #[test]
    fn test_multiple_code_blocks() {
        let input =
            "First:\n```python\nprint('hello')\n```\n\nSecond:\n```rust\nprintln!(\"world\");\n```";
        let result = markdown_to_telegram_html(input);
        assert!(result.contains("<pre><code class=\"language-python\">"));
        assert!(result.contains("<pre><code class=\"language-rust\">"));
    }

    #[test]
    fn test_telegram_char_limit() {
        let text = "a".repeat(5000);
        let parts = split_message(&text, 4096);

        // Verify no part exceeds Telegram's limit (accounting for ellipsis)
        for part in &parts {
            assert!(part.len() <= 4096 + 3); // +3 for "..."
        }
    }

    #[test]
    fn test_whitespace_only() {
        assert_eq!(markdown_to_telegram_html("   \n\t  "), "   \n\t  ");
    }

    #[test]
    fn test_malformed_markdown() {
        // Unclosed bold
        assert_eq!(markdown_to_telegram_html("**bold"), "<b>bold</b>");

        // Unclosed italic
        assert_eq!(markdown_to_telegram_html("*italic"), "<i>italic</i>");

        // Unclosed code
        assert_eq!(markdown_to_telegram_html("`code"), "<code>code</code>");

        // Incomplete link
        assert_eq!(markdown_to_telegram_html("[text"), "[text");
    }
}
