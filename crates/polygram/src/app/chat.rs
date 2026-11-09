//! Панель чата Polygram: общая лента сообщений всех участников.
//! Опрос GET /api/chat/messages каждые ~2 секунды, отправка — POST /api/chat/messages.

use crate::app::PolygramApp;
use serde::{Deserialize, Serialize};
use std::sync::mpsc;
use std::time::Duration;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatObject {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub at: [i32; 2],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: u64,
    pub author: String,
    pub text: String,
    #[serde(default)]
    pub reply_to: Option<String>,
    pub ts: u64,
    #[serde(default)]
    pub object: Option<ChatObject>,
}

#[derive(Deserialize, Serialize)]
pub struct SendChatMessage {
    pub author: String,
    pub text: String,
    #[serde(default)]
    pub reply_to: Option<String>,
}
pub struct ChatState {
    pub messages: Vec<ChatMessage>,
    pub input: String,
    pub reply_to: Option<String>,
    pub last_id: u64,
    pub chat_rx: mpsc::Receiver<Vec<ChatMessage>>,
}

impl Default for ChatState {
    fn default() -> Self {
        let (_, rx) = mpsc::channel();
        Self {
            messages: Vec::new(),
            input: String::new(),
            reply_to: None,
            last_id: 0,
            chat_rx: rx,
        }
    }
}

impl ChatState {
    pub fn start_polling(api_client: reqwest::blocking::Client, api_base: String) -> mpsc::Receiver<Vec<ChatMessage>> {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || loop {
            let result = (|| -> Result<Vec<ChatMessage>, Box<dyn std::error::Error + Send + Sync>> {
                let resp = api_client.get(format!("{api_base}/api/chat/messages")).send()?;
                if !resp.status().is_success() {
                    return Err(format!("HTTP {}", resp.status()).into());
                }
                Ok(resp.json()?)
            })();
            if let Ok(messages) = result {
                let _ = tx.send(messages);
            }
            std::thread::sleep(Duration::from_secs(2));
        });
        rx
    }

    pub fn poll(&mut self) {
        while let Ok(messages) = self.chat_rx.try_recv() {
            if messages.is_empty() {
                continue;
            }
            let max_id = messages.iter().map(|m| m.id).max().unwrap_or(0);
            if max_id <= self.last_id {
                continue;
            }
            for m in &messages {
                if m.id > self.last_id {
                    self.messages.push(m.clone());
                }
            }
            self.last_id = max_id;
            if self.messages.len() > 500 {
                self.messages.drain(0..(self.messages.len() - 500));
            }
        }
    }

    pub fn send_message(&mut self, api_base: String, client: reqwest::blocking::Client) {
        let text = self.input.trim().to_owned();
        if text.is_empty() {
            return;
        }
        let author = "user".to_owned();
        let reply_to = self.reply_to.take();
        self.input.clear();

        let body = serde_json::to_string(&SendChatMessage {
            author: author.clone(),
            text,
            reply_to,
        })
        .unwrap_or_default();

        std::thread::spawn(move || {
            let _ = client
                .post(format!("{api_base}/api/chat/messages"))
                .header("Content-Type", "application/json")
                .body(body)
                .send();
        });
    }
}

pub fn author_color(author: &str) -> egui::Color32 {
    match author {
        "user" => egui::Color32::from_rgb(76, 175, 80),
        "claude" => egui::Color32::from_rgb(33, 150, 243),
        "qwen" => egui::Color32::from_rgb(156, 39, 176),
        "gpt" => egui::Color32::from_rgb(255, 152, 0),
        "gemini" => egui::Color32::from_rgb(0, 150, 136),
        "http" => egui::Color32::from_rgb(158, 158, 158),
        _ => egui::Color32::from_rgb(158, 158, 158),
    }
}

pub fn author_label(author: &str) -> &str {
    match author {
        "user" => "Вы",
        "claude" => "Claude",
        "qwen" => "Qwen",
        "gpt" => "GPT",
        "gemini" => "Gemini",
        "http" => "HTTP",
        _ => author,
    }
}

pub fn render(app: &mut PolygramApp, ui: &mut egui::Ui) {
    app.chat.poll();

    ui.heading("Чат");
    ui.separator();

    egui::ScrollArea::vertical()
        .max_height(260.0)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for message in &app.chat.messages {
                let color = author_color(&message.author);
                let label = author_label(&message.author);
                let reply_hint = message
                    .reply_to
                    .as_ref()
                    .map(|r| format!(" (ответ {}", author_label(r)))
                    .unwrap_or_default();

                ui.horizontal(|ui| {
                    ui.colored_label(color, format!("{}{}:", label, reply_hint));
                    ui.label(&message.text);
                });
                if let Some(obj) = &message.object {
                    ui.colored_label(
                        color,
                        format!(
                            "  └ объект «{}» ({}) в точке [{}, {}]",
                            obj.name, obj.kind, obj.at[0], obj.at[1]
                        ),
                    );
                }
                // Кнопка «Ответить» — помечает ввод как ответ этому автору.
                if ui.button("Ответить").clicked() {
                    app.chat.reply_to = Some(message.author.clone());
                    app.chat.input = format!("@{} ", message.author);
                }
                ui.separator();
            }
        });

    ui.separator();
    ui.horizontal(|ui| {
        ui.label("Ответить:");
        if let Some(ref r) = app.chat.reply_to {
            ui.colored_label(egui::Color32::from_rgb(200, 200, 0), format!("автор {}", author_label(r)));
            if ui.button("✕").clicked() {
                app.chat.reply_to = None;
            }
        }
    });

    ui.horizontal(|ui| {
        let response = ui.text_edit_singleline(&mut app.chat.input);
        if response.lost_focus() {
            app.chat.input = app.chat.input.trim_end().to_owned();
        }
        if ui.button("Отправить").clicked()
            || (response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
        {
            app.chat
                .send_message(app.connection.api_base.clone(), app.connection.api_client.clone());
        }
        if !app.chat.input.is_empty() && ui.button("Очистить").clicked() {
            app.chat.input.clear();
        }
    });
}
