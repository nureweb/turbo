use anyhow::Result;
use swc_core::{
    common::DUMMY_SP,
    ecma::ast::{Expr, Ident},
    quote,
};
use turbopack_core::chunk::{ChunkingContextVc, ModuleId};

use super::{base::ReferencedAsset, EsmAssetReferenceVc};
use crate::{
    code_gen::{CodeGenerateable, CodeGenerateableVc, CodeGeneration, CodeGenerationVc},
    create_visitor, magic_identifier,
    references::{esm::base::insert_hoisted_stmt, AstPathVc},
};

#[turbo_tasks::value(shared)]
#[derive(Hash, Debug)]
pub struct ImportMetaRef {
    initialize: bool,
    inner: EsmAssetReferenceVc,
    ast_path: AstPathVc,
}

#[turbo_tasks::value_impl]
impl ImportMetaRefVc {
    #[turbo_tasks::function]
    pub fn new(initialize: bool, inner: EsmAssetReferenceVc, ast_path: AstPathVc) -> Self {
        ImportMetaRef {
            initialize,
            inner,
            ast_path,
        }
        .cell()
    }
}

#[turbo_tasks::value_impl]
impl CodeGenerateable for ImportMetaRef {
    #[turbo_tasks::function]
    async fn code_generation(&self, context: ChunkingContextVc) -> Result<CodeGenerationVc> {
        // TODO: should only be done in ESM
        let path = &self.ast_path.await?;
        let mut visitors = vec![create_visitor!(path, visit_mut_expr(expr: &mut Expr) {
            let id = Ident::new(magic_identifier::encode("import.meta").into(), DUMMY_SP);
            *expr = Expr::Ident(id);
        })];

        if self.initialize {
            let url =
                if let ReferencedAsset::Some(asset) = &*self.inner.get_referenced_asset().await? {
                    let id = asset.as_chunk_item(context).id().await?;
                    Expr::Lit(match &*id {
                        ModuleId::String(s) => s.clone().into(),
                        ModuleId::Number(n) => (*n as f64).into(),
                    })
                } else {
                    Expr::Lit("unknown".into())
                };
            visitors.push(create_visitor!(visit_mut_program(program: &mut Program) {
                let name = Ident::new(magic_identifier::encode("import.meta").into(), DUMMY_SP);
                let meta = quote!(
                    "const $name = { url: new Url($url, location.href).href };" as Stmt,
                    name = name,
                    url: Expr = url.clone(),
                );
                insert_hoisted_stmt(program, meta);
            }));
        }

        Ok(CodeGeneration { visitors }.into())
    }
}
