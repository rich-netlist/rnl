# 0016. The English terms are the only normative ones; each language keeps its own glossary

- Date: 2026-10-26
- Status: proposed

## Context

[Decision 0015](0015-english-is-the-only-core-language.md) made English the project's only core language and every other language a translation of it. It left one place where another language still had a standing of its own: the Chinese standard. Its chapters and the alias package `rnl-zh` give each term of RNL a Chinese name, such as 包 for package, 用例 for usage, 元件 for instance, 导线 for wire and 代称 for alias. The Chinese translation of the site and the viewers followed those names.

Then Traditional Chinese was translated, by Gemini, reviewed by GPT. GPT listed 222 places where the Simplified and the Traditional translations name the same English differently. Most of them are ordinary differences between the mainland and Taiwan, such as 查看器 and 檢視器. But 29 are terms the Chinese standard names, and Traditional Chinese used Taiwan's words for them: 套件, 用法, 實例, 連線, 別名.

The question put to the owner was whether Traditional Chinese should write the standard's Chinese terms in Traditional characters or use Taiwan's words. The owner answered with a principle instead:

- the standard's terms are the English ones;
- the Chinese standard's terms should have no special standing;
- every language, Simplified and Traditional Chinese included, may keep a set of terms of its own, which does not enter the standard and serves only to communicate;
- the English is shown beside a translation: as a mapping at the start of each translated document, or in brackets after the terms, or on hover.

So Traditional Chinese keeps its own terms, which have nothing to do with the Simplified ones. The owner made four exceptions, for Traditional: usage is 用例, as 用法 reads too easily as "way of use"; device is 器件, so that it does not clash with the Simplified 元件; and bundle is 疊畫, as a bundle is not a set in general. For Simplified, selection is 筛选.

## Decision

1. **The English terms are the only normative terminology of RNL.**
   - A translation of the standard, Chinese included, is a reference translation, with the same standing as any later one, and its terms are renderings of the English.
   - Where a translation and the English differ, the English prevails, as point 7 of decision 0015 says of the standard's text.
   - The Chinese standard's own rules, its fixed phrases of Annex A among them, stay as they are until 0.4 (decision 0015, step 4).
2. **Each language keeps a glossary**, `i18n/<lang>/glossary.json`. It holds one entry per English term and sense:

   | Field | What it holds |
   | --- | --- |
   | `en` | the English term |
   | `sense` | the sense, where the term has several |
   | `term` | the language's rendering |
   | `avoid` | variants not to use |
   | `kind` | `rnl` (a term of RNL), `circuit` (of circuit design), `command` (a command or label of an interface) or `general` |
   | `note` | an optional note |

   - A glossary is not part of the standard. It is a convention, for those who translate RNL and those who talk about it.
   - The glossaries of two languages are independent: Simplified and Traditional Chinese each have their own.
   - A first draft says `"draft": true` until someone who knows the language has reviewed it.
3. **Alias packages stay as they are.** An alias package is code (16.5): files are written with it, and its spellings do not change with a glossary. A language that has one uses its words in its glossary wherever it can. Where the owner chose otherwise, such as 筛选 for selection while `rnl-zh` spells `@SELECT` as `@选出`, the glossary says so.
4. **The English is shown beside a translation.**
   - **At the top of every page.** Every translated page of the site says, under its top bar, that the English terms are the normative ones, with a link to the language's glossary. It says so in English, as every notice about translation does (decision 0015, point 8). A translated document, such as a chapter of the standard, the short guide or the principles, says the same at its top when it is next updated.
   - **At a term's first occurrence.** In a page or a document, the first occurrence of an RNL term carries its English in brackets, as in 用例（usage）. Translations add this as they are revised, starting with the next round of each language.
   - **On hover.** Showing the English when the pointer is on a term is left for later.
5. **The site has a glossary page per language**, `glossary.html`. It shows the language's glossary by kind; the English page lists the languages' glossaries.
6. **A check reports variants.** `python scripts/i18n/status.py` reports a catalog text whose English contains a glossary term and whose translation uses one of the variants listed under `avoid`. Like a missing or stale translation, this fails no check. A malformed glossary is a format error and fails the check, like a malformed catalog.
7. **The first glossaries.**
   - Simplified and Traditional Chinese come from the owner's choices on GPT's list. Traditional Chinese uses Taiwan's words, with the four exceptions above. Each language keeps one rendering where it used two for one sense.
   - The other languages start from drafts taken from their catalogs as they are.

## Consequences

- This amends points 3 and 7 of decision 0015. Point 3 already named a language's glossary among the files of `i18n/<lang>/`; this record gives its format and its standing. Point 7 already made the English text of the standard normative; the terms now are too.
- The catalogs of Simplified and Traditional Chinese are brought in line with their glossaries in one pull request per language.
- A work order for a translation names the language's glossary. A translator follows it, and proposes a change to it in the same pull request when a term seems wrong.
- Two translations may now name one English term differently, and that is not an error. A reader who needs to compare two languages compares each with the English.
- The Chinese standard's chapters keep their present terms until the language line revises them. Whether they then follow the Simplified glossary is decided with 0.4.
