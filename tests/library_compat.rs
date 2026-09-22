#![allow(deprecated)]

use purra::{Engine, Preset, Rule};

#[test]
fn version_1_scalar_api_keeps_its_original_behavior() {
    let engine = Engine::new([Rule::new('a', 'b'), Rule::new('b', 'c')]).unwrap();

    assert_eq!(engine.replacement_for('a'), Some('b'));
    assert_eq!(engine.replace("ab").text, "bc");

    let preset = Preset::parse_inline("…=.").unwrap();
    assert_eq!(preset.engine().replace("wait…").text, "wait.");
}
