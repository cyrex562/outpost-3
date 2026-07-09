//! Compile a world's bound packs' IR content into [`ComponentRecord`]s.
//!
//! A pack's IR-authored content lives under `<packs_root>/<pack-id>/content/ir/`
//! (recursively, multi-document YAML). Legacy content (the `creatures:` lists,
//! bare item lists, etc.) lives elsewhere in the pack and is *not* compiled here —
//! only the `ir/` tree is the IR authoring surface, so legacy files are never
//! mis-parsed. Documents from all bound packs are compiled together (in pack
//! order) so references between them resolve within the batch.
//!
//! This is what makes authored IR content go live at world creation: the result
//! is upserted into the world's `ir_records`, which the runtime reads on load.

use std::path::Path;

use crate::compiler::{compile, Catalogs, VerbRegistry};
use crate::content::loader::load_documents;
use crate::ir::ComponentRecord;

/// Compile the `content/ir/` trees of the given packs (under `packs_root`) into
/// IR records. Returns the compiled records and any diagnostics (load failures,
/// syntax errors, unimplemented primitives) as human-readable strings — a bad
/// pack never aborts the caller (e.g. world creation).
pub fn compile_pack_ir(packs_root: &Path, pack_ids: &[String]) -> (Vec<ComponentRecord>, Vec<String>) {
    let mut docs = Vec::new();
    let mut diagnostics = Vec::new();

    for pack_id in pack_ids {
        let ir_dir = packs_root.join(pack_id).join("content").join("ir");
        if !ir_dir.is_dir() {
            continue;
        }
        match load_documents(&ir_dir) {
            Ok(mut pack_docs) => docs.append(&mut pack_docs),
            Err(e) => diagnostics.push(format!("pack '{pack_id}': failed to load IR content: {e}")),
        }
    }

    if docs.is_empty() {
        return (Vec::new(), diagnostics);
    }

    let report = compile(&docs, &VerbRegistry::builtin(), &Catalogs::default());
    for d in &report.syntax_errors {
        diagnostics.push(format!("pack IR syntax error: {}{}", d.message, fmt_loc(&d.location)));
    }
    for d in &report.unimplemented {
        diagnostics.push(format!(
            "pack IR unimplemented: {}{}",
            d.message,
            fmt_loc(&d.location)
        ));
    }
    (report.records, diagnostics)
}

fn fmt_loc(location: &Option<String>) -> String {
    match location {
        Some(loc) => format!(" [{loc}]"),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Build a temp packs_root with one pack carrying IR content + a legacy file.
    fn fixture_packs_root(dir: &Path) {
        let ir = dir.join("demo-pack").join("content").join("ir");
        fs::create_dir_all(&ir).unwrap();
        fs::write(
            ir.join("envenom.yaml"),
            r#"
item:
  id: envenomed_dagger
  name: Envenomed Dagger
  damage: 1d4
  grants_traits: [envenom_on_hit]
---
status_effect:
  id: poisoned
  name: Poisoned
  provides_tags: [poisoned]
  stacking: replace
"#,
        )
        .unwrap();
        // A legacy file OUTSIDE ir/ must be ignored entirely.
        let legacy = dir.join("demo-pack").join("content").join("creatures");
        fs::create_dir_all(&legacy).unwrap();
        fs::write(
            legacy.join("beasts.yaml"),
            "creatures:\n  - id: wolf\n    name: Wolf\n    hd: 1\n    ac: 13\n",
        )
        .unwrap();
    }

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let mut base = std::env::temp_dir();
        base.push(format!("harsh_pack_compile_{tag}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        base
    }

    #[test]
    fn compiles_ir_dir_and_ignores_legacy() {
        let root = temp_dir("ok");
        fixture_packs_root(&root);
        let (records, diags) = compile_pack_ir(&root, &["demo-pack".to_string()]);
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
        assert_eq!(records.len(), 2, "item + status compiled, legacy ignored");
        let types: Vec<&str> = records.iter().map(|r| r.component_type()).collect();
        assert!(types.contains(&"item"));
        assert!(types.contains(&"status_effect"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_ir_dir_is_a_no_op() {
        let root = temp_dir("empty");
        fs::create_dir_all(root.join("bare-pack").join("content")).unwrap();
        let (records, diags) = compile_pack_ir(&root, &["bare-pack".to_string()]);
        assert!(records.is_empty());
        assert!(diags.is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn syntax_errors_surface_as_diagnostics_without_records() {
        let root = temp_dir("bad");
        let ir = root.join("bad-pack").join("content").join("ir");
        fs::create_dir_all(&ir).unwrap();
        // Missing required `damage` on a creature → compile error (atomic: no records).
        fs::write(ir.join("c.yaml"), "creature:\n  id: x\n  name: X\n").unwrap();
        let (records, diags) = compile_pack_ir(&root, &["bad-pack".to_string()]);
        assert!(records.is_empty());
        assert!(!diags.is_empty());
        let _ = fs::remove_dir_all(&root);
    }
}
