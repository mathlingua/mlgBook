//! Rendering helpers for MathLingua code fences.

use crate::html::{Element, Node};
use ego_tree::{NodeMut, Tree};

/// Returns syntax-highlighted nodes for a MathLingua source block.
pub(crate) fn highlight_source(source: &str) -> Tree<Node> {
    let mut tree = Tree::new(Node::Fragment);
    let mut root = tree.root_mut();
    append_highlighted_source(&mut root, source);
    tree
}

/// Returns rendered nodes for an `mlg-view` block.
pub(crate) fn render_view(source: &str) -> Tree<Node> {
    let mut tree = Tree::new(Node::Fragment);
    let mut view = Element::new("div");
    view.insert_attr("class", "mlg-view hljs".into());

    let mut root = tree.root_mut();
    let mut view = root.append(Node::Element(view));
    append_view_source(&mut view, source);

    tree
}

fn append_view_source(parent: &mut NodeMut<'_, Node>, source: &str) {
    let mut index = 0;
    while index < source.len() {
        let Some(open_relative) = find_unescaped_dollar(&source[index..]) else {
            append_highlighted_source(parent, &source[index..]);
            break;
        };
        let open = index + open_relative;
        append_highlighted_source(parent, &source[index..open]);

        let math_start = open + 1;
        let Some(close_relative) = find_unescaped_dollar(&source[math_start..]) else {
            append_highlighted_source(parent, &source[open..]);
            break;
        };
        let close = math_start + close_relative;
        append_inline_math(parent, &source[math_start..close]);
        index = close + 1;
    }
}

fn append_inline_math(parent: &mut NodeMut<'_, Node>, latex: &str) {
    let mut span = Element::new("span");
    span.insert_attr("class", "math math-inline".into());
    let mut span = parent.append(Node::Element(span));
    span.append(Node::Text(format!("\\({latex}\\)").into()));
}

fn find_unescaped_dollar(source: &str) -> Option<usize> {
    source
        .match_indices('$')
        .find_map(|(index, _)| (!is_escaped(source, index)).then_some(index))
}

fn is_escaped(source: &str, index: usize) -> bool {
    let mut slash_count = 0;
    for ch in source[..index].chars().rev() {
        if ch == '\\' {
            slash_count += 1;
        } else {
            break;
        }
    }
    slash_count % 2 == 1
}

fn append_highlighted_source(parent: &mut NodeMut<'_, Node>, source: &str) {
    for line in source.split_inclusive('\n') {
        let Some(line) = line.strip_suffix('\n') else {
            append_highlighted_line(parent, line);
            continue;
        };
        append_highlighted_line(parent, line);
        parent.append(Node::Text("\n".into()));
    }
}

fn append_highlighted_line(parent: &mut NodeMut<'_, Node>, line: &str) {
    let mut index = append_line_prefix(parent, line);
    if let Some(header_len) = header_len(&line[index..]) {
        let mut span = Element::new("span");
        span.insert_attr("class", "mlg-header".into());
        let mut header = parent.append(Node::Element(span));
        append_highlighted_fragment(&mut header, &line[index..index + header_len]);
        index += header_len;
    }

    append_highlighted_fragment(parent, &line[index..]);
}

fn append_highlighted_fragment(parent: &mut NodeMut<'_, Node>, source: &str) {
    let mut index = 0;
    while index < source.len() {
        let rest = &source[index..];

        if rest.starts_with("--") {
            append_span(parent, "hljs-comment", rest);
            break;
        }

        let ch = rest.chars().next().unwrap();
        if ch == '"' {
            let len = quoted_string_len(rest);
            append_span(parent, "hljs-string", &rest[..len]);
            index += len;
        } else if let Some(len) = label_len(rest) {
            append_span(parent, "hljs-symbol", &rest[..len]);
            index += len;
        } else if let Some(len) = named_operator_len(rest) {
            append_span(parent, "hljs-title", &rest[..len]);
            index += len;
        } else if let Some(len) = alias_operator_len(rest) {
            append_span(parent, "hljs-keyword", &rest[..len]);
            index += len;
        } else if rest.starts_with("\\:") {
            append_span(parent, "hljs-keyword", "\\:");
            index += 2;
        } else if rest.starts_with(":/") {
            append_span(parent, "hljs-keyword", ":/");
            index += 2;
        } else if ch == '\\' {
            let len = command_len(rest);
            if len > 1 {
                append_span(parent, "hljs-title mlg-command", &rest[..len]);
            } else {
                parent.append(Node::Text("\\".into()));
            }
            index += len;
        } else if ch.is_ascii_digit() {
            let len = number_len(rest);
            append_span(parent, "hljs-number", &rest[..len]);
            index += len;
        } else if is_word_start(ch) {
            let len = word_len(rest);
            let word = &rest[..len];
            if is_keyword(word) {
                append_span(parent, "hljs-keyword mlg-keyword", word);
            } else if word.ends_with('_') {
                append_span(parent, "hljs-variable", word);
            } else {
                parent.append(Node::Text(word.into()));
            }
            index += len;
        } else if let Some(len) = special_operator_len(rest) {
            append_span(parent, "hljs-keyword", &rest[..len]);
            index += len;
        } else {
            parent.append(Node::Text(ch.to_string().into()));
            index += ch.len_utf8();
        }
    }
}

fn header_len(source: &str) -> Option<usize> {
    let rest = source.strip_prefix('[')?;
    let end = rest.find(']')?;
    Some(end + 2)
}

fn append_line_prefix(parent: &mut NodeMut<'_, Node>, line: &str) -> usize {
    let mut index = leading_whitespace_len(line);
    if index > 0 {
        parent.append(Node::Text(line[..index].into()));
    }

    if line[index..].starts_with(". ") {
        append_span(parent, "hljs-meta", ". ");
        index += 2;
    }

    let label_start = index;
    let label_len = section_label_name_len(&line[label_start..]);
    if label_len > 0 && line[label_start + label_len..].starts_with(':') {
        append_span(
            parent,
            "hljs-section",
            &line[label_start..label_start + label_len + 1],
        );
        label_start + label_len + 1
    } else {
        index
    }
}

fn leading_whitespace_len(line: &str) -> usize {
    line.char_indices()
        .find_map(|(index, ch)| (!matches!(ch, ' ' | '\t')).then_some(index))
        .unwrap_or(line.len())
}

fn section_label_name_len(source: &str) -> usize {
    let mut chars = source.char_indices();
    let Some((_, first)) = chars.next() else {
        return 0;
    };
    if !first.is_ascii_alphabetic() {
        return 0;
    }
    source
        .char_indices()
        .find_map(|(index, ch)| (!is_word_char(ch)).then_some(index))
        .unwrap_or(source.len())
}

fn quoted_string_len(source: &str) -> usize {
    let mut escaped = false;
    for (index, ch) in source.char_indices().skip(1) {
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return index + ch.len_utf8();
        }
    }
    source.len()
}

fn label_len(source: &str) -> Option<usize> {
    let rest = source.strip_prefix("[:")?;
    let end = rest.find(":]")?;
    Some(2 + end + 2)
}

fn named_operator_len(source: &str) -> Option<usize> {
    let (prefix_len, rest) = if let Some(rest) = source.strip_prefix(":|") {
        (2, rest)
    } else if let Some(rest) = source.strip_prefix('|') {
        (1, rest)
    } else {
        return None;
    };

    let name_len = section_label_name_len(rest);
    if name_len == 0 || !rest[name_len..].starts_with('|') {
        return None;
    }

    let mut len = prefix_len + name_len + 1;
    if rest[name_len + 1..].starts_with(':') {
        len += 1;
    }
    Some(len)
}

fn alias_operator_len(source: &str) -> Option<usize> {
    [":=>", ":->", ":~>", ":="]
        .iter()
        .find_map(|operator| source.starts_with(operator).then_some(operator.len()))
}

fn command_len(source: &str) -> usize {
    let mut len = 1;
    for (index, ch) in source.char_indices().skip(1) {
        if is_word_char(ch) {
            len = index + ch.len_utf8();
        } else {
            break;
        }
    }
    len
}

fn number_len(source: &str) -> usize {
    source
        .char_indices()
        .find_map(|(index, ch)| (!ch.is_ascii_digit()).then_some(index))
        .unwrap_or(source.len())
}

fn word_len(source: &str) -> usize {
    let mut len = 0;
    for (index, ch) in source.char_indices() {
        if is_word_char(ch) {
            len = index + ch.len_utf8();
        } else if ch == '?' {
            len = index + 1;
            break;
        } else {
            break;
        }
    }
    len
}

fn special_operator_len(source: &str) -> Option<usize> {
    let mut len = 0;
    for (index, ch) in source.char_indices() {
        if matches!(
            ch,
            '-' | '~' | '!' | '#' | '%' | '^' | '&' | '*' | '+' | '=' | '|' | '<' | '>' | '/'
        ) {
            len = index + ch.len_utf8();
        } else {
            break;
        }
    }

    if len >= 2
        || matches!(
            source.chars().next(),
            Some('~' | '!' | '#' | '%' | '&' | '<' | '>')
        )
    {
        Some(len)
    } else {
        None
    }
}

fn is_word_start(ch: char) -> bool {
    ch.is_ascii_alphanumeric()
}

fn is_word_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn is_keyword(word: &str) -> bool {
    matches!(
        word,
        "is" | "via"
            | "is?"
            | "is_not?"
            | "not"
            | "allOf"
            | "anyOf"
            | "oneOf"
            | "exists"
            | "existsUnique"
            | "forAll"
            | "if"
            | "iff"
            | "then"
            | "given"
            | "suchThat"
            | "piecewise"
    )
}

fn append_span(parent: &mut NodeMut<'_, Node>, class: &str, text: &str) {
    let mut span = Element::new("span");
    span.insert_attr("class", class.into());
    let mut span = parent.append(Node::Element(span));
    span.append(Node::Text(text.into()));
}
