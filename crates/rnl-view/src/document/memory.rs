//! Conservative retained-heap accounting for the viewer's optional drawing cache.
use std::collections::BTreeMap;
use std::mem::{size_of, size_of_val};

use rnl_runtime::interp::obj::Lit;
use rnl_runtime::markup::Run;
use rnl_runtime::model::{ItemView, SymbolOrigin, SymbolView};

use super::Drawings;

fn buffer<T>(v: &Vec<T>) -> usize {
    v.capacity() * size_of::<T>() + usize::from(v.capacity() > 0) * 32
}
fn string(s: &String) -> usize {
    s.capacity() + 32
}
fn optional(s: &Option<String>) -> usize {
    s.as_ref().map_or(0, string)
}
fn strings(v: &Vec<String>) -> usize {
    buffer(v) + v.iter().map(string).sum::<usize>()
}
fn map<K, V>(m: &BTreeMap<K, V>) -> usize {
    // Even one pair allocates a complete node. Charge a whole node per live pair,
    // including more slots/edges than Rust's current 11-pair, 12-edge nodes.
    m.len() * (16 * size_of::<(K, V)>() + 16 * size_of::<usize>() + 256)
}

fn literal(v: &Lit) -> usize {
    match v {
        Lit::Str(s) => s.as_str().len() + 64, // shared strings counted once per owner (overestimate)
        Lit::Arr(a) => buffer(a) + a.iter().map(literal).sum::<usize>(),
        _ => 0,
    }
}
fn style(m: &BTreeMap<String, Lit>) -> usize {
    map(m) + m.iter().map(|(k, v)| string(k) + literal(v)).sum::<usize>()
}
fn runs(v: &Vec<Run>) -> usize {
    buffer(v)
        + v.iter()
            .map(|r| match r {
                Run::Text { text, .. } => string(text),
                Run::Frac { num, den, .. } => runs(num) + runs(den),
            })
            .sum::<usize>()
}
fn item(it: &ItemView) -> usize {
    string(&it.id)
        + string(&it.object_id)
        + style(&it.style)
        + buffer(&it.order)
        + buffer(&it.host)
        + buffer(&it.pin_nets)
        + it.pin_nets.iter().map(strings).sum::<usize>()
        + map(&it.texts)
        + it.texts.iter().map(|(k, v)| string(k) + buffer(v) + v.iter().map(runs).sum::<usize>()).sum::<usize>()
}
fn symbol(s: &SymbolView) -> usize {
    string(&s.name)
        + optional(&s.package_version)
        + s.source.as_ref().map_or(0, |(a, b)| string(&a.file) + string(&b.file))
        + buffer(&s.labels)
        + s.labels
            .iter()
            .map(|l| optional(&l.key) + string(&l.text) + optional(&l.halign) + optional(&l.valign) + style(&l.style))
            .sum::<usize>()
        + buffer(&s.prims)
        + buffer(&s.pins)
        + buffer(&s.nested)
        + match &s.origin {
            SymbolOrigin::Package(p) => string(p),
            _ => 0,
        }
        + s.prims
            .iter()
            .map(|p| {
                buffer(&p.points)
                    + optional(&p.text)
                    + style(&p.style)
                    + optional(&p.halign)
                    + optional(&p.valign)
                    + optional(&p.params.fill)
                    + optional(&p.params.arrow)
            })
            .sum::<usize>()
        + s.pins.iter().map(|p| optional(&p.label) + strings(&p.terminals)).sum::<usize>()
}

impl Drawings {
    /// Retained bytes for cache budgeting, including vector capacities and nested heap data.
    /// Shared values and BTree node slack are deliberately overcounted. This excludes the
    /// active document/DOM and allocator working memory; it is not process resident memory.
    pub fn retained_bytes(&self) -> usize {
        size_of_val(self)
            + buffer(&self.levels)
            + self
                .levels
                .iter()
                .map(|l| {
                    buffer(&l.items)
                        + l.items.iter().map(item).sum::<usize>()
                        + buffer(&l.symbols)
                        + l.symbols.iter().map(symbol).sum::<usize>()
                        + strings(&l.names)
                })
                .sum::<usize>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rnl_runtime::interp::obj::WStr;

    #[test]
    fn singleton_maps_pay_for_an_entire_node() {
        let mut m = BTreeMap::new();
        assert_eq!(map::<String, Lit>(&m), 0);
        m.insert("fill".to_string(), Lit::Bool(true));
        assert!(map(&m) >= 11 * size_of::<(String, Lit)>() + 12 * size_of::<usize>());
    }

    #[test]
    fn nested_arrays_and_fraction_text_include_retained_capacity() {
        let text = "x".repeat(4096);
        let a = Lit::Arr(vec![Lit::Arr(vec![Lit::Str(WStr::new(&text, None))])]);
        assert!(literal(&a) > text.len() + 2 * size_of::<Lit>());
        let mut oversized = String::with_capacity(8192);
        oversized.push('a');
        let capacity = oversized.capacity();
        let content = vec![Run::Frac {
            num: vec![Run::Frac {
                num: vec![Run::Text { text: oversized, style: Default::default() }],
                den: vec![],
                style: Default::default(),
            }],
            den: vec![Run::Text { text, style: Default::default() }],
            style: Default::default(),
        }];
        assert!(runs(&content) > capacity + 4096);
    }
}
