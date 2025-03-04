mod builder;
mod comment;
mod expr;
mod stats;
mod imports;

use builder::FoldingRangeBuilder;
use emmylua_code_analysis::Emmyrc;
use comment::build_comment_fold_range;
use emmylua_parser::{LuaAst, LuaAstNode};
use expr::{build_closure_expr_fold_range, build_string_fold_range, build_table_expr_fold_range};
use imports::build_imports_fold_range;
use lsp_types::{
    ClientCapabilities, FoldingRange, FoldingRangeParams, FoldingRangeProviderCapability,
    ServerCapabilities,
};
use stats::{
    build_do_stat_fold_range, build_for_range_stat_fold_range, build_for_stat_fold_range,
    build_func_stat_fold_range, build_if_stat_fold_range, build_local_func_stat_fold_range,
    build_repeat_stat_fold_range, build_while_stat_fold_range,
};
use tokio_util::sync::CancellationToken;

use crate::context::ServerContextSnapshot;

pub async fn on_folding_range_handler(
    context: ServerContextSnapshot,
    params: FoldingRangeParams,
    _: CancellationToken,
) -> Option<Vec<FoldingRange>> {
    let uri = params.text_document.uri;
    let analysis = context.analysis.read().await;
    let file_id = analysis.get_file_id(&uri)?;
    let semantic_model = analysis.compilation.get_semantic_model(file_id)?;
    let document = semantic_model.get_document();
    let root = semantic_model.get_root();
    let emmyrc = semantic_model.get_emmyrc();
    let mut builder = FoldingRangeBuilder::new(&document, root.clone());
    build_folding_ranges(&mut builder, emmyrc);
    Some(builder.build())
}

fn build_folding_ranges(builder: &mut FoldingRangeBuilder, emmyrc: &Emmyrc) {
    let root = builder.get_root().clone();
    for child in root.descendants::<LuaAst>() {
        match child {
            LuaAst::LuaForStat(for_stat) => {
                build_for_stat_fold_range(builder, for_stat);
            }
            LuaAst::LuaForRangeStat(for_range_stat) => {
                build_for_range_stat_fold_range(builder, for_range_stat);
            }
            LuaAst::LuaWhileStat(while_stat) => {
                build_while_stat_fold_range(builder, while_stat);
            }
            LuaAst::LuaRepeatStat(repeat_stat) => {
                build_repeat_stat_fold_range(builder, repeat_stat);
            }
            LuaAst::LuaDoStat(do_stat) => {
                build_do_stat_fold_range(builder, do_stat);
            }
            LuaAst::LuaLocalFuncStat(local_func_stat) => {
                build_local_func_stat_fold_range(builder, local_func_stat);
            }
            LuaAst::LuaFuncStat(func_stat) => {
                build_func_stat_fold_range(builder, func_stat);
            }
            LuaAst::LuaTableExpr(table_expr) => {
                build_table_expr_fold_range(builder, table_expr);
            }
            LuaAst::LuaComment(comment) => {
                build_comment_fold_range(builder, comment);
            }
            LuaAst::LuaLiteralExpr(literal) => {
                build_string_fold_range(builder, literal);
            }
            LuaAst::LuaClosureExpr(closure) => {
                build_closure_expr_fold_range(builder, closure);
            }
            LuaAst::LuaIfStat(if_stat) => {
                build_if_stat_fold_range(builder, if_stat);
            }
            _ => {}
        }
    }

    build_imports_fold_range(builder, root, emmyrc);
}

pub fn register_capabilities(
    server_capabilities: &mut ServerCapabilities,
    _: &ClientCapabilities,
) -> Option<()> {
    server_capabilities.folding_range_provider = Some(FoldingRangeProviderCapability::Simple(true));
    Some(())
}
