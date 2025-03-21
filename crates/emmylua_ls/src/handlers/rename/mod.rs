mod rename_decl;
mod rename_member;
mod rename_type;

use std::collections::HashMap;

use emmylua_code_analysis::{LuaCompilation, LuaPropertyOwnerId, SemanticModel};
use emmylua_parser::{LuaAstNode, LuaSyntaxToken, LuaTokenKind};
use lsp_types::{
    ClientCapabilities, OneOf, PrepareRenameResponse, RenameOptions, RenameParams,
    ServerCapabilities, TextDocumentPositionParams, WorkspaceEdit,
};
use rename_decl::rename_decl_references;
use rename_member::rename_member_references;
use rename_type::rename_type_references;
use rowan::TokenAtOffset;
use tokio_util::sync::CancellationToken;

use crate::context::ServerContextSnapshot;

pub async fn on_rename_handler(
    context: ServerContextSnapshot,
    params: RenameParams,
    _: CancellationToken,
) -> Option<WorkspaceEdit> {
    let uri = params.text_document_position.text_document.uri;
    let analysis = context.analysis.read().await;
    let file_id = analysis.get_file_id(&uri)?;
    let position = params.text_document_position.position;
    let mut semantic_model = analysis.compilation.get_semantic_model(file_id)?;
    let root = semantic_model.get_root();
    let position_offset = {
        let document = semantic_model.get_document();
        document.get_offset(position.line as usize, position.character as usize)?
    };

    if position_offset > root.syntax().text_range().end() {
        return None;
    }

    let token = match root.syntax().token_at_offset(position_offset) {
        TokenAtOffset::Single(token) => token,
        TokenAtOffset::Between(left, right) => {
            if left.kind() == LuaTokenKind::TkName.into() {
                left
            } else {
                right
            }
        }
        TokenAtOffset::None => {
            return None;
        }
    };

    rename_references(
        &mut semantic_model,
        &analysis.compilation,
        token,
        params.new_name,
    )
}

pub async fn on_prepare_rename_handler(
    context: ServerContextSnapshot,
    params: TextDocumentPositionParams,
    _: CancellationToken,
) -> Option<PrepareRenameResponse> {
    let uri = params.text_document.uri;
    let analysis = context.analysis.read().await;
    let file_id = analysis.get_file_id(&uri)?;
    let position = params.position;
    let semantic_model = analysis.compilation.get_semantic_model(file_id)?;
    let root = semantic_model.get_root();
    let position_offset = {
        let document = semantic_model.get_document();
        document.get_offset(position.line as usize, position.character as usize)?
    };

    if position_offset > root.syntax().text_range().end() {
        return None;
    }

    let token = match root.syntax().token_at_offset(position_offset) {
        TokenAtOffset::Single(token) => token,
        TokenAtOffset::Between(left, right) => {
            if left.kind() == LuaTokenKind::TkName.into() {
                left
            } else {
                right
            }
        }
        TokenAtOffset::None => {
            return None;
        }
    };

    if matches!(
        token.kind().into(),
        LuaTokenKind::TkName | LuaTokenKind::TkInt | LuaTokenKind::TkString
    ) {
        Some(PrepareRenameResponse::DefaultBehavior {
            default_behavior: true,
        })
    } else {
        None
    }
}

fn rename_references(
    semantic_model: &SemanticModel,
    compilation: &LuaCompilation,
    token: LuaSyntaxToken,
    new_name: String,
) -> Option<WorkspaceEdit> {
    let mut result = HashMap::new();
    let semantic_info = semantic_model.get_semantic_info(token.into())?;
    match semantic_info.property_owner? {
        LuaPropertyOwnerId::LuaDecl(decl_id) => {
            rename_decl_references(semantic_model, compilation, decl_id, new_name, &mut result);
        }
        LuaPropertyOwnerId::Member(member_id) => {
            rename_member_references(
                semantic_model,
                compilation,
                member_id,
                new_name,
                &mut result,
            );
        }
        LuaPropertyOwnerId::TypeDecl(type_decl_id) => {
            rename_type_references(semantic_model, type_decl_id, new_name, &mut result);
        }
        _ => {}
    }

    let changes = result
        .into_iter()
        .map(|(uri, ranges)| {
            let text_edits = ranges
                .into_iter()
                .map(|(range, new_text)| lsp_types::TextEdit { range, new_text })
                .collect();
            (uri, text_edits)
        })
        .collect();

    Some(WorkspaceEdit {
        changes: Some(changes),
        document_changes: None,
        change_annotations: None,
    })
}

pub fn register_capabilities(
    server_capabilities: &mut ServerCapabilities,
    _: &ClientCapabilities,
) -> Option<()> {
    server_capabilities.rename_provider = Some(OneOf::Right(RenameOptions {
        prepare_provider: Some(true),
        work_done_progress_options: Default::default(),
    }));
    Some(())
}
