//! Complete annotation layout transport; unlike the public snapshot this retains text flags.
use crate::{
    data,
    json::{Node, Result},
};
use rnl_runtime::{
    interp::obj::{PrimParams, Shape},
    model::{AnnotationGraphic, AnnotationTextRef, AnnotationView, Layer, Style, SymbolPrim},
    problem::AnnotationInfo,
};
use serde_json::{json, Value};

fn style_value(style: &Style) -> Value {
    data::pairs_value(&style.iter().map(|(k, v)| (k.clone(), v.clone())).collect::<Vec<_>>())
}
fn style(n: Node<'_>) -> Result<Style> {
    let pairs = data::pairs(n.clone())?;
    let mut result = Style::new();
    for (key, value) in pairs {
        if result.insert(key, value).is_some() {
            return Err(n.error("duplicate style key"));
        }
    }
    Ok(result)
}
fn graphic_value(g: &AnnotationGraphic) -> Value {
    let p = &g.primitive;
    json!({"order":p.order,"text":p.text,"text_name_formats":p.text_name_formats,"modern_text":p.modern_text,
        "shape":p.shape.as_str(),"points":p.points.iter().map(|(x,y)|[*x,*y]).collect::<Vec<_>>(),
        "params":{"width":p.params.width,"height":p.params.height,"radius":p.params.radius,"fill":p.params.fill,"arrow":p.params.arrow},
        "style":style_value(&p.style),"halign":p.halign,"valign":p.valign,"rotate":p.rotate,"fix_rotate":p.fix_rotate,"upright":p.upright,
        "lines":g.lines.iter().map(|l|l.iter().map(data::run_value).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "text_refs":g.text_refs.iter().map(|r|match r {AnnotationTextRef::X=>"x", AnnotationTextRef::Y=>"y", AnnotationTextRef::Empty=>"empty"}).collect::<Vec<_>>()})
}
fn graphic(n: Node<'_>) -> Result<AnnotationGraphic> {
    let params = n.field("params")?;
    let optnum = |name| params.field(name)?.nullable(|n| n.number());
    let optstr = |name| data::optional_string(params.field(name)?);
    let shape = Shape::parse(n.field("shape")?.string()?).ok_or_else(|| n.error("unknown annotation shape"))?;
    Ok(AnnotationGraphic {
        primitive: SymbolPrim {
            order: n.field("order")?.index()?,
            text: data::optional_string(n.field("text")?)?,
            text_name_formats: n.field("text_name_formats")?.boolean()?,
            modern_text: n.field("modern_text")?.boolean()?,
            shape,
            points: n.field("points")?.list(|n| n.point())?,
            params: PrimParams {
                width: optnum("width")?,
                height: optnum("height")?,
                radius: optnum("radius")?,
                fill: optstr("fill")?,
                arrow: optstr("arrow")?,
            },
            style: style(n.field("style")?)?,
            halign: data::optional_string(n.field("halign")?)?,
            valign: data::optional_string(n.field("valign")?)?,
            rotate: n.field("rotate")?.nullable(|n| n.number())?,
            fix_rotate: n.field("fix_rotate")?.boolean()?,
            upright: n.field("upright")?.nullable(|n| n.boolean())?,
        },
        lines: n.field("lines")?.list(|line| line.list(|run| data::run(run, 0)))?,
        text_refs: n.field("text_refs")?.list(|r| {
            Ok(match r.string()? {
                "x" => AnnotationTextRef::X,
                "y" => AnnotationTextRef::Y,
                "empty" => AnnotationTextRef::Empty,
                _ => return Err(r.error("unknown text reference")),
            })
        })?,
    })
}
pub(crate) fn value(a: &AnnotationInfo) -> Value {
    let v = &a.view;
    json!({"object":a.object,"object_id":v.object_id,"source_kind":v.source_kind,"source":v.source,"anchor_kind":v.anchor_kind,
        "anchor_object":v.anchor_object,"anchor":[v.anchor.0,v.anchor.1],"rotate":v.rotate,"mirror":v.mirror,
        "style":style_value(&v.style),"own_style":style_value(&v.own_style),"layer":v.layer.as_str(),"order":v.order,
        "graphics":v.graphics.iter().map(graphic_value).collect::<Vec<_>>()})
}
pub(crate) fn decode(n: Node<'_>) -> Result<AnnotationInfo> {
    let source_kind = match n.field("source_kind")?.string()? {
        "symbol" => "symbol",
        "definition" => "definition",
        "usage" => "usage",
        _ => return Err(n.error("unknown annotation source")),
    };
    let anchor_kind = match n.field("anchor_kind")?.string()? {
        "independent" => "independent",
        "follow" => "follow",
        _ => return Err(n.error("unknown annotation anchor")),
    };
    let layer = match n.field("layer")?.string()? {
        "bottom" => Layer::Bottom,
        "middle" => Layer::Middle,
        "top" => Layer::Top,
        _ => return Err(n.error("unknown layer")),
    };
    Ok(AnnotationInfo {
        object: n.field("object")?.nullable(|n| n.index())?,
        view: AnnotationView {
            object_id: n.field("object_id")?.string()?.into(),
            source_kind,
            source: n.field("source")?.string()?.into(),
            anchor_kind,
            anchor_object: data::optional_string(n.field("anchor_object")?)?,
            anchor: n.field("anchor")?.point()?,
            rotate: n.field("rotate")?.number()?,
            mirror: n.field("mirror")?.boolean()?,
            style: style(n.field("style")?)?,
            own_style: style(n.field("own_style")?)?,
            layer,
            order: n.field("order")?.list(|n| n.number())?,
            graphics: n.field("graphics")?.list(graphic)?,
        },
    })
}
