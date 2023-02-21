use farmfe_core::{
  plugin::{PluginAnalyzeDepsHookResultEntry, ResolveKind},
  swc_common::Mark,
  swc_ecma_ast::{CallExpr, ExportAll, Expr, Lit, Module, ModuleDecl, ModuleItem, NamedExport},
};

use farmfe_toolkit::{
  script::is_commonjs_require,
  swc_ecma_visit::{Visit, VisitWith},
};

pub struct DepsAnalyzer<'a> {
  ast: &'a Module,
  deps: Option<Vec<PluginAnalyzeDepsHookResultEntry>>,
  unresolved_mark: Mark,
}

impl<'a> DepsAnalyzer<'a> {
  pub fn new(ast: &'a Module, unresolved_mark: Mark) -> Self {
    Self {
      ast,
      deps: None,
      unresolved_mark,
    }
  }

  pub fn analyze_deps(&mut self) -> Vec<PluginAnalyzeDepsHookResultEntry> {
    self.ast.visit_with(self);
    self.deps.take().unwrap_or(vec![])
  }

  fn insert_dep(&mut self, dep: PluginAnalyzeDepsHookResultEntry) {
    if let Some(deps) = &mut self.deps {
      deps.push(dep);
    } else {
      self.deps.replace(vec![dep]);
    }
  }
}

impl<'a> Visit for DepsAnalyzer<'a> {
  fn visit_module_item(&mut self, n: &ModuleItem) {
    match n {
      ModuleItem::ModuleDecl(decl) => {
        match decl {
          ModuleDecl::Import(import) => {
            self.insert_dep(PluginAnalyzeDepsHookResultEntry {
              source: import.src.value.to_string(),
              kind: ResolveKind::Import,
            });
          }
          ModuleDecl::ExportAll(ExportAll { src, .. }) => {
            self.insert_dep(PluginAnalyzeDepsHookResultEntry {
              source: src.value.to_string(),
              kind: ResolveKind::ExportFrom,
            });
          }
          ModuleDecl::ExportNamed(NamedExport { src, .. }) => {
            if let Some(src) = src {
              self.insert_dep(PluginAnalyzeDepsHookResultEntry {
                source: src.value.to_string(),
                kind: ResolveKind::ExportFrom,
              });
            }
          }
          _ => { /* others are ignored */ }
        }
      }
      _ => {
        n.visit_children_with(self);
      }
    }
  }

  fn visit_call_expr(&mut self, call_expr: &CallExpr) {
    if call_expr.args.len() == 1 && is_commonjs_require(self.unresolved_mark, call_expr) {
      if let box Expr::Lit(Lit::Str(str)) = &call_expr.args[0].expr {
        self.insert_dep(PluginAnalyzeDepsHookResultEntry {
          source: str.value.to_string(),
          kind: ResolveKind::Require,
        })
      }
    }

    call_expr.visit_children_with(self);
  }
}