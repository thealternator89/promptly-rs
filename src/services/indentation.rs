/// Normalizes surplus indentation on pasted code blocks.
///
/// If line 0 has no leading whitespace and there are multiple lines, `min_indent` is
/// computed across non-empty lines in `lines[1..]`, leaving line 0 untouched.
/// Otherwise `min_indent` is computed across all non-empty lines.
pub fn fix_indentation(text: &str) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    if lines.is_empty() {
        return text.to_string();
    }

    let has_first_line_indent = lines[0].starts_with(|c: char| c == ' ' || c == '\t');
    let candidate_lines = if !has_first_line_indent && lines.len() > 1 {
        &lines[1..]
    } else {
        &lines[..]
    };

    let min_indent = candidate_lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.chars()
                .take_while(|c| *c == ' ' || *c == '\t')
                .count()
        })
        .min();

    let Some(min_indent) = min_indent else {
        return text.to_string();
    };

    if min_indent == 0 {
        return text.to_string();
    }

    lines
        .iter()
        .enumerate()
        .map(|(idx, line)| {
            if idx == 0 && !has_first_line_indent && lines.len() > 1 {
                (*line).to_string()
            } else if line.trim().is_empty() {
                String::new()
            } else {
                line.chars().skip(min_indent).collect()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedents_standard_indented_block() {
        let input = "    fn foo() {\n        println!(\"hi\");\n    }";
        let expected = "fn foo() {\n    println!(\"hi\");\n}";
        assert_eq!(fix_indentation(input), expected);
    }

    #[test]
    fn dedents_ide_copy_with_unindented_first_line() {
        let input = "function foo() {\n        const x = 1;\n        return x;\n    }";
        let expected = "function foo() {\n    const x = 1;\n    return x;\n}";
        assert_eq!(fix_indentation(input), expected);
    }

    #[test]
    fn normalizes_whitespace_only_lines_and_keeps_single_line_noop() {
        let single = "    let x = 1;";
        assert_eq!(fix_indentation(single), "let x = 1;");

        let no_indent = "let x = 1;";
        assert_eq!(fix_indentation(no_indent), "let x = 1;");

        let with_blank = "    a\n      \n    b";
        assert_eq!(fix_indentation(with_blank), "a\n\nb");
    }
}
