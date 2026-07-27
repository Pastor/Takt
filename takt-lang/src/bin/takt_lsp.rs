//! LSP-сервер для языка Takt.
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
    GotoDefinition, HoverRequest, References, Request as _,
};
use lsp_types::*;

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    // Инициализируем соединение через stdin/stdout
    let (connection, io_threads) = Connection::stdio();

    // Описываем возможности сервера. Список живёт в библиотеке (фича 0131):
    // бинарник тестами не покрыть, а «что объявлено» — проверяемый факт.
    let server_capabilities = serde_json::to_value(takt_lang::lsp::server_capabilities())?;

    // Выполняем инициализационное рукопожатие. Параметры клиента больше НЕ
    // игнорируются (фича 0072): из `initializationOptions.searchPaths` берутся
    // пути поиска импортов (аналог `-I` у `taktc`), иначе импорт из общей
    // библиотеки вне каталога документа в редакторе не находится.
    let (init_id, init_params) = connection.initialize_start()?;
    let search_paths = search_paths_from_init(&init_params);
    let init_result = InitializeResult {
        capabilities: serde_json::from_value(server_capabilities)?,
        server_info: Some(ServerInfo {
            name: "takt-lsp".to_string(),
            version: Some(env!("CARGO_PKG_VERSION").to_string()),
        }),
    };
    connection.initialize_finish(init_id, serde_json::to_value(init_result)?)?;

    // Запускаем основной цикл обработки сообщений
    main_loop(connection, search_paths)?;
    io_threads.join()?;

    Ok(())
}

/// Пути поиска импортов из `initializationOptions` клиента (фича 0072).
///
/// `init_params` — сырой JSON `InitializeParams` из `initialize_start()`.
/// Относительные пути `searchPaths` разрешаются от корня рабочей области
/// (`root_uri`); разбор и устойчивость к плохому конфигу — в библиотеке
/// `takt_lang::lsp::search_paths_from_options` (ради тестируемости: бинарник
/// юнит-тестами не покрыть).
fn search_paths_from_init(init_params: &serde_json::Value) -> Vec<String> {
    let params: InitializeParams = match serde_json::from_value(init_params.clone()) {
        Ok(p) => p,
        // Нечитаемые параметры инициализации — работаем без путей поиска
        // (прежнее поведение), а не падаем на старте.
        Err(e) => {
            eprintln!("[takt-lsp] initializationOptions не разобраны: {e}");
            return Vec::new();
        }
    };
    #[allow(deprecated)] // root_uri помечен устаревшим, но именно его шлют клиенты (Zed).
    let root = params.root_uri.as_ref().map(uri_to_path);
    takt_lang::lsp::search_paths_from_options(
        params.initialization_options.as_ref(),
        root.as_deref(),
    )
}

/// Состояние сервера: открытые документы + пути поиска импортов.
struct ServerState {
    /// Содержимое открытых документов: URI → текст.
    documents: HashMap<Uri, String>,
    /// Пути поиска импортов (аналог `-I` у `taktc`), из `initializationOptions`
    /// (фича 0072). Пустой список = прежнее поведение (только каталог документа).
    search_paths: Vec<String>,
}

impl ServerState {
    fn new(search_paths: Vec<String>) -> Self {
        ServerState {
            documents: HashMap::new(),
            search_paths,
        }
    }

    /// Возвращает содержимое документа по URI.
    fn get_text(&self, uri: &Uri) -> Option<&str> {
        self.documents.get(uri).map(String::as_str)
    }
}

/// Основной цикл обработки LSP-сообщений.
fn main_loop(
    connection: Connection,
    search_paths: Vec<String>,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let mut state = ServerState::new(search_paths);

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
            let items = takt_lang::lsp::completion_items(text);
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
            let result = match takt_lang::lsp::formatting_edits(text) {
                Ok(edits) => edits,
                Err(e) => {
                    eprintln!("[takt-lsp] форматирование не выполнено: {e}");
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
            let hover = takt_lang::lsp::hover_info(text, position);
            connection.sender.send(Message::Response(Response::new_ok(
                req.id,
                serde_json::to_value(hover)?,
            )))?;
        }
        // ⚠️ Одна ветка на оба метода (фича 0131): в Takt объявление и
        // определение — одно и то же, и разделять их нечего. Разные редакторы
        // шлют по F12 разное (VS Code — `definition`, Zed — `declaration`);
        // обслуживая их **разным** кодом, сервер рано или поздно ответил бы
        // по-разному на один и тот же курсор. Параметры и ответ у методов
        // совпадают по типу (`GotoDeclarationParams = GotoDefinitionParams`),
        // поэтому объединение бесплатно.
        GotoDeclaration::METHOD | GotoDefinition::METHOD => {
            let params: GotoDeclarationParams = serde_json::from_value(req.params)?;
            let uri = &params.text_document_position_params.text_document.uri;
            let position = params.text_document_position_params.position;
            let text = state.get_text(uri).unwrap_or("");
            // Кросс-файловый вариант с путём документа: каталог документа —
            // неявный путь импорта (0055), без него переходить в чужой файл
            // некуда. Прежде звался однофайловый `goto_declaration`, и URI ответа
            // был ВСЕГДА текущим — переход в импортированный файл не работал
            // вовсе (фича 0056).
            let result = takt_lang::lsp::goto_declaration_at(
                &uri_to_path(uri),
                text,
                position,
                &state.search_paths,
            )
            .and_then(|loc| {
                // Пустой URI — контракт «это текущий файл»: подставляет
                // вызывающий, у которого URI документа и так на руках.
                let target = if loc.uri.is_empty() {
                    uri.clone()
                } else {
                    loc.uri.parse().ok()?
                };
                Some(GotoDeclarationResponse::Scalar(Location {
                    uri: target,
                    range: loc.range,
                }))
            });
            connection.sender.send(Message::Response(Response::new_ok(
                req.id,
                serde_json::to_value(result)?,
            )))?;
        }
        References::METHOD => {
            let params: ReferenceParams = serde_json::from_value(req.params)?;
            let uri = &params.text_document_position.text_document.uri;
            let position = params.text_document_position.position;
            let text = state.get_text(uri).unwrap_or("");
            // Вхождения ищутся в открытом документе: рабочая область сервером не
            // индексируется (фича 0131). Ответ правдив, но не всеобъемлющ — и
            // это лучше, чем молчание.
            let result =
                takt_lang::lsp::references_at(text, position, params.context.include_declaration)
                    .map(|ranges| {
                        ranges
                            .into_iter()
                            .map(|range| Location {
                                uri: uri.clone(),
                                range,
                            })
                            .collect::<Vec<_>>()
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
            let symbols = takt_lang::lsp::document_symbols(text);
            connection.sender.send(Message::Response(Response::new_ok(
                req.id,
                serde_json::to_value(symbols)?,
            )))?;
        }
        "textDocument/semanticTokens/full" => {
            let params: SemanticTokensParams = serde_json::from_value(req.params)?;
            let uri = &params.text_document.uri;
            let text = state.get_text(uri).unwrap_or("");
            let tokens = takt_lang::lsp::semantic_tokens(text);
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
            let diagnostics = takt_lang::lsp::collect_diagnostics_at(
                &uri_to_path(&uri),
                &text,
                &state.search_paths,
            );
            state.documents.insert(uri.clone(), text);
            publish_diagnostics(connection, uri, diagnostics)?;
        }
        DidChangeTextDocument::METHOD => {
            let params: DidChangeTextDocumentParams = serde_json::from_value(not.params)?;
            let uri = params.text_document.uri;
            // Используем полный текст документа (sync kind FULL)
            if let Some(change) = params.content_changes.into_iter().last() {
                let text = change.text;
                let diagnostics = takt_lang::lsp::collect_diagnostics_at(
                    &uri_to_path(&uri),
                    &text,
                    &state.search_paths,
                );
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
/// Нужен, чтобы редактор разрешал `import` так же, как `taktc`: каталог документа
/// — неявный путь поиска. Прежде диагностики собирались вообще без путей, и
/// `import "lib.takt";` всегда давал «файл не найден».
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
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(byte) = u8::from_str_radix(&s[i + 1..i + 3], 16)
        {
            out.push(byte);
            i += 3;
            continue;
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
