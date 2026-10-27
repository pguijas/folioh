//! Files the plugin registry puts in the workspace: component copies under
//! `components/__folioh_components/`, `lib/*.ts` data modules, the
//! `mdx-components.tsx` injection and `app/**/page.tsx` views.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use folioh_config::canonicalize_lenient;
use serde_json::{Map, Value};

use crate::fs::write_text_if_changed;
use crate::json;
use crate::re;
use crate::{Result, SiteError};
use folioh_plugins::{
    has_component_entry, import_statements, strip_import_statements, strip_js_comments,
    ComponentDefinition, ComponentOrigin, ExtensionRegistry, LayoutDefinition, ViewDefinition,
};

/// Emits registry extensions into a build workspace.
pub struct ExtensionEmitter {
    /// The workspace the files land in.
    pub build_dir: PathBuf,
    /// `false` for a `template.path` frontend, which ships no builtin files.
    pub inject_builtins: bool,
    /// Anchors relative component `source_path` values.
    pub project_dir: PathBuf,
}

fn io(path: &Path) -> impl Fn(std::io::Error) -> SiteError + '_ {
    move |e| SiteError::io(path, e)
}

fn component_import_line(component: &ComponentDefinition) -> String {
    let name = &component.name;
    let imported = component.imported_name();
    if imported == "default" {
        format!("import {name} from \"{}\"", component.import_path)
    } else if imported == name {
        format!("import {{ {name} }} from \"{}\"", component.import_path)
    } else {
        format!(
            "import {{ {imported} as {name} }} from \"{}\"",
            component.import_path
        )
    }
}

fn layout_import_line(layout: &LayoutDefinition) -> String {
    if layout.export_name == "default" {
        format!("import Layout from \"{}\"", layout.import_path)
    } else {
        format!(
            "import {{ {} }} from \"{}\"",
            layout.export_name, layout.import_path
        )
    }
}

impl ExtensionEmitter {
    /// Copy components, write data modules, inject `mdx-components.tsx`, write views.
    pub fn apply(&self, registry: &ExtensionRegistry) -> Result<()> {
        let ledger = self.build_dir.join(".folioh-extension-views.json");
        let previous = crate::generated::load(&ledger)?;
        self.copy_components(registry)?;
        for module in registry.data_modules.values() {
            self.write_data_module(module)?;
        }
        self.inject_mdx_components(registry)?;
        let mut current = BTreeMap::new();
        for view in registry.views.values() {
            self.write_view(view, registry)?;
            let path = self.app_route_path(&view.path)?;
            if let Some(digest) = crate::generated::digest(&path)? {
                current.insert(view.path.clone(), digest);
            }
        }
        for (route, digest) in previous {
            if current.contains_key(&route) {
                continue;
            }
            let path = self.app_route_path(&route)?;
            if crate::generated::digest(&path)?.as_ref() == Some(&digest) {
                std::fs::remove_file(&path).map_err(io(&path))?;
            }
        }
        write_text_if_changed(&ledger, &json::pretty(&current)).map_err(io(&ledger))?;
        Ok(())
    }

    fn copy_components(&self, registry: &ExtensionRegistry) -> Result<()> {
        for component in registry.components.values() {
            let Some(source_path) = &component.source_path else {
                continue;
            };
            let source = if source_path.is_absolute() {
                source_path.clone()
            } else {
                self.project_dir.join(source_path)
            };
            if !source.is_file() {
                return Err(SiteError::NotFound(format!(
                    "Component source not found: {}",
                    source.display()
                )));
            }
            let target = self.component_target(component, &source)?;
            std::fs::create_dir_all(target.parent().unwrap()).map_err(io(&target))?;
            std::fs::copy(&source, &target).map_err(io(&target))?;
        }
        Ok(())
    }

    fn component_target(&self, component: &ComponentDefinition, source: &Path) -> Result<PathBuf> {
        let target = if let Some(rest) = component.import_path.strip_prefix("@/") {
            let mut rel = PathBuf::from(rest);
            if rel.extension().is_none() {
                if let Some(ext) = source.extension() {
                    rel.set_extension(ext);
                }
            }
            canonicalize_lenient(&self.build_dir.join(rel))
        } else {
            canonicalize_lenient(
                &self
                    .build_dir
                    .join("components")
                    .join("__folioh_components")
                    .join(source.file_name().unwrap_or_default()),
            )
        };
        if !target.starts_with(canonicalize_lenient(&self.build_dir)) {
            return Err(SiteError::Value(format!(
                "Component import path would write outside build directory: {}",
                component.import_path
            )));
        }
        Ok(target)
    }

    fn write_data_module(&self, module: &folioh_plugins::DataModuleDefinition) -> Result<()> {
        let module_path = if module.module_path.is_empty() {
            format!("__folioh_data/{}", module.name)
        } else {
            module.module_path.clone()
        };
        let rel = PathBuf::from(&module_path);
        if rel.is_absolute() || rel.components().any(|c| matches!(c, Component::ParentDir)) {
            return Err(SiteError::Value(format!(
                "Data module path must stay inside lib: {module_path}"
            )));
        }
        let rel = if rel.extension().map(|e| e != "ts").unwrap_or(true) {
            rel.with_extension("ts")
        } else {
            rel
        };
        let lib_dir = canonicalize_lenient(&self.build_dir.join("lib"));
        let target = canonicalize_lenient(&lib_dir.join(rel));
        if !target.starts_with(&lib_dir) {
            return Err(SiteError::Value(format!(
                "Data module path would write outside lib: {module_path}"
            )));
        }
        std::fs::create_dir_all(target.parent().unwrap()).map_err(io(&target))?;
        let mut type_source = module.type_source.clone();
        if !type_source.is_empty() && !type_source.ends_with('\n') {
            type_source.push('\n');
        }
        let annotation = if module.type_annotation.is_empty() {
            String::new()
        } else {
            format!(": {}", module.type_annotation)
        };
        let content = format!(
            "{type_source}export const {}{annotation} = {}\n",
            module.export_name,
            json::pretty(&module.data)
        );
        write_text_if_changed(&target, &content).map_err(io(&target))?;
        Ok(())
    }

    fn inject_mdx_components(&self, registry: &ExtensionRegistry) -> Result<()> {
        let path = self.build_dir.join("mdx-components.tsx");
        if !path.exists() {
            return Ok(());
        }
        let candidates: Vec<&ComponentDefinition> = registry
            .components
            .values()
            .filter(|c| {
                c.expose_mdx && (self.inject_builtins || c.origin != ComponentOrigin::Builtin)
            })
            .collect();
        if candidates.is_empty() {
            return Ok(());
        }
        let mut content = std::fs::read_to_string(&path).map_err(io(&path))?;
        let code = strip_js_comments(&content);
        let imports_text = import_statements(&code).join("\n");
        let entry_code = strip_import_statements(&code);
        let pending: Vec<&ComponentDefinition> = candidates
            .into_iter()
            .filter(|c| !has_component_entry(&entry_code, &c.name))
            .collect();
        let import_lines: Vec<String> = pending
            .iter()
            .filter(|c| !re(&format!(r"\b{}\b", regex::escape(&c.name))).is_match(&imports_text))
            .map(|c| component_import_line(c))
            .collect();
        let entry_lines: Vec<String> = pending.iter().map(|c| format!("    {},", c.name)).collect();
        if import_lines.is_empty() && entry_lines.is_empty() {
            return Ok(());
        }
        if !import_lines.is_empty() {
            let imports = import_lines.join("\n");
            content = if content.contains("__FOLIOH_COMPONENT_IMPORTS__") {
                content.replace("// __FOLIOH_COMPONENT_IMPORTS__", &imports)
            } else {
                format!("{imports}\n{content}")
            };
        }
        if !entry_lines.is_empty() {
            let entries = entry_lines.join("\n");
            if content.contains("__FOLIOH_COMPONENT_ENTRIES__") {
                content = content.replace("    // __FOLIOH_COMPONENT_ENTRIES__", &entries);
            } else if content.contains("...components,") {
                content = content.replacen(
                    "    ...components,",
                    &format!("{entries}\n    ...components,"),
                    1,
                );
            }
        }
        write_text_if_changed(&path, &content).map_err(io(&path))?;
        Ok(())
    }

    fn app_route_path(&self, route: &str) -> Result<PathBuf> {
        let normalized = route.trim_matches('/');
        let app_dir = canonicalize_lenient(&self.build_dir.join("app"));
        let target = if normalized.is_empty() {
            app_dir.join("page.tsx")
        } else {
            let rel = PathBuf::from(normalized);
            if rel.components().any(|c| matches!(c, Component::ParentDir)) {
                return Err(SiteError::Value(format!(
                    "Route would write outside app directory: {route}"
                )));
            }
            canonicalize_lenient(&app_dir.join(rel).join("page.tsx"))
        };
        if !target.starts_with(&app_dir) {
            return Err(SiteError::Value(format!(
                "Route would write outside app directory: {route}"
            )));
        }
        Ok(target)
    }

    fn write_view(&self, view: &ViewDefinition, registry: &ExtensionRegistry) -> Result<()> {
        let depth = view
            .path
            .trim_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .count();
        let path_to_root = if depth == 0 {
            ".".to_string()
        } else {
            "../".repeat(depth).trim_end_matches('/').to_string()
        };
        let page_path = self.app_route_path(&view.path)?;
        std::fs::create_dir_all(page_path.parent().unwrap()).map_err(io(&page_path))?;
        let layout = registry
            .layouts
            .get(&view.layout)
            .ok_or_else(|| SiteError::Value(format!("Unknown layout: {}", view.layout)))?;
        let mut imports = vec![layout_import_line(layout)];
        let mut used: Vec<&str> = view
            .slots
            .values()
            .flatten()
            .map(|b| b.component.as_str())
            .collect();
        used.sort();
        used.dedup();
        for name in used {
            let component = registry
                .components
                .get(name)
                .ok_or_else(|| SiteError::Value(format!("Unknown component: {name}")))?;
            imports.push(component_import_line(component));
        }
        let mut layout_props: Map<String, Value> = view.props.clone();
        if !view.title.is_empty() {
            layout_props
                .entry("title")
                .or_insert_with(|| Value::String(view.title.clone()));
        }
        layout_props.insert("pathToRoot".into(), Value::String(path_to_root));
        let mut constants = vec![format!(
            "const layoutProps = {}",
            json::pretty(&layout_props)
        )];
        let mut blocks = Vec::new();
        let mut index = 0;
        for (slot, slot_blocks) in &view.slots {
            if slot != "main" {
                blocks.push(format!("        {{/* Slot: {slot} */}}"));
            }
            for block in slot_blocks {
                if block.props.is_empty() {
                    blocks.push(format!("        <{} />", block.component));
                } else {
                    constants.push(format!(
                        "const block{index}Props = {}",
                        json::pretty(&block.props)
                    ));
                    blocks.push(format!(
                        "        <{} {{...block{index}Props}} />",
                        block.component
                    ));
                }
                index += 1;
            }
        }
        let body = if blocks.is_empty() {
            "        {null}".to_string()
        } else {
            blocks.join("\n")
        };
        let metadata = if view.title.is_empty() {
            String::new()
        } else {
            format!(
                "export const metadata = {{ title: {} }}\n\n",
                json::string(&view.title)
            )
        };
        let content = format!(
            "{}\n\n{}\n\n{metadata}export default function FoliohExtensionView() {{\n  return (\n    <{layout} {{...layoutProps}}>\n{body}\n    </{layout}>\n  )\n}}\n",
            imports.join("\n"),
            constants.join("\n\n"),
            layout = layout.export_name
        );
        write_text_if_changed(&page_path, &content).map_err(io(&page_path))?;
        Ok(())
    }
}
