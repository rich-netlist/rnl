# skills

Skill packages for AI agents, in the [Agent Skills](https://agentskills.io/specification) format: a folder with a `SKILL.md`, whose front matter names the skill and says when to use it, and the files it points to. Agents that read such folders load a skill when a task matches it. A folder under `skills/` can be copied as it is into an agent's skills directory (`~/.claude/skills/` or a project's `.claude/skills/` for Claude Code); the project site publishes it as a zip and as plain-text files, and tells agents how to fetch it ([For agents](https://rich-netlist.github.io/rnl/en/agents.html), [`llms.txt`](https://rich-netlist.github.io/rnl/llms.txt)).

| Path | Content |
| --- | --- |
| [`rnl/`](rnl/SKILL.md) | The RNL skill: reading, writing, checking and drawing RNL 0.3. `SKILL.md` is what the agent reads first; `references/` holds the short guide to the language, the reference to the standard packages and the notes on the tools; `examples/` holds netlists that read without errors; `LICENSE.md` and `LICENSES/` the terms. |

## Why here

The package is not a web page (`site/`), and not the standard (`spec/`): it is a folder an agent installs as it is, and `skills/<name>/` is where such folders are kept in repositories that publish them, so that a person can copy one and a tool can find it. The site's pages are built from it, not the other way round.

The package is in English only: its files, the guide and the reference to the packages. The site's pages and `llms.txt` are in both languages.

## What is written and what is generated

| File of `skills/rnl/` | Source |
| --- | --- |
| `SKILL.md`, `references/tools.md`, `examples/*`, `LICENSE.md` | Written by hand. |
| `references/language.md` | A copy of [`spec/rnl-0.3-guide.md`](../spec/rnl-0.3-guide.md), the short guide to the language, which the page of the standard ([`scripts/spec`](../scripts/spec/README.md)) also shows, with a line that it is not the standard. The Chinese version of the guide, `spec/rnl-0.3-guide.zh.md`, is only on the Chinese page of the standard: the package is in English. |
| `references/packages.md` | Written from the package files in [`libraries/`](../libraries/README.md), from `rnl catalog` and from what the gallery says each package is for ([`site/src/assets/gallery.js`](../site/src/assets/gallery.js)), so that it is in step with them. |
| `LICENSES/` | Copies of the licence texts in [`LICENSES/`](../LICENSES) that the package's files are under. |

```bash
python scripts/skill/build.py                  # write the generated files (needs the rnl command)
python scripts/skill/build.py --check          # check that the package is whole and in step
python scripts/skill/build.py --site site/dist # add the zip, the plain-text files and llms.txt to a built site
```

`--check` fails when a generated file is not what its sources give, when `SKILL.md` has no valid front matter or a link of the package leads nowhere, when a file of `references/` or `examples/` is not linked from `SKILL.md`, when the page for agents or `llms.txt` do not name every file of the package, and when a netlist in the package does not read: every code block marked `spice` or `spectre` in a Markdown file, and every file in `examples/`, is read by `rnl check`, and must have no errors and no warnings; a block marked `error` must have an error. It needs the `rnl` command (`cargo build -p rnl-cli`, found in `target/`, or given with `--rnl PATH` or `$RNL`), as [`skill.yml`](../.github/workflows/skill.yml) has it. `--site` needs nothing but Python 3 and git; [`pages.yml`](../.github/workflows/pages.yml) runs it after the pages of the standard.

What `--site DIR` writes into the built site:

| Path in the site | Content |
| --- | --- |
| `skill/rnl-skill.zip` | The package as a zip with the folder `rnl/` in it, so that unpacking it into a skills directory leaves `rnl/SKILL.md`; the same bytes for the same files. |
| `skill/rnl/…` | Each file of the package as it is, readable as plain text. |
| `skill/manifest.json`, `skill/SHA256SUMS.txt` | Each file with its address, size and SHA-256, and the hashes of the zip and the files. |
| `llms.txt`, `zh/llms.txt` | The plain-text entry for agents, in English and in Chinese, from [`site/src/llms.txt`](../site/src/llms.txt) and [`site/src/llms.zh.txt`](../site/src/llms.zh.txt). |

## Licences

The files are under the licences of the project's other files of the same kind ([`REUSE.toml`](../REUSE.toml)): `SKILL.md`, `references/tools.md` and `LICENSE.md` MIT; the guide, which is the standard's text, CC-BY-4.0; the reference to the packages and the examples CC0-1.0. `skills/rnl/LICENSE.md` says so for the people who get the folder alone.
