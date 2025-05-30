use emmylua_parser::{BinaryOperator, LuaAstNode, LuaBlock, LuaDocTagCast};
use rowan::TextRange;

use crate::{compilation::analyzer::AnalyzeContext, FileId, InFiled, LuaType, TypeAssertion};

use super::var_analyze::VarTrace;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastAction {
    Force,
    Add,
    Remove,
}

pub fn analyze_cast(
    var_trace: &mut VarTrace,
    file_id: FileId,
    tag: LuaDocTagCast,
    context: &AnalyzeContext,
) -> Option<()> {
    let block_range = tag.ancestors::<LuaBlock>().next()?.get_range();
    let cast_range = tag.get_range();

    let cast_end = cast_range.end();
    let block_end = block_range.end();

    if block_end <= cast_end {
        return Some(());
    }
    let effect_range = TextRange::new(cast_end, block_end);
    for cast_op_type in tag.get_op_types() {
        let action = match cast_op_type.get_op() {
            Some(op) => {
                if op.get_op() == BinaryOperator::OpAdd {
                    CastAction::Add
                } else {
                    CastAction::Remove
                }
            }
            None => CastAction::Force,
        };

        if cast_op_type.is_nullable() {
            match action {
                CastAction::Add => {
                    var_trace.add_assert(TypeAssertion::Add(LuaType::Nil), effect_range);
                }
                CastAction::Remove => {
                    var_trace.add_assert(TypeAssertion::Remove(LuaType::Nil), effect_range);
                }
                _ => {}
            }
        } else if let Some(doc_typ) = cast_op_type.get_type() {
            let key = InFiled::new(file_id, doc_typ.get_syntax_id());
            let typ = match context.cast_flow.get(&key) {
                Some(t) => t.clone(),
                None => continue,
            };

            match action {
                CastAction::Add => {
                    var_trace.add_assert(TypeAssertion::Add(typ), effect_range);
                }
                CastAction::Remove => {
                    var_trace.add_assert(TypeAssertion::Remove(typ), effect_range);
                }
                CastAction::Force => {
                    var_trace.add_assert(TypeAssertion::Force(typ), effect_range);
                }
            }
        }
    }

    Some(())
}
