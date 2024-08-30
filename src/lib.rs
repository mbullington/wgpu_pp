#![feature(proc_macro_span)]

extern crate proc_macro;

mod preprocessor;

use naga::front::wgsl::Frontend;
use naga::valid::{Capabilities, ValidationFlags, Validator};

use litrs::Literal;
use preprocessor::{preprocess, PreprocessorError};
use proc_macro::{Span, TokenStream, TokenTree};

fn validate_wgsl(wgsl_source: &str) -> Result<(), TokenStream> {
    let mut frontend = Frontend::new();
    let module = frontend.parse(wgsl_source).map_err(|e| {
        let msg = format!("failed to parse WGSL: {}", e.emit_to_string(wgsl_source));
        format!("compile_error!(\"{}\")", msg)
            .parse::<TokenStream>()
            .unwrap()
    })?;

    let mut validator = Validator::new(ValidationFlags::all(), Capabilities::default());
    validator.validate(&module).map_err(|e| {
        let msg = format!("failed to validate WGSL: {}", e.emit_to_string(wgsl_source));
        format!("compile_error!(\"{}\")", msg)
            .parse::<TokenStream>()
            .unwrap()
    })?;

    Ok(())
}

#[proc_macro]
pub fn include_wgsl(input: TokenStream) -> TokenStream {
    let input = input.into_iter().collect::<Vec<_>>();
    if input.len() != 1 {
        let msg = format!("expected exactly one input token, got {}", input.len());
        return format!("compile_error!(\"{}\")", msg).parse().unwrap();
    }

    let call_site = Span::call_site();
    let source_path = call_site.source_file().path();
    let basepath = match source_path.parent() {
        Some(p) => p,
        _ => {
            // This happens in the Rust Analyzer, just let it go...
            return "\"\"".parse().unwrap();
        }
    };

    let filename = match Literal::try_from(&input[0]) {
        Ok(Literal::String(str)) => str.value().to_string(),
        // Error if the token is not a string literal
        Err(e) => return e.to_compile_error(),
        _ => {
            return "compile_error!(\"expected a string literal\")"
                .to_string()
                .parse()
                .unwrap();
        }
    };

    let shader = match preprocess(&filename, basepath) {
        Ok(value) => value,
        Err(e) => match e {
            PreprocessorError::FileNotFound(filename) => {
                let msg = format!(
                    "file not found: {}",
                    basepath.join(filename).to_string_lossy()
                );
                return format!("compile_error!(\"{}\")", msg).parse().unwrap();
            }
            PreprocessorError::FileNotValidUtf8(filename) => {
                let msg = format!("file not valid utf-8: {}", filename);
                return format!("compile_error!(\"{}\")", msg).parse().unwrap();
            }
            PreprocessorError::UnknownDirective(directive) => {
                let msg = format!("unknown directive: {}", directive);
                return format!("compile_error!(\"{}\")", msg).parse().unwrap();
            }
            PreprocessorError::IncludeIncorrectArgs => {
                return "compile_error!(\"incorrect arguments to #include\")"
                    .to_string()
                    .parse()
                    .unwrap();
            }
            PreprocessorError::MacroNoParenthesis => {
                return "compile_error!(\"macro must have parenthesis\")"
                    .to_string()
                    .parse()
                    .unwrap();
            }
            PreprocessorError::MacroIncorrectArgs(expected, got) => {
                let msg = format!("macro expected {} arguments, got {}", expected, got);
                return format!("compile_error!(\"{}\")", msg).parse().unwrap();
            }
        },
    };

    match validate_wgsl(&shader) {
        Ok(_) => {}
        Err(e) => return e,
    }

    TokenTree::Literal(proc_macro::Literal::string(&shader)).into()
}
