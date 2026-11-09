//! Лента чата Polygram: общая для всех участников (GUI, Claude, Qwen, HTTP-клиенты).
//! Хранится в памяти внутри ApiService (MVP — без персистентности между перезапусками).

use polygram_core::types::NodeKind;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

pub const CHAT_CAPACITY: usize = 500;

/// Структурное описание созданного объекта для отображения в чате.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatObject {
    pub id: String,
    pub name: String,
    pub kind: NodeKind,
    pub at: [i32; 2],
}

/// Одно сообщение в общей ленте.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: u64,
    pub author: String,
    pub text: String,
    /// Автор, которому адресован ответ (опционально).
    pub reply_to: Option<String>,
    pub ts: u64,
    pub object: Option<ChatObject>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CreateChatMessage {
    pub author: String,
    pub text: String,
    #[serde(default)]
    pub reply_to: Option<String>,
}

pub struct ChatStore {
    messages: VecDeque<ChatMessage>,
    next_id: u64,
}

impl ChatStore {
    pub fn new() -> Self {
        Self {
            messages: VecDeque::new(),
            next_id: 1,
        }
    }

    pub fn push(&mut self, message: ChatMessage) {
        self.messages.push_back(message);
        while self.messages.len() > CHAT_CAPACITY {
            self.messages.pop_front();
        }
    }

    pub fn messages(&self) -> Vec<ChatMessage> {
        self.messages.iter().cloned().collect()
    }

    fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn post_text(&mut self, author: String, text: String, reply_to: Option<String>) -> ChatMessage {
        let message = ChatMessage {
            id: self.next_id(),
            author,
            text,
            reply_to,
            ts: now_ts(),
            object: None,
        };
        self.push(message.clone());
        message
    }

    pub fn note_object_created(
        &mut self,
        author: String,
        name: String,
        object_id: String,
        kind: NodeKind,
        at: [i32; 2],
    ) -> ChatMessage {
        let text = format!("{} создал объект «{}» в точке [{}, {}]", author, name, at[0], at[1]);
        let message = ChatMessage {
            id: self.next_id(),
            author,
            text,
            reply_to: None,
            ts: now_ts(),
            object: Some(ChatObject { id: object_id, name, kind, at }),
        };
        self.push(message.clone());
        message
    }
}

impl Default for ChatStore {
    fn default() -> Self {
        Self::new()
    }
}

fn now_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Обёртка над хранилище чата, разделяемая между HTTP-обработчиками.
#[derive(Clone, Default)]
pub struct ChatService {
    pub inner: Arc<Mutex<ChatStore>>,
}

impl ChatService {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(ChatStore::new())),
        }
    }

    pub fn messages(&self) -> Vec<ChatMessage> {
        self.inner.lock().unwrap().messages()
    }

    pub fn post_text(&self, author: String, text: String, reply_to: Option<String>) -> ChatMessage {
        self.inner.lock().unwrap().post_text(author, text, reply_to)
    }

    pub fn note_object_created(
        &self,
        author: String,
        name: String,
        object_id: String,
        kind: NodeKind,
        at: [i32; 2],
    ) -> ChatMessage {
        self.inner
            .lock()
            .unwrap()
            .note_object_created(author, name, object_id, kind, at)
    }
}
