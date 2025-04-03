use std::collections::HashSet;

use emmylua_code_analysis::{LuaFlowId, LuaSignatureId, LuaType};
use emmylua_parser::{LuaAst, LuaAstNode, LuaCallArgList, LuaClosureExpr, LuaParamList};
use lsp_types::{CompletionItem, CompletionItemKind, CompletionTriggerKind};

use crate::handlers::completion::{
    add_completions::{add_decl_completion, check_match_word},
    completion_builder::CompletionBuilder,
};

pub fn add_completion(builder: &mut CompletionBuilder) -> Option<()> {
    if builder.is_cancelled() {
        return None;
    }

    if check_can_add_completion(builder).is_none() {
        return Some(());
    }

    let parent_node = LuaAst::cast(builder.trigger_token.parent()?)?;
    match parent_node {
        LuaAst::LuaNameExpr(_) => {}
        LuaAst::LuaBlock(_) => {}
        LuaAst::LuaClosureExpr(_) => {}
        LuaAst::LuaCallArgList(_) => {}
        // 字符串中触发的补全
        LuaAst::LuaLiteralExpr(_) => return None,
        _ => return None,
    };

    let mut duplicated_name = HashSet::new();
    builder.env_range.0 = builder.get_completion_items_mut().len();
    add_local_env(builder, &mut duplicated_name, &parent_node);
    add_global_env(builder, &mut duplicated_name);
    add_self(builder, &mut duplicated_name, &parent_node);
    builder.env_range.1 = builder.get_completion_items_mut().len();

    builder.env_duplicate_name.extend(duplicated_name);

    Some(())
}

fn check_can_add_completion(builder: &CompletionBuilder) -> Option<()> {
    if builder.is_space_trigger_character {
        return None;
    }

    let trigger_text = builder.get_trigger_text();
    if builder.trigger_kind == CompletionTriggerKind::TRIGGER_CHARACTER {
        let parent = builder.trigger_token.parent()?;

        if trigger_text == "(" {
            if LuaCallArgList::can_cast(parent.kind().into())
                || LuaParamList::can_cast(parent.kind().into())
            {
                return None;
            }
        }
    } else if builder.trigger_kind == CompletionTriggerKind::INVOKED {
        let parent = builder.trigger_token.parent()?;
        // 即时是主动触发, 也不允许在函数定义的参数列表中添加
        if trigger_text == "(" {
            if LuaParamList::can_cast(parent.kind().into()) {
                return None;
            }
        }
    }

    Some(())
}

fn add_self(
    builder: &mut CompletionBuilder,
    duplicated_name: &mut HashSet<String>,
    node: &LuaAst,
) -> Option<()> {
    let closure_expr = node.ancestors::<LuaClosureExpr>().next()?;
    let signature_id =
        LuaSignatureId::from_closure(builder.semantic_model.get_file_id(), &closure_expr);
    let signature = builder
        .semantic_model
        .get_db()
        .get_signature_index()
        .get(&signature_id)?;
    if signature.is_colon_define {
        let completion_item = CompletionItem {
            label: "self".to_string(),
            kind: Some(CompletionItemKind::VARIABLE),
            data: None,
            label_details: Some(lsp_types::CompletionItemLabelDetails {
                detail: None,
                description: None,
            }),
            ..Default::default()
        };

        builder.add_completion_item(completion_item)?;
        duplicated_name.insert("self".to_string());
    }

    Some(())
}

fn add_local_env(
    builder: &mut CompletionBuilder,
    duplicated_name: &mut HashSet<String>,
    node: &LuaAst,
) -> Option<()> {
    let flow_id = LuaFlowId::from_node(node.syntax());

    let file_id = builder.semantic_model.get_file_id();
    let decl_tree = builder
        .semantic_model
        .get_db()
        .get_decl_index()
        .get_decl_tree(&file_id)?;
    let local_env = decl_tree.get_env_decls(builder.trigger_token.text_range().start())?;

    let trigger_text = builder.get_trigger_text();

    for decl_id in local_env.iter() {
        // 获取变量名和类型
        let (name, mut typ) = {
            let decl = builder
                .semantic_model
                .get_db()
                .get_decl_index()
                .get_decl(&decl_id)?;
            (
                decl.get_name().to_string(),
                builder
                    .semantic_model
                    .get_db()
                    .get_type_index()
                    .get_type_cache(&decl_id.clone().into())
                    .map(|cache| cache.as_type().clone())
                    .unwrap_or(LuaType::Unknown),
            )
        };

        if duplicated_name.contains(&name) {
            continue;
        }

        if !env_check_match_word(&trigger_text, name.as_str()) {
            duplicated_name.insert(name.clone());
            continue;
        }

        // 类型缩窄
        if let Some(chain) = builder
            .semantic_model
            .get_db()
            .get_flow_index()
            .get_flow_chain(file_id, flow_id)
        {
            let semantic_model = &builder.semantic_model;
            let db = semantic_model.get_db();
            let root = semantic_model.get_root().syntax();
            let config = semantic_model.get_config();
            for type_assert in
                chain.get_type_asserts(&name, node.get_position(), Some(decl_id.position))
            {
                typ = type_assert
                    .tighten_type(db, &mut config.borrow_mut(), root, typ)
                    .unwrap_or(LuaType::Unknown);
            }
        }

        duplicated_name.insert(name.clone());
        add_decl_completion(builder, decl_id.clone(), &name, &typ);
    }

    Some(())
}

fn add_global_env(
    builder: &mut CompletionBuilder,
    duplicated_name: &mut HashSet<String>,
) -> Option<()> {
    let trigger_text = builder.get_trigger_text();
    let global_env = builder
        .semantic_model
        .get_db()
        .get_global_index()
        .get_all_global_decl_ids();
    for decl_id in global_env.iter() {
        let decl = builder
            .semantic_model
            .get_db()
            .get_decl_index()
            .get_decl(&decl_id)?;
        let (name, typ) = {
            (
                decl.get_name().to_string(),
                builder
                    .semantic_model
                    .get_db()
                    .get_type_index()
                    .get_type_cache(&decl_id.clone().into())
                    .map(|cache| cache.as_type().clone())
                    .unwrap_or(LuaType::Unknown),
            )
        };
        if duplicated_name.contains(&name) {
            continue;
        }
        if !env_check_match_word(&trigger_text, name.as_str()) {
            duplicated_name.insert(name.clone());
            continue;
        }
        // 如果范围相同, 则是在定义一个新的全局变量, 不需要添加
        if decl.get_range() == builder.trigger_token.text_range() {
            continue;
        }

        duplicated_name.insert(name.clone());
        add_decl_completion(builder, decl_id.clone(), &name, &typ);
    }

    Some(())
}

fn env_check_match_word(trigger_text: &str, name: &str) -> bool {
    // 如果首字母是`(`或者`,`则允许, 用于在函数参数调用处触发补全
    match trigger_text.chars().next() {
        Some('(') | Some(',') => true,
        _ => check_match_word(trigger_text, name),
    }
}
