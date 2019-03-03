pub fn is_opening_indent(ch: char) -> bool {
    match ch {
        '[' | '(' | '<' | '{' => true,
        _ => false,
    }
}

pub fn is_closing_indent(ch: char) -> bool {
    match ch {
        ']' | ')' | '>' | '}' => true,
        _ => false,
    }
}

pub fn is_word_forming(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

pub fn is_newline(ch: char) -> bool {
    ch == '\n'
}

pub fn is_not_newline(ch: char) -> bool {
    ch != '\n'
}

pub fn is_non_newline_whitespace(ch: char) -> bool {
    ch.is_whitespace() && ch != '\n'
}
