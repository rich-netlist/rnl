// Pandoc template for the PDF of the RNL standard (pandoc -t typst, then typst compile).
// Variables: title, short-title, revision-label, typst-lang, and the font lists serif, sans and mono, which
// the build fills with the families that typst finds on the machine.

#let horizontalrule = line(start: (25%, 0%), end: (75%, 0%))
#show terms: it => {
  it.children
    .map(child => [
      #strong[#child.term]
      #block(inset: (left: 1.5em, top: -0.4em))[#child.description]
    ])
    .join()
}
#set smartquote(enabled: false)

#let serif = ($for(serif)$"$serif$", $endfor$)
#let sans = ($for(sans)$"$sans$", $endfor$)
#let mono = ($for(mono)$"$mono$", $endfor$)

// RNL code, coloured by the filter rnl.lua: a list of lines, each a list of (class, text).
// The colours of the project site (site/src/assets/intro.css, light palette).
#let colors = (
  k: rgb("#c2410c"), i: rgb("#1f4fd8"), n: rgb("#0f766e"), v: rgb("#7c3aed"), d: rgb("#ad1f78"),
  r: rgb("#975a06"), pt: rgb("#15803d"), s: rgb("#8a5a00"), o: rgb("#435368"), t: rgb("#8792a3"),
  c: rgb("#97a2b0"), h: rgb("#435368"), hi: rgb("#132033"),
)
#let rnlcode(label: none, lines) = block(
  fill: if label == none { luma(246) } else { rgb("#fef3f2") },
  stroke: 0.5pt + if label == none { luma(215) } else { rgb("#d92d20") },
  inset: (x: 7pt, y: 6pt), radius: 3pt, width: 100%, breakable: true,
)[
  #if label != none {
    text(font: sans, size: 7.5pt, weight: "bold", fill: rgb("#b42318"), label)
    linebreak()
  }
  #set text(font: mono, size: 7.6pt)
  #set par(justify: false, leading: 0.5em)
  #for (n, line) in lines.enumerate() {
    if n > 0 { linebreak() }
    if line.len() == 0 { h(0pt) }
    for seg in line {
      let cls = seg.at(0)
      let piece = text(fill: colors.at(cls, default: black), seg.at(1))
      if cls == "k" or cls == "o" { piece = strong(piece) }
      if cls == "t" or cls == "c" { piece = emph(piece) }
      piece
    }
  }
]

#set document(title: [$title$])
#set text(font: serif, size: 10pt, lang: "$typst-lang$")
#set par(justify: true, leading: 0.7em, spacing: 1em)
#set page(
  paper: "a4",
  margin: (x: 2.3cm, top: 2.6cm, bottom: 2.4cm),
  header: context {
    if counter(page).get().first() > 1 {
      set text(font: sans, size: 7.5pt, fill: luma(110))
      [$short-title$ #h(1fr) $revision-label$]
      v(-4pt)
      line(length: 100%, stroke: 0.4pt + luma(200))
    }
  },
  footer: context {
    if counter(page).get().first() > 1 {
      set text(font: sans, size: 8pt)
      h(1fr)
      counter(page).display()
      h(1fr)
    }
  },
)
#show raw: set text(font: mono, size: 0.88em)
#show raw.where(block: false): box.with(fill: luma(242), inset: (x: 2pt), outset: (y: 2.5pt), radius: 2pt)
#show raw.where(block: true): block.with(fill: luma(246), inset: 7pt, radius: 3pt, width: 100%, stroke: 0.5pt + luma(215))
#show heading: set text(font: sans)
#show heading: set block(above: 1.5em, below: 0.75em)
#show heading.where(level: 1): it => {
  pagebreak(weak: true)
  set text(size: 17pt)
  block(below: 1.1em, it.body)
}
#show heading.where(level: 2): set text(size: 12.5pt)
#show heading.where(level: 3): set text(size: 10.8pt)
#show link: set text(fill: rgb("#0b57d0"))
#set table(inset: 4.5pt, stroke: 0.4pt + luma(200))
#show table: set text(size: 8.6pt)
#show table: set par(justify: false)
#show table.cell: set align(start)  // pandoc wraps tables in align(center), which the cells would inherit
#show figure: set block(breakable: true)
#show outline.entry.where(level: 1): set text(font: sans, weight: "bold")

#page(header: none, footer: none)[
  #v(28%)
  #text(font: sans, size: 24pt, weight: "bold")[$title$]
  #v(0.8em)
  #text(font: sans, size: 11pt, fill: luma(80))[$revision-label$]
]

$body$
