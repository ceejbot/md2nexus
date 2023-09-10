# md2nexus

This is a command-line tool for converting single files or directories of files from Markdown to valid [Nexusmods](https://nexusmods.com) [bbcode](https://wiki.nexusmods.com/index.php/Formating_and_BBCode_in_Descriptions). The Nexus bbcode editors are hell to use if you're writing any complex documentation.

To install, you can download a MacOS, Linux, or Windows prebuilt executable from [the releases](https://github.com/ceejbot/md2nexus/releases/latest). Or you can use Homebrew:

```shell
brew tap ceejbot/tap
brew install md2nexus
```

If you have rust installed and prefer to build from source:

```shell
gh repo clone ceejbot/md2nexus
cd md2nexus
cargo build --release

# once installed:
md2nexus --help
```

## Usage

Simplest possible usage:

```shell
cat docs.md | md2nexus > docs.bbcode
# or
cat docs.md | md2nexus | pbcopy
# and then paste into the nexus editor
```

If you don't specify an input file, `md2nexus` reads stdin. If you don't specify an output file, `md2nexus` writes the converted text to stdout. It prints nothing to stdout other than program output.

If the input option is a _directory_, `md2nexus` converts all markdown files in the input directory and writes them to the output option, which must be a directory. If you do not specify an output directory, it writes them to the current directory. It prints the names of converted files to stdout.

## Notes on conversion

Markdown is far more expressive than Nexus bbcode, which is a stunted bbcode variation. This tool converts some Markdown concepts to code blocks to make them work. For example, to handle Markdown tables, this tool uses a table output formatter intended for terminal usage and wraps it all in a code block.

Github-flavored markdown is supported. [MDX](https://mdxjs.com) is not supported at all, as Nexus bbcode doesn't allow components or javascript or indeed any potentially dangerous user-generated content.

In-document references such as footnotes or link references are not converted into usable links, but they are emitted into the text. [^note1]

## Full usage

```text
$ md2nexus -h
A command-line tool to convert gfm markdown to NexusMods-flavored bbcode

Usage: md2nexus [OPTIONS]

Options:
  -i, --input <PATHNAME>           Path to an input file or directory.
                                   If omitted, input is read from stdin.
  -o, --output <PATHNAME>          Path to an output file or directory
  -c, --completions <COMPLETIONS>  Emit completion data for your preferred shell [possible values:
                                   bash, elvish, fish, powershell, zsh]
      --license                    Print GPL-3.0 license information
  -h, --help                       Print help (see more with '--help')
  -V, --version                    Print version
```

## Implementation notes

The heavy lifting is done by the [markdown](https://lib.rs/crates/markdown) crate, which generates an AST for input documents. The tool visits the tree's nodes and generates appropriate bbcode.

[^note1]: This is an example footnote. It's better than nothing?

## LICENSE

GPL-3.0.
