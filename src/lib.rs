use markdown::{to_html_with_options, CompileOptions, Options};
use mdbook::{
    book::{Book, BookItem, Chapter},
    errors::Error,
    preprocess::PreprocessorContext,
};
use regex::Regex;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

pub struct HintPreprocessor;

impl mdbook::preprocess::Preprocessor for HintPreprocessor {
    fn name(&self) -> &str {
        "hints"
    }

    fn run(&self, ctx: &PreprocessorContext, mut book: Book) -> Result<Book, Error> {
        let mut error: Option<Error> = None;
        book.for_each_mut(|item: &mut BookItem| {
            if error.is_some() {
                return;
            }
            if let BookItem::Chapter(ref mut chapter) = *item {
                if let Err(err) = handle_chapter(chapter, &ctx.config.book.src) {
                    error = Some(err)
                }
            }
        });
        error.map_or(Ok(book), Err)
    }

    fn supports_renderer(&self, renderer: &str) -> bool {
        renderer == "html"
    }
}

pub fn handle_chapter(chapter: &mut Chapter, src_path: &Path) -> Result<(), Error> {
    let extractor = HintsExtractor::new();
    extractor.output_json(src_path)?;

    render_hints(chapter, extractor.toml()?)?;

    Ok(())
}

#[derive(serde::Deserialize, Debug)]
struct HintEntry {
    hint: String,
    _auto: Option<Vec<String>>,
}

fn render_hints(chapter: &mut Chapter, hints: HashMap<String, HintEntry>) -> Result<(), Error> {
    render_manual(chapter, &hints)?;
    //render_auto(chapter, &hints)?;
    Ok(())
}
fn _render_auto(_chapter: &mut Chapter, _hints: &HashMap<String, HintEntry>) -> Result<(), Error> {
    todo!()
}

fn render_manual(chapter: &mut Chapter, hints: &HashMap<String, HintEntry>) -> Result<(), Error> {
    let re = Regex::new(r"\[(.*?)]\(~(.*?)\)")?;
    if re.is_match(&chapter.content) {
        let content = re
            .replace_all(&chapter.content, |caps: &regex::Captures| {
                let first_capture = &caps[1];
                let second_capture = &caps[2];

                if hints.get(second_capture).is_none() {
                    eprintln!(
                        "-----\nHint for `{}` ({}) is missing in hints.toml!\n-----",
                        second_capture,
                        &chapter.path.clone().unwrap().display()
                    );
                    first_capture.to_string()
                } else if second_capture.starts_with("!") {
                    first_capture.to_string()
                } else {
                    format!(
                        r#"<span class="hint" hint="{}">{}</span>"#,
                        second_capture, first_capture
                    )
                }
            })
            .to_string();

        chapter.content = content;
    };

    Ok(())
}

struct HintsExtractor {
    path: PathBuf,
}

impl HintsExtractor {
    fn new() -> Self {
        let path = std::env::current_dir().unwrap();
        Self { path }
    }
    fn toml(&self) -> Result<HashMap<String, HintEntry>, Error> {
        let hints = self.path.join("hints.toml");
        let hints = std::fs::read_to_string(hints)?;

        let toml: HashMap<String, HintEntry> = toml::from_str(&hints)?;

        Ok(toml)
    }

    fn output_json(&self, src_path: &Path) -> Result<(), Error> {
        let mut json: HashMap<String, String> = HashMap::new();

        for (name, entry) in self.toml()? {
            let mut hint = to_html_with_options(
                &entry.hint,
                &Options {
                    compile: CompileOptions {
                        allow_dangerous_html: true,
                        ..CompileOptions::default()
                    },
                    ..Options::default()
                },
            )
            .unwrap();

            hint = hint.replace("<p>", "").replace("</p>", "");

            json.insert(name, hint);
        }

        let file = File::create(src_path.join("hints.json"))?;
        let writer = BufWriter::new(file);

        serde_json::to_writer(writer, &json)?;

        Ok(())
    }
}
