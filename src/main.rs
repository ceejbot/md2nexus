use std::fs::create_dir_all;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::PathBuf;

use clap::Parser;
use clap_complete::{generate, Shell};
use markdown::mdast::Node;
use markdown::{to_mdast, ParseOptions};
use owo_colors::OwoColorize;
use prettytable::{Cell, Row, Table};

#[derive(Clone, Debug, Parser)]
#[clap(name = "md2nexus", version)]
/// A command-line tool to convert gfm markdown to NexusMods-flavored bbcode.
///
pub struct Args {
    /// Path to an input file or directory. If omitted, input is read from stdin.
    #[clap(short, long, value_name = "FILE")]
    input: Option<PathBuf>,
    /// Path to an output file or directory. If omitted, single-file input is written
    /// to stdout. If the input option is a directory, output files are written to '.'.
    #[clap(short, long, value_name = "FILE")]
    output: Option<PathBuf>,
    /// Emit completion data for your preferred shell.
    #[clap(short, long)]
    completions: Option<Shell>,
    /// Print GPL-3.0 license information.
    #[clap(long)]
    license: bool,
}

/// Handle all the markdown files in a directory, converting each an.
/// This does not yet visit subdirs, because I haven't needed that use case.
/// This function assumes you've created the output dir already.
fn handle_directory(dirpath: PathBuf, outpath: PathBuf) -> anyhow::Result<()> {
    println!(
        "Converting all files in directory {}",
        dirpath.display().bold().blue()
    );
    for entry in std::fs::read_dir(dirpath)? {
        let entry = entry?;
        let fpath = entry.path();
        if !fpath.is_file() {
            continue;
        }
        let Some(filename) = fpath.file_name() else {
        continue;
    };
        let Some(ext) = fpath.extension() else {
        continue;
    };
        if ext != "md" {
            continue;
        }
        let outfname = outpath
            .clone()
            .join(filename.to_string_lossy().replace(".md", ".bbcode"));

        let mut data = String::new();
        let file = File::open(&fpath)?;
        let mut reader = BufReader::new(file);
        reader.read_to_string(&mut data)?;
        let result = convert_buffer(&data)?;
        let mut output = File::create(&outfname)?;
        write!(output, "{result}")?;
        println!(
            "    {} => {}",
            fpath.display().yellow(),
            outfname.display().bright_magenta()
        );
    }

    Ok(())
}

/// Given an input markdown str, emit nexus bbcode as a string.
fn convert_buffer(input: &str) -> anyhow::Result<String> {
    // This function is infallible with the default options.
    let tree =
        to_mdast(input, &ParseOptions::gfm()).expect("failed to parse markdown as valid GFM.");
    if let Some(children) = tree.children() {
        let mut state = State::new();
        Ok(state.convert_children(children))
    } else {
        Ok("".to_string())
    }
}

/// State is the worst.
struct State {
    table: Option<Table>,
    row: Option<Row>,
}

impl State {
    pub fn new() -> Self {
        State {
            table: None,
            row: None,
        }
    }

    /// Convert a single node type to bbcode, recursing into children if it has any.
    pub fn convert_node(&mut self, node: &Node) -> String {
        match node {
            Node::Root(root) => self.convert_children(&root.children),
            Node::Paragraph(p) => format!("{}\n\n", self.convert_children(&p.children)),
            Node::BlockQuote(t) => {
                format!("\n[quote]{}[/quote]\n", self.convert_children(&t.children))
            }
            Node::List(list) => {
                if list.ordered {
                    format!(
                        "\n[list=1]\n{}[/list]\n\n",
                        self.convert_children(&list.children)
                    )
                } else {
                    format!(
                        "\n[list]\n{}[/list]\n\n",
                        self.convert_children(&list.children)
                    )
                }
            }
            Node::Toml(t) => format!("\n[code]{}[/code]\n\n", t.value),
            Node::Yaml(t) => format!("\n[code]{}[/code]\n\n", t.value),
            Node::Break(_) => "\n\n".to_string(),
            Node::InlineCode(t) => {
                if self.table.is_none() && self.row.is_none() {
                    format!("[font=\"Courier\"]{}[/font]", t.value)
                } else {
                    t.value.clone()
                }
            }
            Node::InlineMath(t) => format!("[font=\"Courier\"]{}[/font]", t.value),
            Node::Delete(t) => format!("[s]{}[/s]", self.convert_children(&t.children)),
            Node::Emphasis(t) => format!("[i]{}[/i]", self.convert_children(&t.children)),
            Node::Html(t) => t.value.clone(),
            Node::Image(t) => format!("[img]{}[/img]", t.url),
            Node::Link(link) => format!(
                "[url={}]{}[/url]",
                link.url.clone(),
                self.convert_children(&link.children)
            ),
            Node::Strong(t) => format!("\n[b]{}[/b]\n", self.convert_children(&t.children)),
            Node::Text(t) => t.value.clone(),
            Node::Code(t) => format!("\n[code]\n{}\n[/code]\n\n", t.value),
            Node::Math(t) => format!("\n[code]\n{}\n[/code]\n\n", t.value), // no equivalent in bbcode
            Node::Heading(h) => {
                format!(
                    "\n[heading]{}[/heading]\n\n",
                    self.convert_children(&h.children)
                )
            }
            Node::Table(t) => {
                let mut tablestate = State::new();
                tablestate.table = Some(Table::new());
                tablestate.convert_children(&t.children);
                if let Some(finished) = tablestate.table.clone() {
                    let result = format!("\n[code]\n{finished}[/code]\n");
                    result
                } else {
                    "".to_string()
                }
            }
            Node::ThematicBreak(_) => "\n\n[line]\n\n".to_string(),
            Node::TableRow(row) => {
                if let Some(ref table) = self.table {
                    let mut rowstate = State::new();
                    rowstate.row = Some(Row::new(Vec::new()));
                    rowstate.convert_children(&row.children);
                    if let Some(finished) = rowstate.row.clone() {
                        let mut clone = table.clone();
                        clone.add_row(finished);
                        self.table = Some(clone);
                    }
                }
                "".to_string()
            }
            Node::TableCell(cell) => {
                if let Some(ref row) = self.row {
                    let mut cloned = row.clone();
                    let string = format!("{}", self.convert_children(&cell.children));
                    let cell = Cell::new(string.as_str());
                    cloned.add_cell(cell);
                    self.row = Some(cloned);
                }
                "".to_string()
            }
            Node::ListItem(t) => format!("[*]{}", self.convert_children(&t.children)),

            // the following markup types have meh support
            Node::FootnoteReference(footie) => {
                format!("(See ^{})", footie.identifier)
            }
            Node::FootnoteDefinition(note) => {
                format!(
                    "\n^{}: {}",
                    note.identifier,
                    self.convert_children(&note.children)
                )
            }
            Node::Definition(def) => {
                // this is a definition used by link and image references
                let anchor = if let Some(ref title) = def.title {
                    title.clone()
                } else {
                    def.identifier.clone()
                };
                format!("\n^{}: [url={}]{}[/url]", def.identifier, def.url, anchor)
            }
            Node::ImageReference(imgref) => {
                format!("[{}] {}", imgref.identifier, imgref.alt)
            }
            Node::LinkReference(linkref) => {
                format!(
                    "(See image {}; {})",
                    linkref.identifier,
                    self.convert_children(&linkref.children)
                )
            }

            // completely unsupported markup types follow
            Node::MdxTextExpression(_) => {
                eprintln!("mdx not supported in nexus bbcode");
                "".to_string()
            }
            Node::MdxFlowExpression(_) => {
                eprintln!("mdx not supported in nexus bbcode");
                "".to_string()
            }
            Node::MdxJsxFlowElement(_) => {
                eprintln!("mdx/jxs not supported in nexus bbcode");
                "".to_string()
            }
            Node::MdxjsEsm(_) => {
                eprintln!("mdx/jxs not supported in nexus bbcode");
                "".to_string()
            }
            Node::MdxJsxTextElement(_) => {
                eprintln!("mdx/jxs not supported in nexus bbcode");
                "".to_string()
            }
        }
    }

    pub fn convert_children(&mut self, children: &[Node]) -> String {
        children
            .iter()
            .map(|xs| self.convert_node(xs))
            .collect::<Vec<String>>()
            .join("")
    }
}

fn license() {
    println!(
        r#"
md2nexus Copyright (C) 2023 C J Silverio

This program is free software: you can redistribute it and/or modify it under
the terms of the GNU General Public License as published by the Free Software
Foundation, either version 3 of the License, or (at your option) any later
version.

This program is distributed in the hope that it will be useful, but WITHOUT
ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.

You should have received a copy of the GNU General Public License along with
this program. If not, see <https://www.gnu.org/licenses/>.
"#
    );
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if let Some(shell) = args.completions {
        use clap::CommandFactory;
        let mut app = Args::command();
        generate(shell, &mut app, "md2nexus", &mut std::io::stdout());
        return Ok(());
    }

    if args.license {
        license();
        return Ok(());
    }

    if let Some(ref input) = args.input {
        if input.is_dir() {
            let output = if let Some(ref out) = args.output {
                out.clone()
            } else {
                PathBuf::from(".")
            };
            if !output.exists() {
                create_dir_all(&output)?;
            }
            handle_directory(input.clone(), output)?;
            return Ok(());
        }
    };

    // Not dealing with directories, only a single input file, which might be stdin.
    let mut data = String::new();
    if let Some(input) = args.input {
        let file = File::open(input.clone())?;
        let mut reader = BufReader::new(file);
        reader.read_to_string(&mut data)?;
    } else {
        let mut reader = BufReader::new(std::io::stdin());
        reader.read_to_string(&mut data)?;
    }
    let result = convert_buffer(&data)?;
    if let Some(outpath) = args.output {
        let mut output = File::create(outpath)?;
        write!(output, "{result}")?;
    } else {
        println!("{result}");
    }

    Ok(())
}
