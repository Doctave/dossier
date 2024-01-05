/// Parses the comment and returns it without the comment delimiters.
pub(crate) fn process_comment(comment: &str) -> String {
    let mut tmp = comment.trim().to_owned();
    tmp = tmp.trim_start_matches("/**").to_owned();
    tmp = tmp.trim_end_matches("*/").to_owned();

    tmp.lines()
        .map(|l| l.trim().trim_start_matches("* ").trim_start_matches('*'))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_owned()
}
