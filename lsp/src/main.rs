use serde::de::{self, Deserializer};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

const SCHEMA_JSON: &str = include_str!("../../schema/ghostty-config.schema.json");

#[derive(Debug, Deserialize)]
struct GhosttySchema {
    options: HashMap<String, ConfigOption>,
    types: Option<TypeDefinitions>,
    #[serde(rename = "repeatableKeys")]
    #[allow(dead_code)]
    repeatable_keys: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct ConfigOption {
    #[serde(rename = "type")]
    option_type: String,
    description: String,
    #[serde(default)]
    repeatable: bool,
    #[serde(default, deserialize_with = "deserialize_deprecated")]
    deprecated: Option<String>,
    #[serde(rename = "enum")]
    enum_values: Option<Vec<String>>,
    examples: Option<Vec<String>>,
    platforms: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct TypeDefinitions {
    keybind: Option<KeybindType>,
    color: Option<ColorType>,
}

#[derive(Debug, Deserialize)]
struct KeybindType {
    prefixes: Option<Vec<String>>,
    modifiers: Option<Vec<String>>,
    actions: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct ColorType {
    #[serde(rename = "namedValues")]
    named_values: Option<Vec<String>>,
}

fn deserialize_deprecated<'de, D>(deserializer: D) -> std::result::Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: serde_json::Value = Deserialize::deserialize(deserializer)?;
    match value {
        serde_json::Value::Bool(false) | serde_json::Value::Null => Ok(None),
        serde_json::Value::Bool(true) => Ok(Some("Deprecated".to_string())),
        serde_json::Value::String(s) => Ok(Some(s)),
        _ => Err(de::Error::custom("expected bool or string for deprecated")),
    }
}

struct GhosttyLsp {
    client: Client,
    schema: GhosttySchema,
    documents: RwLock<HashMap<Url, String>>,
}

impl GhosttyLsp {
    fn new(client: Client) -> Self {
        let schema: GhosttySchema =
            serde_json::from_str(SCHEMA_JSON).expect("Failed to parse embedded schema");
        Self {
            client,
            schema,
            documents: RwLock::new(HashMap::new()),
        }
    }

    fn get_key_completions(&self, partial: &str) -> Vec<CompletionItem> {
        let partial_lower = partial.to_lowercase();
        self.schema
            .options
            .iter()
            .filter(|(key, _)| partial.is_empty() || key.to_lowercase().contains(&partial_lower))
            .map(|(key, opt)| {
                let detail = self.format_type_detail(opt);
                let mut item = CompletionItem {
                    label: key.clone(),
                    kind: Some(CompletionItemKind::PROPERTY),
                    detail: Some(detail),
                    documentation: Some(Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: self.format_key_documentation(key, opt),
                    })),
                    insert_text: Some(format!("{} = ", key)),
                    insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                    ..Default::default()
                };
                if opt.deprecated.is_some() {
                    item.tags = Some(vec![CompletionItemTag::DEPRECATED]);
                    item.sort_text = Some(format!("z_{}", key));
                }
                item
            })
            .collect()
    }

    fn format_type_detail(&self, opt: &ConfigOption) -> String {
        let mut parts = vec![opt.option_type.clone()];
        if opt.repeatable {
            parts.push("repeatable".to_string());
        }
        if let Some(platforms) = &opt.platforms {
            parts.push(format!("[{}]", platforms.join(", ")));
        }
        parts.join(" | ")
    }

    fn format_key_documentation(&self, key: &str, opt: &ConfigOption) -> String {
        let mut doc = String::new();
        if let Some(msg) = &opt.deprecated {
            doc.push_str(&format!("**Deprecated:** {}\n\n", msg));
        }
        doc.push_str(&opt.description);
        if let Some(examples) = &opt.examples {
            doc.push_str("\n\n**Examples:**\n");
            for ex in examples.iter().take(3) {
                doc.push_str(&format!("- `{} = {}`\n", key, ex));
            }
        }
        if let Some(enum_values) = &opt.enum_values {
            doc.push_str("\n\n**Valid values:** ");
            doc.push_str(&enum_values.join(", "));
        }
        doc
    }

    fn get_value_completions(&self, key: &str, partial: &str) -> Vec<CompletionItem> {
        let Some(opt) = self.schema.options.get(key) else {
            return vec![];
        };

        let partial_lower = partial.to_lowercase().trim().to_string();

        match opt.option_type.as_str() {
            "boolean" => self.get_boolean_completions(&partial_lower),
            "enum" => self.get_enum_completions(opt, &partial_lower),
            "color" => self.get_colour_completions(&partial_lower),
            "keybind" => self.get_keybind_completions(&partial_lower),
            "theme" => self.get_theme_completions(&partial_lower),
            _ => self.get_example_completions(opt, &partial_lower),
        }
    }

    fn get_boolean_completions(&self, partial: &str) -> Vec<CompletionItem> {
        ["true", "false"]
            .iter()
            .filter(|v| partial.is_empty() || v.contains(partial))
            .map(|v| self.simple_completion(v, CompletionItemKind::VALUE))
            .collect()
    }

    fn get_enum_completions(&self, opt: &ConfigOption, partial: &str) -> Vec<CompletionItem> {
        opt.enum_values
            .as_ref()
            .map(|vals| {
                vals.iter()
                    .filter(|v| partial.is_empty() || v.to_lowercase().contains(partial))
                    .map(|v| self.simple_completion(v, CompletionItemKind::ENUM_MEMBER))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn get_colour_completions(&self, partial: &str) -> Vec<CompletionItem> {
        let mut items: Vec<CompletionItem> = vec![];

        // Named colours from schema
        if let Some(types) = &self.schema.types {
            if let Some(color_type) = &types.color {
                if let Some(named) = &color_type.named_values {
                    for name in named {
                        if partial.is_empty() || name.to_lowercase().contains(partial) {
                            items.push(self.simple_completion(name, CompletionItemKind::COLOR));
                        }
                    }
                }
            }
        }

        // Hex colour template
        if partial.is_empty() || "#".contains(partial) || partial.starts_with('#') {
            let mut hex_item = self.simple_completion("#RRGGBB", CompletionItemKind::COLOR);
            hex_item.detail = Some("Hex colour".to_string());
            hex_item.insert_text = Some("#".to_string());
            items.push(hex_item);
        }

        items
    }

    fn get_keybind_completions(&self, partial: &str) -> Vec<CompletionItem> {
        let mut items: Vec<CompletionItem> = vec![];

        if let Some(types) = &self.schema.types {
            if let Some(keybind) = &types.keybind {
                // Prefixes (global:, all:, etc.)
                if let Some(prefixes) = &keybind.prefixes {
                    for prefix in prefixes {
                        let label = format!("{}:", prefix);
                        if partial.is_empty() || label.to_lowercase().contains(partial) {
                            let mut item =
                                self.simple_completion(&label, CompletionItemKind::KEYWORD);
                            item.detail = Some("Keybind prefix".to_string());
                            items.push(item);
                        }
                    }
                }

                // Modifiers (ctrl+, alt+, etc.)
                if let Some(modifiers) = &keybind.modifiers {
                    for modifier in modifiers {
                        let label = format!("{}+", modifier);
                        if partial.is_empty() || label.to_lowercase().contains(partial) {
                            let mut item =
                                self.simple_completion(&label, CompletionItemKind::KEYWORD);
                            item.detail = Some("Modifier key".to_string());
                            items.push(item);
                        }
                    }
                }

                // Actions (after =)
                if partial.contains('=') || partial.is_empty() {
                    if let Some(actions) = &keybind.actions {
                        let after_eq = partial.split('=').last().unwrap_or("").trim();
                        for action in actions {
                            if after_eq.is_empty() || action.to_lowercase().contains(after_eq) {
                                let mut item =
                                    self.simple_completion(action, CompletionItemKind::FUNCTION);
                                item.detail = Some("Keybind action".to_string());
                                items.push(item);
                            }
                        }
                    }
                }
            }
        }

        items
    }

    fn get_theme_completions(&self, partial: &str) -> Vec<CompletionItem> {
        let themes = [
            "auto",
            "Catppuccin Mocha",
            "Catppuccin Macchiato",
            "Catppuccin Frappe",
            "Catppuccin Latte",
            "Dracula",
            "Gruvbox Dark",
            "Gruvbox Light",
            "Nord",
            "One Dark",
            "Solarized Dark",
            "Solarized Light",
            "Tokyo Night",
            "Tokyo Night Storm",
            "Tomorrow Night",
        ];

        let mut items: Vec<CompletionItem> = themes
            .iter()
            .filter(|t| partial.is_empty() || t.to_lowercase().contains(partial))
            .map(|t| {
                let mut item = self.simple_completion(t, CompletionItemKind::VALUE);
                item.detail = Some("Built-in theme".to_string());
                item
            })
            .collect();

        // Light/dark combo snippet
        if partial.is_empty() || "light:".contains(partial) {
            let mut combo = CompletionItem {
                label: "light:...,dark:...".to_string(),
                kind: Some(CompletionItemKind::SNIPPET),
                detail: Some("Light/dark theme combination".to_string()),
                insert_text: Some("light:${1:Catppuccin Latte},dark:${2:Catppuccin Mocha}".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            };
            combo.documentation = Some(Documentation::String(
                "Use different themes for light and dark mode".to_string(),
            ));
            items.push(combo);
        }

        items
    }

    fn get_example_completions(&self, opt: &ConfigOption, partial: &str) -> Vec<CompletionItem> {
        opt.examples
            .as_ref()
            .map(|examples| {
                examples
                    .iter()
                    .filter(|ex| partial.is_empty() || ex.to_lowercase().contains(partial))
                    .map(|ex| {
                        let mut item = self.simple_completion(ex, CompletionItemKind::VALUE);
                        item.detail = Some("Example value".to_string());
                        item
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn simple_completion(&self, label: &str, kind: CompletionItemKind) -> CompletionItem {
        CompletionItem {
            label: label.to_string(),
            kind: Some(kind),
            ..Default::default()
        }
    }

    fn parse_line_context(&self, line: &str, character: u32) -> LineContext {
        let char_pos = character as usize;
        let trimmed = line.trim_start();

        // Skip comments
        if trimmed.starts_with('#') {
            return LineContext::Comment;
        }

        // Find equals position
        if let Some(eq_pos) = line.find('=') {
            if char_pos <= eq_pos {
                // Cursor is before or at equals - completing key
                let key_part = &line[..char_pos];
                LineContext::Key(key_part.trim().to_string())
            } else {
                // Cursor is after equals - completing value
                let key = line[..eq_pos].trim().to_string();
                let value_part = &line[eq_pos + 1..char_pos];
                LineContext::Value {
                    key,
                    partial: value_part.trim_start().to_string(),
                }
            }
        } else {
            // No equals - completing key
            let key_part = &line[..char_pos.min(line.len())];
            LineContext::Key(key_part.trim().to_string())
        }
    }
}

#[derive(Debug)]
enum LineContext {
    Comment,
    Key(String),
    Value { key: String, partial: String },
}

#[tower_lsp::async_trait]
impl LanguageServer for GhosttyLsp {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        ..Default::default()
                    },
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec!["=".to_string(), " ".to_string()]),
                    resolve_provider: Some(false),
                    ..Default::default()
                }),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "ghostty-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Ghostty LSP initialised")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        if let Ok(mut docs) = self.documents.write() {
            docs.insert(uri, text);
        }
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.into_iter().last() {
            if let Ok(mut docs) = self.documents.write() {
                docs.insert(uri, change.text);
            }
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        if let Ok(mut docs) = self.documents.write() {
            docs.remove(&params.text_document.uri);
        }
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        // Get the document content
        let content = {
            let docs = self.documents.read().unwrap();
            docs.get(uri).cloned()
        };

        let Some(content) = content else {
            self.client
                .log_message(
                    MessageType::WARNING,
                    format!("No document content for {}", uri),
                )
                .await;
            // Fallback: return all key completions
            return Ok(Some(CompletionResponse::Array(self.get_key_completions(""))));
        };

        // Get the current line
        let lines: Vec<&str> = content.lines().collect();
        let line_num = position.line as usize;
        if line_num >= lines.len() {
            return Ok(Some(CompletionResponse::Array(self.get_key_completions(""))));
        }
        let line = lines[line_num];

        // Parse context and get completions
        let context = self.parse_line_context(line, position.character);

        let items = match context {
            LineContext::Comment => vec![],
            LineContext::Key(partial) => self.get_key_completions(&partial),
            LineContext::Value { key, partial } => self.get_value_completions(&key, &partial),
        };

        Ok(Some(CompletionResponse::Array(items)))
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(GhosttyLsp::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
