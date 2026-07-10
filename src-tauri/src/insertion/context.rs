pub fn contextualize_spacing(text: &str, before: Option<char>, after: Option<char>) -> String {
    let mut result = text.to_string();
    if before.is_some_and(|value| !value.is_whitespace() && !"([{/'\"".contains(value))
        && !result.starts_with(|value: char| value.is_whitespace() || ",.!?;:)]}".contains(value))
    {
        result.insert(0, ' ');
    }
    if after.is_some_and(|value| !value.is_whitespace() && !",.!?;:)]}".contains(value))
        && !result.ends_with(char::is_whitespace)
    {
        result.push(' ');
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn adds_only_required_context_spacing() {
        assert_eq!(
            contextualize_spacing("world", Some('o'), Some('!')),
            " world"
        );
        assert_eq!(contextualize_spacing("hello", None, Some('w')), "hello ");
        assert_eq!(contextualize_spacing(", next", Some('d'), None), ", next");
    }
}
