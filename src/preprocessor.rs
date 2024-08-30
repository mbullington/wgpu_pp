use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{self, BufRead, BufReader},
    path::{Path, PathBuf},
};

use lazy_static::lazy_static;
use regex::Regex;

#[derive(Debug)]
pub enum PreprocessorError {
    FileNotFound(String),
    FileNotValidUtf8(String),
    UnknownDirective(String),
    IncludeIncorrectArgs,
    MacroNoParenthesis,
    MacroIncorrectArgs(usize, usize),
}

lazy_static! {
    static ref REGEX_ID: Regex =
        Regex::new(r"([_\p{XID_Start}][\p{XID_Continue}]+)|([\p{XID_Start}])").unwrap();

    // Regex for define expressions. This only covers macros.
    // - Group 1: identifier
    // - Group 2: arguments separated by commas
    // - Group 3: body
    static ref REGEX_DEFINE_MACRO: Regex = Regex::new(r"((?:[_\p{XID_Start}][\p{XID_Continue}]+)|(?:[\p{XID_Start}]))\(((?:[_\p{XID_Start}][\p{XID_Continue}]*(?:,\s*)*)+)\)\s+(.*)").unwrap();

    // Regex for block comments.
    static ref REGEX_BLOCK_COMMENT: Regex = Regex::new(r"/\*.*?\*/").unwrap();
}

fn _remove_comments(line: &mut String, in_block_comment: bool) -> bool {
    // If we're already in a block comment, we need a */.
    if in_block_comment {
        if let Some(closing_idx) = line.find("*/") {
            line.replace_range(0..closing_idx + 2, "")
        } else {
            // Return "identity," i.e. skip this line.
            return in_block_comment;
        }
    }

    // Replace all contained block comments with regex.
    while let Some(block_comment) = REGEX_BLOCK_COMMENT.find(line) {
        line.replace_range(block_comment.start()..block_comment.end(), "");
    }

    // Remove inline comments.
    if let Some(inline_comment_idx) = line.find("//") {
        line.replace_range(inline_comment_idx.., "");
        return false;
    }
    // See if there's any hanging block comments.
    if line.contains("/*") {
        return true;
    }
    false
}

enum DefineDirective {
    Value(String),
    Macro(Vec<String>, String),
}

fn _substitute_macros(
    line: String,
    defines: &HashMap<String, DefineDirective>,
) -> Result<(bool, String), PreprocessorError> {
    // Get all the identifiers in the line.
    let mut result = line.clone();
    let mut i = 0;

    while i < result.len() {
        let id = REGEX_ID.find(&result[i..]);
        if let Some(id) = id {
            let id_start = i + id.start();
            let id_end = i + id.end();

            let id = id.as_str();
            let id_len = id.len();

            match defines.get(id) {
                Some(DefineDirective::Value(value)) => {
                    result.replace_range(id_start..id_end, value);
                }
                Some(DefineDirective::Macro(args, body)) => {
                    // Make sure the directly next token is a parenthesis.
                    if &result[id_end..id_end + 1] != "(" {
                        i += id_len;
                        continue;
                    }

                    // Find the closing parenthesis and where the commas are.
                    let mut paren_count = 0;
                    let mut paren_idx = id_end;
                    let mut commas_idx: Vec<usize> = vec![];
                    while paren_idx < result.len() {
                        match &result[paren_idx..paren_idx + 1] {
                            "(" | "{" => paren_count += 1,
                            ")" | "}" => {
                                paren_count -= 1;
                                if paren_count == 0 {
                                    break;
                                }
                            }
                            "," => {
                                // If we're only at the root level, add the comma index.
                                if paren_count == 1 {
                                    commas_idx.push(paren_idx);
                                }
                            }
                            _ => {}
                        }
                        paren_idx += 1;
                    }

                    // If we didn't find a closing parenthesis, skip this.
                    if paren_count != 0 {
                        i += id_len;
                        return Err(PreprocessorError::MacroNoParenthesis);
                    }

                    // Split the string id_end..paren_idx+1 by commas_idx.
                    let mut arg_values: Vec<String> = vec![];
                    let mut arg_start = id_end + 1;
                    for comma_idx in commas_idx.iter() {
                        arg_values.push(result[arg_start..*comma_idx].trim().to_string());
                        arg_start = comma_idx + 1;
                    }

                    // Add the last argument.
                    arg_values.push(result[arg_start..paren_idx].trim().to_string());

                    // If the number of arguments doesn't match, skip this.
                    if arg_values.len() != args.len() {
                        i += id_len;
                        return Err(PreprocessorError::MacroIncorrectArgs(
                            args.len(),
                            arg_values.len(),
                        ));
                    }

                    // Create a "defines" map with the arguments.
                    let mut arg_defines = HashMap::new();
                    for (arg_name, arg_value) in args.iter().zip(arg_values.iter()) {
                        arg_defines.insert(
                            arg_name.to_string(),
                            // Swallow errors here, as they might be incomplete.
                            DefineDirective::Value(
                                _substitute_macros(arg_value.to_string(), defines)
                                    .or_else(|_| Ok((false, arg_value.to_string())))?
                                    .1,
                            ),
                        );
                    }

                    // Substitute the body with the arguments.
                    let (_changed, new_body) = _substitute_macros(body.to_string(), &arg_defines)?;

                    result.replace_range(id_start..paren_idx + 1, &new_body);
                }
                _ => {}
            }

            i += id_len;
        } else {
            break;
        }
    }

    Ok((result != line, result))
}

fn _preprocess(
    filename: &str,
    basepath: &Path,
    visited: &mut HashSet<PathBuf>,
    defines: &mut HashMap<String, DefineDirective>,
) -> Result<String, PreprocessorError> {
    // See if the file exists, relative to the basepath.
    // If it doesn't, return an error.
    let source_path = basepath.join(filename);
    let source_path_parent = PathBuf::from(source_path.parent().unwrap());

    if visited.contains(&source_path) {
        return Ok("".to_string());
    }
    visited.insert(source_path.clone());

    let file = match File::open(source_path) {
        Ok(f) => f,
        Err(_) => return Err(PreprocessorError::FileNotFound(filename.to_string())),
    };

    // Read the file into a string.
    let br = BufReader::new(file);
    let mut contents = String::new();
    let lines = br
        .lines()
        .collect::<Result<Vec<_>, io::Error>>()
        .map_err(|_| PreprocessorError::FileNotValidUtf8(filename.to_string()))?;

    let mut i = 0;
    let mut in_block_comment = false;
    loop {
        if i >= lines.len() {
            break;
        }

        let mut line = lines[i].to_string();
        // While the line's last character is a backslash, remove the backslash and append the next line.
        while line.ends_with('\\') {
            line.pop();
            i += 1;
            if i >= lines.len() {
                break;
            }
            line += &lines[i];
        }

        // Remove opening/closing pairs of block comments via regex.
        in_block_comment = _remove_comments(&mut line, in_block_comment);
        if in_block_comment {
            i += 1;
            continue;
        }

        // Parse precompiler directives.
        if let Some(directive_idx) = line.find('#') {
            // If the closer index is a directive, process it.
            let mut directive_content = "".to_string();
            let directive_line = &line[directive_idx..];
            let directive_args = directive_line
                .split(' ')
                .filter(|arg| !arg.trim().is_empty())
                .collect::<Vec<&str>>();

            if directive_args[0] == "#include" {
                if directive_args.len() != 2 {
                    return Err(PreprocessorError::IncludeIncorrectArgs);
                }
                let dest_path = directive_args[1];
                if !((dest_path.starts_with('"') && dest_path.ends_with('"'))
                    || (dest_path.starts_with('<') && dest_path.ends_with('>')))
                {
                    return Err(PreprocessorError::IncludeIncorrectArgs);
                }

                let dest_path = &dest_path[1..dest_path.len() - 1];

                let contents_to_add =
                    _preprocess(dest_path, &source_path_parent, visited, defines)?;
                directive_content += &contents_to_add
            } else if directive_args[0] == "#define" {
                if directive_args.len() < 3 {
                    return Err(PreprocessorError::IncludeIncorrectArgs);
                }

                // Check if it's a macro.
                if let Some(caps) = REGEX_DEFINE_MACRO.captures(directive_line) {
                    let macro_name = caps.get(1).unwrap().as_str();
                    let macro_args = caps.get(2).unwrap().as_str();
                    let macro_body = caps.get(3).unwrap().as_str();

                    let macro_args = macro_args
                        .split(',')
                        .map(|arg| arg.trim().to_string())
                        .collect::<Vec<String>>();

                    defines.insert(
                        macro_name.to_string(),
                        DefineDirective::Macro(macro_args, macro_body.to_string()),
                    );
                } else {
                    let var_name = directive_args[1];
                    let var_value = directive_args[2..].join(" ");

                    defines.insert(var_name.to_string(), DefineDirective::Value(var_value));
                }
            } else if directive_args[0] == "#undef" {
                if directive_args.len() != 2 {
                    return Err(PreprocessorError::IncludeIncorrectArgs);
                }

                let var_name = directive_args[1];
                defines.remove(var_name);
            } else {
                return Err(PreprocessorError::UnknownDirective(
                    directive_args[0].to_string(),
                ));
            }

            // Return the line up to the closer index.
            line.replace_range(directive_idx.., &directive_content);
        };

        // Substitute macros until there are no more to substitute.
        loop {
            let (changed, new_line) = _substitute_macros(line, defines)?;
            line = new_line;

            if !changed {
                break;
            }
        }

        // Add the line to the contents.
        contents.push_str(&line);
        contents.push('\n');

        i += 1;
    }

    Ok(contents)
}

/// Loads a WGSL and preprocesses it.
pub fn preprocess(filename: &str, basepath: &Path) -> Result<String, PreprocessorError> {
    _preprocess(
        filename,
        basepath,
        &mut HashSet::new(), // visited
        &mut HashMap::new(), // defines
    )
}

#[cfg(test)]
mod tests {
    use std::fs::read_dir;

    use super::*;

    #[test]
    fn test_snapshot() {
        // Get workspace root from CARGO_WORKSPACE_DIR.
        let workspace_root = Path::new(env!("CARGO_WORKSPACE_DIR"));
        let current_file = file!();

        let snapshot_dir = Path::new(workspace_root)
            .join(current_file)
            .join("../../fixtures")
            .canonicalize()
            .unwrap();

        for entry in read_dir(&snapshot_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let filename = path.file_name().unwrap().to_str().unwrap().to_string();
            if !filename.ends_with(".wgsl") {
                continue;
            }

            println!("starting {}", filename);

            let result = preprocess(&filename, &snapshot_dir);
            if !result.is_ok() {
                println!("{:?}", result);
            }
            assert!(result.is_ok(), "Failed to preprocess file: {}", filename);

            insta::assert_snapshot!(filename, result.unwrap());
        }
    }
}
