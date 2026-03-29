//! LSP-сервер для языка BuT.
//!
//! Реализует протокол Language Server Protocol для редактора Zed и других LSP-клиентов.
//!
//! ## Поддерживаемые возможности
//!
//! - `textDocument/didOpen`, `didChange`, `didClose` — синхронизация документов
//! - `textDocument/publishDiagnostics` — отправка ошибок и предупреждений
//! - `textDocument/completion` — автодополнение ключевых слов и идентификаторов
//! - `textDocument/hover` — информация о типе идентификатора под курсором

use std::collections::HashMap;
use std::error::Error;

use lsp_server::{Connection, Message, Notification, Request, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification as _,
    PublishDiagnostics,
};
use lsp_types::request::{Completion, HoverRequest, Request as _};
use lsp_types::*;

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    // Инициализируем соединение через stdin/stdout
    let (connection, io_threads) = Connection::stdio();

    // Описываем возможности сервера
    let server_capabilities = serde_json::to_value(ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(TextDocumentSyncKind::FULL),
                save: Some(TextDocumentSyncSaveOptions::Supported(true)),
                ..Default::default()
            },
        )),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec![
                " ".to_string(),
                ".".to_string(),
                ":".to_string(),
            ]),
            resolve_provider: Some(false),
            ..Default::default()
        }),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        document_symbol_provider: Some(OneOf::Left(true)),
        semantic_tokens_provider: Some(
            SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
                legend: SemanticTokensLegend {
                    token_types: grammar::lsp::SEMANTIC_TOKEN_TYPES.to_vec(),
                    token_modifiers: vec![],
                },
                full: Some(SemanticTokensFullOptions::Bool(true)),
                range: None,
                work_done_progress_options: Default::default(),
            }),
        ),
        ..Default::default()
    })?;

    // Выполняем инициализационное рукопожатие
    let (init_id, _init_params) = connection.initialize_start()?;
    let init_result = InitializeResult {
        capabilities: serde_json::from_value(server_capabilities)?,
        server_info: Some(ServerInfo {
            name: "but-lsp".to_string(),
            version: Some(env!("CARGO_PKG_VERSION").to_string()),
        }),
    };
    connection.initialize_finish(init_id, serde_json::to_value(init_result)?)?;

    // Запускаем основной цикл обработки сообщений
    main_loop(connection)?;
    io_threads.join()?;

    Ok(())
}

/// Состояние сервера: открытые документы.
struct ServerState {
    /// Содержимое открытых документов: URI → текст.
    documents: HashMap<Uri, String>,
}

impl ServerState {
    fn new() -> Self {
        ServerState {
            documents: HashMap::new(),
        }
    }

    /// Возвращает содержимое документа по URI.
    fn get_text(&self, uri: &Uri) -> Option<&str> {
        self.documents.get(uri).map(String::as_str)
    }
}

/// Основной цикл обработки LSP-сообщений.
fn main_loop(connection: Connection) -> Result<(), Box<dyn Error + Sync + Send>> {
    let mut state = ServerState::new();

    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                // Проверяем запрос на завершение работы
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                handle_request(&connection, &state, req)?;
            }
            Message::Notification(not) => {
                handle_notification(&connection, &mut state, not)?;
            }
            Message::Response(_) => {
                // Ответы от клиента игнорируем
            }
        }
    }
    Ok(())
}

/// Обрабатывает входящий запрос от клиента.
fn handle_request(
    connection: &Connection,
    state: &ServerState,
    req: Request,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    match req.method.as_str() {
        Completion::METHOD => {
            let params: CompletionParams = serde_json::from_value(req.params)?;
            let uri = &params.text_document_position.text_document.uri;
            let text = state.get_text(uri).unwrap_or("");
            let items = grammar::lsp::completion_items(text);
            let result = CompletionResponse::Array(items);
            connection.sender.send(Message::Response(Response::new_ok(
                req.id,
                serde_json::to_value(result)?,
            )))?;
        }
        HoverRequest::METHOD => {
            let params: HoverParams = serde_json::from_value(req.params)?;
            let uri = &params.text_document_position_params.text_document.uri;
            let position = params.text_document_position_params.position;
            let text = state.get_text(uri).unwrap_or("");
            let hover = grammar::lsp::hover_info(text, position);
            connection.sender.send(Message::Response(Response::new_ok(
                req.id,
                serde_json::to_value(hover)?,
            )))?;
        }
        "textDocument/documentSymbol" => {
            let params: DocumentSymbolParams = serde_json::from_value(req.params)?;
            let uri = &params.text_document.uri;
            let text = state.get_text(uri).unwrap_or("");
            let symbols = grammar::lsp::document_symbols(text);
            connection.sender.send(Message::Response(Response::new_ok(
                req.id,
                serde_json::to_value(symbols)?,
            )))?;
        }
        "textDocument/semanticTokens/full" => {
            let params: SemanticTokensParams = serde_json::from_value(req.params)?;
            let uri = &params.text_document.uri;
            let text = state.get_text(uri).unwrap_or("");
            let tokens = grammar::lsp::semantic_tokens(text);
            connection.sender.send(Message::Response(Response::new_ok(
                req.id,
                serde_json::to_value(tokens)?,
            )))?;
        }
        _ => {
            // Неизвестный запрос — отвечаем ошибкой «метод не найден»
            connection.sender.send(Message::Response(Response::new_err(
                req.id,
                lsp_server::ErrorCode::MethodNotFound as i32,
                format!("неизвестный метод: {}", req.method),
            )))?;
        }
    }
    Ok(())
}

/// Обрабатывает входящее уведомление от клиента.
fn handle_notification(
    connection: &Connection,
    state: &mut ServerState,
    not: Notification,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    match not.method.as_str() {
        DidOpenTextDocument::METHOD => {
            let params: DidOpenTextDocumentParams = serde_json::from_value(not.params)?;
            let uri = params.text_document.uri;
            let text = params.text_document.text;
            let diagnostics = grammar::lsp::collect_diagnostics(&text);
            state.documents.insert(uri.clone(), text);
            publish_diagnostics(connection, uri, diagnostics)?;
        }
        DidChangeTextDocument::METHOD => {
            let params: DidChangeTextDocumentParams = serde_json::from_value(not.params)?;
            let uri = params.text_document.uri;
            // Используем полный текст документа (sync kind FULL)
            if let Some(change) = params.content_changes.into_iter().last() {
                let text = change.text;
                let diagnostics = grammar::lsp::collect_diagnostics(&text);
                state.documents.insert(uri.clone(), text);
                publish_diagnostics(connection, uri, diagnostics)?;
            }
        }
        DidCloseTextDocument::METHOD => {
            let params: DidCloseTextDocumentParams = serde_json::from_value(not.params)?;
            let uri = params.text_document.uri;
            state.documents.remove(&uri);
            // Очищаем диагностику для закрытого документа
            publish_diagnostics(connection, uri, vec![])?;
        }
        _ => {
            // Неизвестные уведомления игнорируем
        }
    }
    Ok(())
}

/// Отправляет диагностику клиенту.
fn publish_diagnostics(
    connection: &Connection,
    uri: Uri,
    diagnostics: Vec<Diagnostic>,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let params = PublishDiagnosticsParams {
        uri,
        diagnostics,
        version: None,
    };
    connection.sender.send(Message::Notification(Notification::new(
        PublishDiagnostics::METHOD.to_string(),
        params,
    )))?;
    Ok(())
}
