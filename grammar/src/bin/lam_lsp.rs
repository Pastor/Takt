//! LSP-сервер для языка Lam.
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
use lsp_types::request::{
    Completion, Formatting, GotoDeclaration, GotoDeclarationParams, GotoDeclarationResponse,
    HoverRequest, Request as _,
};
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
            trigger_characters: Some(vec![" ".to_string(), ".".to_string(), ":".to_string()]),
            resolve_provider: Some(false),
            ..Default::default()
        }),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        declaration_provider: Some(DeclarationCapability::Simple(true)),
        document_symbol_provider: Some(OneOf::Left(true)),
        // Фича 0024: канонический форматтер. То же ядро, что у `lamc fmt`, —
        // расхождение стилей между CLI и редактором невозможно по построению.
        document_formatting_provider: Some(OneOf::Left(true)),
        semantic_tokens_provider: Some(SemanticTokensServerCapabilities::SemanticTokensOptions(
            SemanticTokensOptions {
                legend: SemanticTokensLegend {
                    token_types: grammar::lsp::SEMANTIC_TOKEN_TYPES.to_vec(),
                    token_modifiers: vec![],
                },
                full: Some(SemanticTokensFullOptions::Bool(true)),
                range: None,
                work_done_progress_options: Default::default(),
            },
        )),
        ..Default::default()
    })?;

    // Выполняем инициализационное рукопожатие
    let (init_id, _init_params) = connection.initialize_start()?;
    let init_result = InitializeResult {
        capabilities: serde_json::from_value(server_capabilities)?,
        server_info: Some(ServerInfo {
            name: "lam-lsp".to_string(),
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
        Formatting::METHOD => {
            let params: DocumentFormattingParams = serde_json::from_value(req.params)?;
            let uri = &params.text_document.uri;
            let text = state.get_text(uri).unwrap_or("");
            // `Ok(None)` — текст уже каноничен: правок нет, файл не трогаем.
            // `Err` — форматировать нельзя: логируем и отвечаем `null`. Молча
            // «отформатировать во что-то» хуже, чем не форматировать вовсе.
            let result = match grammar::lsp::formatting_edits(text) {
                Ok(edits) => edits,
                Err(e) => {
                    eprintln!("[lam-lsp] форматирование не выполнено: {e}");
                    None
                }
            };
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
        GotoDeclaration::METHOD => {
            let params: GotoDeclarationParams = serde_json::from_value(req.params)?;
            let uri = &params.text_document_position_params.text_document.uri;
            let position = params.text_document_position_params.position;
            let text = state.get_text(uri).unwrap_or("");
            let result = grammar::lsp::goto_declaration(text, position).map(|range| {
                GotoDeclarationResponse::Scalar(Location {
                    uri: uri.clone(),
                    range,
                })
            });
            connection.sender.send(Message::Response(Response::new_ok(
                req.id,
                serde_json::to_value(result)?,
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
            let diagnostics = grammar::lsp::collect_diagnostics_at(&uri_to_path(&uri), &text, &[]);
            state.documents.insert(uri.clone(), text);
            publish_diagnostics(connection, uri, diagnostics)?;
        }
        DidChangeTextDocument::METHOD => {
            let params: DidChangeTextDocumentParams = serde_json::from_value(not.params)?;
            let uri = params.text_document.uri;
            // Используем полный текст документа (sync kind FULL)
            if let Some(change) = params.content_changes.into_iter().last() {
                let text = change.text;
                let diagnostics =
                    grammar::lsp::collect_diagnostics_at(&uri_to_path(&uri), &text, &[]);
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

/// Путь файла из URI документа (фича 0055).
///
/// Нужен, чтобы редактор разрешал `import` так же, как `lamc`: каталог документа
/// — неявный путь поиска. Прежде диагностики собирались вообще без путей, и
/// `import "lib.lam";` всегда давал «файл не найден».
///
/// Обрабатывается только схема `file:` — иные (например, `untitled:`) пути не
/// имеют, и импорт для них не разрешится: это честнее, чем угадывать каталог.
fn uri_to_path(uri: &Uri) -> String {
    let raw = uri.as_str();
    let path = raw.strip_prefix("file://").unwrap_or(raw);
    percent_decode(path)
}

/// Раскодирует `%XX` в пути URI: пробелы и кириллица в путях реальны.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
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
    connection
        .sender
        .send(Message::Notification(Notification::new(
            PublishDiagnostics::METHOD.to_string(),
            params,
        )))?;
    Ok(())
}
