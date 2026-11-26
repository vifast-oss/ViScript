// SPDX-License-Identifier: MIT OR Apache-2.0
//! Kết quả `initialize` — capabilities **cố định** (không version/rootUri) để snapshot tất định
//! (tách nguyên văn khỏi `lib.rs` ở `ST-317b`).

use vis_core::value::Value;

use crate::{code_action, jsonrpc, semantic_tokens};

/// Kết quả `initialize` — **cố định** (không version/rootUri) để snapshot tất định.
pub(crate) fn ket_qua_khoi_tao() -> Value {
    let completion = Value::map(vec![jsonrpc::kv(
        "triggerCharacters",
        Value::list(vec![Value::Str(".".to_string())]),
    )]);
    let capabilities = Value::map(vec![
        jsonrpc::kv(
            "textDocumentSync",
            jsonrpc::m(vec![
                ("openClose", Value::Bool(true)),
                ("change", Value::Int(2)),
            ]),
        ),
        jsonrpc::kv("positionEncoding", Value::Str("utf-16".to_string())),
        jsonrpc::kv("hoverProvider", Value::Bool(true)),
        jsonrpc::kv("definitionProvider", Value::Bool(true)),
        jsonrpc::kv("implementationProvider", Value::Bool(true)),
        jsonrpc::kv("documentSymbolProvider", Value::Bool(true)),
        jsonrpc::kv(
            "codeLensProvider",
            jsonrpc::m(vec![("resolveProvider", Value::Bool(false))]),
        ),
        jsonrpc::kv("foldingRangeProvider", Value::Bool(true)),
        jsonrpc::kv("workspaceSymbolProvider", Value::Bool(true)),
        jsonrpc::kv("documentFormattingProvider", Value::Bool(true)),
        jsonrpc::kv("codeActionProvider", code_action::provider()),
        jsonrpc::kv("referencesProvider", Value::Bool(true)),
        jsonrpc::kv("documentHighlightProvider", Value::Bool(true)),
        jsonrpc::kv("selectionRangeProvider", Value::Bool(true)),
        jsonrpc::kv("inlayHintProvider", Value::Bool(true)),
        jsonrpc::kv("typeDefinitionProvider", Value::Bool(true)),
        jsonrpc::kv(
            "renameProvider",
            jsonrpc::m(vec![("prepareProvider", Value::Bool(true))]),
        ),
        jsonrpc::kv(
            "signatureHelpProvider",
            jsonrpc::m(vec![(
                "triggerCharacters",
                Value::list(vec![Value::Str("(".into()), Value::Str(",".into())]),
            )]),
        ),
        jsonrpc::kv("completionProvider", completion),
        jsonrpc::kv("semanticTokensProvider", semantic_tokens::provider()),
    ]);
    let server_info = Value::map(vec![jsonrpc::kv("name", Value::Str("vis".to_string()))]);
    Value::map(vec![
        jsonrpc::kv("capabilities", capabilities),
        jsonrpc::kv("serverInfo", server_info),
    ])
}
