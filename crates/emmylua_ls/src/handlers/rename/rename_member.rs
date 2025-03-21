use std::collections::HashMap;

use emmylua_code_analysis::{LuaCompilation, LuaMemberId, LuaPropertyOwnerId, SemanticModel};
use emmylua_parser::{LuaAst, LuaAstNode, LuaAstToken, LuaNameToken, LuaSyntaxNode};
use lsp_types::Uri;

pub fn rename_member_references(
    semantic_model: &SemanticModel,
    compilation: &LuaCompilation,
    member_id: LuaMemberId,
    new_name: String,
    result: &mut HashMap<Uri, HashMap<lsp_types::Range, String>>,
) -> Option<()> {
    let member = semantic_model
        .get_db()
        .get_member_index()
        .get_member(&member_id)?;
    let key = member.get_key();
    let index_references = semantic_model
        .get_db()
        .get_reference_index()
        .get_index_references(&key)?;

    let property_owner = LuaPropertyOwnerId::Member(member_id);
    let mut semantic_cache = HashMap::new();
    for in_filed_syntax_id in index_references {
        let semantic_model =
            if let Some(semantic_model) = semantic_cache.get_mut(&in_filed_syntax_id.file_id) {
                semantic_model
            } else {
                let semantic_model = compilation.get_semantic_model(in_filed_syntax_id.file_id)?;
                semantic_cache.insert(in_filed_syntax_id.file_id, semantic_model);
                semantic_cache.get_mut(&in_filed_syntax_id.file_id)?
            };
        let root = semantic_model.get_root();
        let node = in_filed_syntax_id.value.to_node_from_root(root.syntax())?;
        if semantic_model.is_reference_to(node.clone(), property_owner.clone()) {
            let range = get_member_name_token_lsp_range(semantic_model, node.clone())?;
            result
                .entry(semantic_model.get_document().get_uri())
                .or_insert_with(HashMap::new)
                .insert(range, new_name.clone());
        }
    }

    Some(())
}

fn get_member_name_token_lsp_range(
    semantic_model: &SemanticModel,
    node: LuaSyntaxNode,
) -> Option<lsp_types::Range> {
    let document = semantic_model.get_document();
    let node = LuaAst::cast(node)?;
    // todo
    let token = node.token::<LuaNameToken>()?;
    document.to_lsp_range(token.get_range())
}
