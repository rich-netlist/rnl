//! placeholder until the renderer lands
use rnl_runtime::model::LevelView;
#[derive(Default)]
pub struct RenderOptions {}
pub fn svg(_lv: &LevelView, _o: &RenderOptions) -> String { String::new() }
