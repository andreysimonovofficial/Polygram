//! Перечень инструментов MCP и их диспетчер.

use polygram_core::{
    types::{MetricKind, NodeKind},
    Command,
};
use serde_json::{json, Value};
use std::path::Path;

use super::proxy::{proxy_get, proxy_post_command, proxy_post_json};
use super::result::{mcp_tool_error, mcp_tool_result};
use super::{McpEvent, McpServer};

pub fn handle_initialize(_params: Value) -> Value {
    json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": "polygram",
            "version": "0.1.0"
        },
        "instructions": "Polygram MCP Server — инструкция для агентов:\n\nПОРЯДОК РАБОТЫ:\n1. Сначала вызовите get_commands, чтобы увидеть полный список команд с примерами.\n2. Используйте отдельные инструменты (add_object, rename_object и т.д.) вместо execute_command — у них понятные поля.\n3. execute_command принимает произвольную JSON-команду в поле command.\n\nФОРМАТ КОМАНД:\n- Каждая команда — JSON с полем \"cmd\" (имя команды) и остальными полями в зависимости от типа.\n- Пример add_object: {\"cmd\":\"add_object\",\"name\":\"Claude\",\"kind\":\"process\"}\n\nИМЕНА:\n- Пока пишите имена объектов латиницей (поддержка кириллицы в разработке).\n- ID объекта можно не указывать — сгенерируется автоматически.\n- Позиция at=[x,y] можно не указывать — подберётся автоматически.\n\nОШИБКИ:\n- Если команда не принимается, проверьте поле cmd и обязательные поля через get_commands.\n- При неверном JSON или неизвестном cmd вернётся понятное сообщение со списком доступных команд."
    })
}

pub fn handle_tools_list() -> Value {
    json!({
        "tools": [
            {
                "name": "get_commands",
                "description": "Справочник всех команд Polygram с описанием полей и примерами. Вызовите этот инструмент первым, чтобы узнать формат команд.",
                "inputSchema": { "type": "object", "properties": {} }
            },
            {
                "name": "get_snapshot",
                "description": "Получить полный снимок состояния графа (объекты, метрики, связи).",
                "inputSchema": { "type": "object", "properties": {} }
            },
            {
                "name": "get_objects",
                "description": "Список всех объектов на холсте.",
                "inputSchema": { "type": "object", "properties": {} }
            },
            {
                "name": "get_stats",
                "description": "Статистика графа: количество объектов, связей, показателей.",
                "inputSchema": { "type": "object", "properties": {} }
            },
            {
                "name": "add_object",
                "description": "Создать объект на холсте (процесс, отдел). Имя пишите латиницей.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "Имя объекта (латиницей), например \"Claude\"" },
                        "kind": { "type": "string", "description": "Тип: process (процесс), department (отдел). По умолчанию process." },
                        "id": { "type": "string", "description": "Идентификатор объекта. Если не указан, сгенерируется автоматически." },
                        "at": { "type": "array", "description": "Позиция [x, y]. Если не указана, подбирается автоматически." },
                        "parent": { "type": "string", "description": "ID родительского отдела. null — корень." }
                    },
                    "required": ["name"]
                }
            },
            {
                "name": "rename_object",
                "description": "Переименовать существующий объект.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "object": { "type": "string", "description": "ID объекта для переименования" },
                        "name": { "type": "string", "description": "Новое имя объекта (латиницей)" }
                    },
                    "required": ["object", "name"]
                }
            },
            {
                "name": "remove_object",
                "description": "Удалить объект с холста.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "object": { "type": "string", "description": "ID объекта для удаления" }
                    },
                    "required": ["object"]
                }
            },
            {
                "name": "move_object",
                "description": "Переместить объект на новые координаты.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "object": { "type": "string", "description": "ID объекта" },
                        "at": { "type": "array", "description": "Новая позиция [x, y]" }
                    },
                    "required": ["object", "at"]
                }
            },
            {
                "name": "add_metric_def",
                "description": "Создать глобальный показатель (метрику).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "Имя показателя" },
                        "kind": { "type": "string", "description": "Тип: number, time, percentage, binary, duration, formula" },
                        "unit": { "type": "string", "description": "Единица измерения, например \"$\" или \"шт.\"" },
                        "id": { "type": "string", "description": "Идентификатор показателя. Если не указан, сгенерируется автоматически." }
                    },
                    "required": ["name", "kind"]
                }
            },
            {
                "name": "chat_post",
                "description": "Опубликовать сообщение в общую ленту чата (видно в GUI и через HTTP API).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "author": { "type": "string", "description": "Автор сообщения, например \"agent\"" },
                        "text": { "type": "string", "description": "Текст сообщения" },
                        "reply_to": { "type": "string", "description": "ID или имя адресата (опционально)" }
                    },
                    "required": ["author", "text"]
                }
            },
            {
                "name": "document_open",
                "description": "Открыть проект (.pg файл), заменив текущий граф.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Путь к .pg файлу" }
                    },
                    "required": ["path"]
                }
            },
            {
                "name": "document_save_as",
                "description": "Сохранить текущий граф в указанный .pg файл.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Путь для сохранения .pg файла" }
                    },
                    "required": ["path"]
                }
            },
            {
                "name": "execute_command",
                "description": "Выполнить произвольную команду Polygram в формате JSON. Доступные команды: add_object, rename_object, remove_object, move_object, add_metric_def, remove_metric_def, rename_metric_def, set_metric_def_unit, add_metric_entry, remove_metric_entry, set_value, set_required, set_max, set_fact, connect, disconnect, auto_layout, virtual_mouse_move, virtual_mouse_down, virtual_mouse_up, virtual_keyboard_type, virtual_draw_shape. Каждая команда имеет поле cmd с именем команды и остальные поля в зависимости от типа. Пример add_object: {\"cmd\":\"add_object\",\"name\":\"Claude\",\"kind\":\"process\"}.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "command": { "type": "string", "description": "JSON-команда, например {\"cmd\":\"add_object\",\"name\":\"Claude\",\"kind\":\"process\"}" }
                    },
                    "required": ["command"]
                }
            },
            {
                "name": "virtual_mouse_move",
                "description": "Переместить виртуальную мышь на экранные координаты [x, y] внутри приложения.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "x": { "type": "integer", "description": "Координата X курсора" },
                        "y": { "type": "integer", "description": "Координата Y курсора" }
                    },
                    "required": ["x", "y"]
                }
            },
            {
                "name": "virtual_mouse_down",
                "description": "Зажать кнопку виртуальной мыши (0 — левая, 1 — правая).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "button": { "type": "integer", "description": "Кнопка: 0 — левая, 1 — правая", "minimum": 0, "maximum": 1 }
                    },
                    "required": ["button"]
                }
            },
            {
                "name": "virtual_mouse_up",
                "description": "Отпустить кнопку виртуальной мыши (0 — левая, 1 — правая).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "button": { "type": "integer", "description": "Кнопка: 0 — левая, 1 — правая", "minimum": 0, "maximum": 1 }
                    },
                    "required": ["button"]
                }
            },
            {
                "name": "virtual_mouse_click",
                "description": "Клик виртуальной мышью (нажать и отпустить) в текущей позиции курсора.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "button": { "type": "integer", "description": "Кнопка: 0 — левая, 1 — правая", "minimum": 0, "maximum": 1 }
                    },
                    "required": ["button"]
                }
            },
            {
                "name": "virtual_mouse_drag",
                "description": "Перетащить: зажать мышь в точке [x1, y1], переместить в [x2, y2] и отпустить.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "x1": { "type": "integer", "description": "Начальная координата X" },
                        "y1": { "type": "integer", "description": "Начальная координата Y" },
                        "x2": { "type": "integer", "description": "Конечная координата X" },
                        "y2": { "type": "integer", "description": "Конечная координата Y" },
                        "button": { "type": "integer", "description": "Кнопка: 0 — левая, 1 — правая", "minimum": 0, "maximum": 1 }
                    },
                    "required": ["x1", "y1", "x2", "y2"]
                }
            },
            {
                "name": "virtual_keyboard_type",
                "description": "Ввести текст виртуальной клавиатурой (в активное поле ввода приложения).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "text": { "type": "string", "description": "Текст для ввода" }
                    },
                    "required": ["text"]
                }
            },
            {
                "name": "virtual_input_state",
                "description": "Получить текущее состояние виртуального ввода (позиция мыши, зажатые кнопки, буфер клавиатуры).",
                "inputSchema": { "type": "object", "properties": {} }
            },
            {
                "name": "virtual_draw_shape",
                "description": "Нарисовать фигуру курсором от текущей позиции мыши до [x2, y2] (мышь должна быть зажата). kind: 0 — эллипс, 1 — прямоугольник.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "kind": { "type": "integer", "description": "Тип фигуры: 0 — эллипс, 1 — прямоугольник", "minimum": 0, "maximum": 1 },
                        "x2": { "type": "integer", "description": "Конечная координата X" },
                        "y2": { "type": "integer", "description": "Конечная координата Y" }
                    },
                    "required": ["kind", "x2", "y2"]
                }
            },
            {
                "name": "virtual_mouse_curve",
                "description": "Плавное движение мыши по кривой Безье к [x, y] через контрольную точку [cx, cy].",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "x": { "type": "integer", "description": "Конечная координата X" },
                        "y": { "type": "integer", "description": "Конечная координата Y" },
                        "cx": { "type": "integer", "description": "Контрольная точка X кривой Безье" },
                        "cy": { "type": "integer", "description": "Контрольная точка Y кривой Безье" }
                    },
                    "required": ["x", "y", "cx", "cy"]
                }
            },
            {
                "name": "virtual_mouse_path",
                "description": "Движение мыши по скруглённой кривой через массив точек [[x,y], ...] с плавной анимацией.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "points": { "type": "array", "description": "Массив точек кривой [[x,y], ...]", "items": { "type": "array", "items": { "type": "integer" } } }
                    },
                    "required": ["points"]
                }
            }
        ]
    })
}

pub fn handle_tools_call(server: &McpServer, params: Value) -> Value {
    let name = match params.get("name").and_then(|v| v.as_str()) {
        Some(n) => n,
        None => return mcp_tool_error("Не указано имя инструмента. Доступные: get_commands, add_object, rename_object, remove_object, move_object, add_metric_def, chat_post, document_open, document_save_as, get_snapshot, get_objects, get_stats, execute_command, virtual_mouse_move, virtual_mouse_down, virtual_mouse_up, virtual_mouse_click, virtual_mouse_drag, virtual_mouse_curve, virtual_mouse_path, virtual_keyboard_type, virtual_input_state, virtual_draw_shape."),
    };
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    match name {
        "get_commands" => mcp_tool_result(&serde_json::to_string(&json!({
            "commands": [
                {"cmd": "add_object", "example": "{\"cmd\":\"add_object\",\"name\":\"Claude\",\"kind\":\"process\"}", "description": "Создать объект (процесс/отдел)"},
                {"cmd": "rename_object", "example": "{\"cmd\":\"rename_object\",\"object\":\"claude\",\"name\":\"Новое имя\"}", "description": "Переименовать объект"},
                {"cmd": "remove_object", "example": "{\"cmd\":\"remove_object\",\"object\":\"claude\"}", "description": "Удалить объект"},
                {"cmd": "move_object", "example": "{\"cmd\":\"move_object\",\"object\":\"claude\",\"at\":[100,200]}", "description": "Переместить объект"},
                {"cmd": "add_metric_def", "example": "{\"cmd\":\"add_metric_def\",\"name\":\"Прибыль\",\"kind\":\"number\",\"unit\":\"$\"}", "description": "Создать показатель"},
                {"cmd": "get_snapshot", "example": "{\"cmd\":\"get_snapshot\"}", "description": "Снимок состояния графа"},
                {"cmd": "get_objects", "example": "{\"cmd\":\"get_objects\"}", "description": "Список объектов"},
                {"cmd": "get_stats", "example": "{\"cmd\":\"get_stats\"}", "description": "Статистика графа"},
                {"cmd": "virtual_mouse_move", "example": "{\"cmd\":\"virtual_mouse_move\",\"x\":100,\"y\":200}", "description": "Переместить виртуальную мышь"},
                {"cmd": "virtual_mouse_down", "example": "{\"cmd\":\"virtual_mouse_down\",\"button\":0}", "description": "Зажать кнопку виртуальной мыши"},
                {"cmd": "virtual_mouse_up", "example": "{\"cmd\":\"virtual_mouse_up\",\"button\":0}", "description": "Отпустить кнопку виртуальной мыши"},
                {"cmd": "virtual_keyboard_type", "example": "{\"cmd\":\"virtual_keyboard_type\",\"text\":\"привет\"}", "description": "Ввести текст виртуальной клавиатурой"},
                {"cmd": "virtual_draw_shape", "example": "{\"cmd\":\"virtual_draw_shape\",\"kind\":0,\"x2\":400,\"y2\":300}", "description": "Нарисовать фигуру курсором"},
                {"cmd": "virtual_mouse_curve", "example": "{\"cmd\":\"virtual_mouse_curve\",\"x\":500,\"y\":400,\"cx\":300,\"cy\":100}", "description": "Кривая Безье к точке"},
                {"cmd": "virtual_mouse_path", "example": "{\"cmd\":\"virtual_mouse_path\",\"points\":[[100,100],[200,150],[300,100]]}", "description": "Движение по скруглённой кривой через точки"}
            ]
        })).unwrap()),
        "add_object" => {
            let name = match args.get("name").and_then(|v| v.as_str()) {
                Some(n) => n.to_owned(),
                None => return mcp_tool_error("Для add_object требуется поле name (имя объекта). Пример: {\"name\":\"Claude\",\"kind\":\"process\"}"),
            };
            let kind = args.get("kind").and_then(|v| v.as_str()).unwrap_or("process");
            let author = args.get("author").and_then(|v| v.as_str()).map(|s| s.to_owned());
            let cmd = Command::AddObject {
                id: args.get("id").and_then(|v| v.as_str()).map(|s| s.to_owned()),
                name,
                kind: match kind {
                    "department" => NodeKind::Department,
                    _ => NodeKind::Process,
                },
                at: args.get("at").and_then(|v| v.as_array()).map(|arr| {
                    arr.iter().filter_map(|v| v.as_i64()).map(|v| v as i32).collect::<Vec<_>>().try_into().unwrap_or([0, 0])
                }),
                parent: args.get("parent").and_then(|v| v.as_str()).map(|s| s.to_owned()),
                author,
            };
            apply_command(server, cmd, &args)
        }
        "rename_object" => {
            let object = match args.get("object").and_then(|v| v.as_str()) {
                Some(s) => s.to_owned(),
                None => return mcp_tool_error("Для rename_object требуется поле object (ID объекта)."),
            };
            let name = match args.get("name").and_then(|v| v.as_str()) {
                Some(s) => s.to_owned(),
                None => return mcp_tool_error("Для rename_object требуется поле name (новое имя)."),
            };
            let cmd = Command::RenameObject { object, name };
            apply_command(server, cmd, &args)
        }
        "remove_object" => {
            let object = match args.get("object").and_then(|v| v.as_str()) {
                Some(s) => s.to_owned(),
                None => return mcp_tool_error("Для remove_object требуется поле object (ID объекта)."),
            };
            let cmd = Command::RemoveObject { object };
            apply_command(server, cmd, &args)
        }
        "move_object" => {
            let object = match args.get("object").and_then(|v| v.as_str()) {
                Some(s) => s.to_owned(),
                None => return mcp_tool_error("Для move_object требуется поле object (ID объекта)."),
            };
            let at = match args.get("at").and_then(|v| v.as_array()) {
                Some(arr) => {
                    let coords: Vec<i32> = arr.iter().filter_map(|v| v.as_i64()).map(|v| v as i32).collect();
                    if coords.len() != 2 {
                        return mcp_tool_error("Для move_object поле at должно быть массивом из двух чисел [x, y].");
                    }
                    [coords[0], coords[1]]
                }
                None => return mcp_tool_error("Для move_object требуется поле at ([x, y])."),
            };
            let cmd = Command::MoveObject { object, at };
            apply_command(server, cmd, &args)
        }
        "add_metric_def" => {
            let name = match args.get("name").and_then(|v| v.as_str()) {
                Some(s) => s.to_owned(),
                None => return mcp_tool_error("Для add_metric_def требуется поле name (имя показателя)."),
            };
            let kind = match args.get("kind").and_then(|v| v.as_str()) {
                Some(s) => s.to_owned(),
                None => return mcp_tool_error("Для add_metric_def требуется поле kind (number, time, date, deadline, percentage, binary, duration, money, text, formula)."),
            };
            let cmd = Command::AddMetricDef {
                id: args.get("id").and_then(|v| v.as_str()).map(|s| s.to_owned()),
                name,
                kind: match kind.as_str() {
                    "time" => MetricKind::Time,
                    "date" => MetricKind::Date,
                    "deadline" => MetricKind::Deadline,
                    "percentage" => MetricKind::Percentage,
                    "binary" => MetricKind::Binary,
                    "duration" => MetricKind::Duration,
                    "money" => MetricKind::Money,
                    "text" => MetricKind::Text,
                    "formula" => MetricKind::Formula,
                    _ => MetricKind::Number,
                },
                unit: args.get("unit").and_then(|v| v.as_str()).unwrap_or("ед.").to_owned(),
                api: None,
            };
            apply_command(server, cmd, &args)
        }
        "chat_post" => {
            let author = args.get("author").and_then(|v| v.as_str()).map(|s| s.to_owned()).unwrap_or_else(|| "agent".to_owned());
            let text = match args.get("text").and_then(|v| v.as_str()) {
                Some(t) => t.to_owned(),
                None => return mcp_tool_error("Для chat_post требуется поле text (текст сообщения)."),
            };
            let reply_to = args.get("reply_to").and_then(|v| v.as_str()).map(|s| s.to_owned());
            if let Some(base) = &server.http_base {
                let body = serde_json::to_string(&json!({ "author": author, "text": text, "reply_to": reply_to })).unwrap();
                match proxy_post_json(base, "/api/chat/messages", &body) {
                    Ok(b) => mcp_tool_result(&b),
                    Err(e) => mcp_tool_error(&format!("прокси HTTP: {e}"))
                }
            } else {
                let message = server.api.chat().post_text(author, text, reply_to);
                mcp_tool_result(&serde_json::to_string(&json!({ "ok": true, "message": message })).unwrap())
            }
        }
        "document_open" => {
            let path = match args.get("path").and_then(|v| v.as_str()) {
                Some(p) => p.to_owned(),
                None => return mcp_tool_error("Для document_open требуется поле path (путь к .pg файлу)."),
            };
            if let Some(base) = &server.http_base {
                let body = serde_json::to_string(&json!({ "path": path })).unwrap();
                match proxy_post_json(base, "/api/document/open", &body) {
                    Ok(b) => mcp_tool_result(&b),
                    Err(e) => mcp_tool_error(&format!("прокси HTTP: {e}"))
                }
            } else {
                match server.api.open_project(Path::new(&path)) {
                    Ok(r) => mcp_tool_result(&serde_json::to_string(&r).unwrap()),
                    Err(e) => mcp_tool_error(&e.to_string())
                }
            }
        }
        "document_save_as" => {
            let path = match args.get("path").and_then(|v| v.as_str()) {
                Some(p) => p.to_owned(),
                None => return mcp_tool_error("Для document_save_as требуется поле path (путь для сохранения .pg файла)."),
            };
            if let Some(base) = &server.http_base {
                let body = serde_json::to_string(&json!({ "path": path })).unwrap();
                match proxy_post_json(base, "/api/document/save-as", &body) {
                    Ok(b) => mcp_tool_result(&b),
                    Err(e) => mcp_tool_error(&format!("прокси HTTP: {e}"))
                }
            } else {
                match server.api.save_project_as(Path::new(&path)) {
                    Ok(r) => mcp_tool_result(&serde_json::to_string(&r).unwrap()),
                    Err(e) => mcp_tool_error(&e.to_string())
                }
            }
        }
        "execute_command" => {
            let cmd_json = match args.get("command").and_then(|v| v.as_str()) {
                Some(s) => s,
                None => return mcp_tool_error("Для execute_command требуется поле command (JSON-команда). Доступные команды: add_object, rename_object, remove_object, move_object, add_metric_def и другие. См. get_commands."),
            };
            if let Some(base) = &server.http_base {
                match proxy_post_command(base, cmd_json) {
                    Ok(body) => {
                        let cmd_str = cmd_json.to_owned();
                        if let Some(tx) = &server.mcp_tx {
                            let _ = tx.send(McpEvent { cmd: cmd_str, result: body.clone() });
                        }
                        mcp_tool_result(&body)
                    }
                    Err(e) => {
                        let err_str = format!("прокси HTTP: {e}");
                        if let Some(tx) = &server.mcp_tx {
                            let _ = tx.send(McpEvent { cmd: cmd_json.to_owned(), result: err_str.clone() });
                        }
                        mcp_tool_error(&err_str)
                    }
                }
            } else {
                let cmd: Command = match serde_json::from_str(cmd_json) {
                    Ok(c) => c,
                    Err(e) => return mcp_tool_error(&format!("Не удалось распарсить JSON-команду: {e}. Пример: {{\"cmd\":\"add_object\",\"name\":\"Claude\",\"kind\":\"process\"}}. Доступные команды: add_object, rename_object, remove_object, move_object, add_metric_def. См. get_commands.")),
                };
                apply_command(server, cmd, &args)
            }
        }
        "virtual_mouse_move" => {
            let x = match args.get("x").and_then(|v| v.as_i64()) {
                Some(v) => v as i32,
                None => return mcp_tool_error("Для virtual_mouse_move требуется поле x (целое число)."),
            };
            let y = match args.get("y").and_then(|v| v.as_i64()) {
                Some(v) => v as i32,
                None => return mcp_tool_error("Для virtual_mouse_move требуется поле y (целое число)."),
            };
            let cmd = Command::VirtualMouseMove { x, y };
            apply_command(server, cmd, &args)
        }
        "virtual_mouse_down" => {
            let button = match args.get("button").and_then(|v| v.as_u64()) {
                Some(v) => v as u8,
                None => return mcp_tool_error("Для virtual_mouse_down требуется поле button (0 — левая, 1 — правая)."),
            };
            if button > 1 {
                return mcp_tool_error("Для virtual_mouse_down button должен быть 0 (левая) или 1 (правая).");
            }
            let cmd = Command::VirtualMouseDown { button };
            apply_command(server, cmd, &args)
        }
        "virtual_mouse_up" => {
            let button = match args.get("button").and_then(|v| v.as_u64()) {
                Some(v) => v as u8,
                None => return mcp_tool_error("Для virtual_mouse_up требуется поле button (0 — левая, 1 — правая)."),
            };
            if button > 1 {
                return mcp_tool_error("Для virtual_mouse_up button должен быть 0 (левая) или 1 (правая).");
            }
            let cmd = Command::VirtualMouseUp { button };
            apply_command(server, cmd, &args)
        }
        "virtual_mouse_click" => {
            let button = match args.get("button").and_then(|v| v.as_u64()) {
                Some(v) => v as u8,
                None => return mcp_tool_error("Для virtual_mouse_click требуется поле button (0 — левая, 1 — правая)."),
            };
            if button > 1 {
                return mcp_tool_error("Для virtual_mouse_click button должен быть 0 (левая) или 1 (правая).");
            }
            // Нажать, потом отпустить.
            let down = Command::VirtualMouseDown { button };
            let up = Command::VirtualMouseUp { button };
            if let Some(base) = &server.http_base {
                let down_json = serde_json::to_string(&down).unwrap();
                let up_json = serde_json::to_string(&up).unwrap();
                match proxy_post_command(base, &down_json) {
                    Ok(_) => {}
                    Err(e) => return mcp_tool_error(&format!("прокси HTTP (down): {e}")),
                }
                match proxy_post_command(base, &up_json) {
                    Ok(body) => mcp_tool_result(&body),
                    Err(e) => mcp_tool_error(&format!("прокси HTTP (up): {e}")),
                }
            } else {
                apply_command(server, down, &args);
                apply_command(server, up, &args);
                mcp_tool_result(&serde_json::to_string(&json!({"ok": true})).unwrap())
            }
        }
        "virtual_mouse_drag" => {
            let x1 = match args.get("x1").and_then(|v| v.as_i64()) {
                Some(v) => v as i32,
                None => return mcp_tool_error("Для virtual_mouse_drag требуется поле x1."),
            };
            let y1 = match args.get("y1").and_then(|v| v.as_i64()) {
                Some(v) => v as i32,
                None => return mcp_tool_error("Для virtual_mouse_drag требуется поле y1."),
            };
            let x2 = match args.get("x2").and_then(|v| v.as_i64()) {
                Some(v) => v as i32,
                None => return mcp_tool_error("Для virtual_mouse_drag требуется поле x2."),
            };
            let y2 = match args.get("y2").and_then(|v| v.as_i64()) {
                Some(v) => v as i32,
                None => return mcp_tool_error("Для virtual_mouse_drag требуется поле y2."),
            };
            let button = match args.get("button").and_then(|v| v.as_u64()) {
                Some(v) => v as u8,
                None => 0,
            };
            if button > 1 {
                return mcp_tool_error("Для virtual_mouse_drag button должен быть 0 (левая) или 1 (правая).");
            }
            let move_to_start = Command::VirtualMouseMove { x: x1, y: y1 };
            let down = Command::VirtualMouseDown { button };
            let move_to_end = Command::VirtualMouseMove { x: x2, y: y2 };
            let up = Command::VirtualMouseUp { button };
            if let Some(base) = &server.http_base {
                for cmd_json in [&serde_json::to_string(&move_to_start).unwrap(),
                                 &serde_json::to_string(&down).unwrap(),
                                 &serde_json::to_string(&move_to_end).unwrap(),
                                 &serde_json::to_string(&up).unwrap()] {
                    match proxy_post_command(base, cmd_json) {
                        Ok(_) => {}
                        Err(e) => return mcp_tool_error(&format!("прокси HTTP (drag): {e}")),
                    }
                }
                mcp_tool_result(&serde_json::to_string(&json!({"ok": true, "from": [x1, y1], "to": [x2, y2]})).unwrap())
            } else {
                for cmd in [move_to_start, down, move_to_end, up] {
                    apply_command(server, cmd, &args);
                }
                mcp_tool_result(&serde_json::to_string(&json!({"ok": true, "from": [x1, y1], "to": [x2, y2]})).unwrap())
            }
        }
        "virtual_keyboard_type" => {
            let text = match args.get("text").and_then(|v| v.as_str()) {
                Some(t) => t.to_owned(),
                None => return mcp_tool_error("Для virtual_keyboard_type требуется поле text (строка для ввода)."),
            };
            let cmd = Command::VirtualKeyboardType { text };
            apply_command(server, cmd, &args)
        }
        "virtual_input_state" => {
            let state = if let Some(base) = &server.http_base {
                match proxy_get(base, "/api/virtual-input") {
                    Ok(v) => v,
                    Err(e) => return mcp_tool_error(&format!("прокси HTTP: {e}")),
                }
            } else {
                let vi = server.api.virtual_input();
                serde_json::to_value(&vi).unwrap_or_default()
            };
            mcp_tool_result(&serde_json::to_string(&state).unwrap())
        }
        "virtual_draw_shape" => {
            let kind = match args.get("kind").and_then(|v| v.as_u64()) {
                Some(v) => v as u8,
                None => return mcp_tool_error("Для virtual_draw_shape требуется поле kind (0 — эллипс, 1 — прямоугольник)."),
            };
            if kind > 1 {
                return mcp_tool_error("Для virtual_draw_shape kind должен быть 0 (эллипс) или 1 (прямоугольник).");
            }
            let x2 = match args.get("x2").and_then(|v| v.as_i64()) {
                Some(v) => v as i32,
                None => return mcp_tool_error("Для virtual_draw_shape требуется поле x2."),
            };
            let y2 = match args.get("y2").and_then(|v| v.as_i64()) {
                Some(v) => v as i32,
                None => return mcp_tool_error("Для virtual_draw_shape требуется поле y2."),
            };
            let cmd = Command::VirtualDrawShape { kind, x2, y2 };
            apply_command(server, cmd, &args)
        }
        "virtual_mouse_curve" => {
            let x = match args.get("x").and_then(|v| v.as_i64()) {
                Some(v) => v as i32,
                None => return mcp_tool_error("Для virtual_mouse_curve требуется поле x."),
            };
            let y = match args.get("y").and_then(|v| v.as_i64()) {
                Some(v) => v as i32,
                None => return mcp_tool_error("Для virtual_mouse_curve требуется поле y."),
            };
            let cx = match args.get("cx").and_then(|v| v.as_i64()) {
                Some(v) => v as i32,
                None => return mcp_tool_error("Для virtual_mouse_curve требуется поле cx."),
            };
            let cy = match args.get("cy").and_then(|v| v.as_i64()) {
                Some(v) => v as i32,
                None => return mcp_tool_error("Для virtual_mouse_curve требуется поле cy."),
            };
            let cmd = Command::VirtualMouseCurve { x, y, cx, cy };
            apply_command(server, cmd, &args)
        }
        "virtual_mouse_path" => {
            let points = match args.get("points").and_then(|v| v.as_array()) {
                Some(arr) => {
                    let mut pts = Vec::new();
                    for p in arr {
                        let arr2 = match p.as_array() {
                            Some(a) => a,
                            None => return mcp_tool_error("Для virtual_mouse_path каждая точка должна быть массивом [x, y]."),
                        };
                        if arr2.len() < 2 {
                            return mcp_tool_error("Для virtual_mouse_path каждая точка должна содержать минимум 2 элемента [x, y].");
                        }
                        let x = match arr2[0].as_i64() {
                            Some(v) => v as i32,
                            None => return mcp_tool_error("Для virtual_mouse_path x должен быть целым числом."),
                        };
                        let y = match arr2[1].as_i64() {
                            Some(v) => v as i32,
                            None => return mcp_tool_error("Для virtual_mouse_path y должен быть целым числом."),
                        };
                        pts.push((x, y));
                    }
                    pts
                }
                None => return mcp_tool_error("Для virtual_mouse_path требуется поле points (массив точек [[x,y], ...])."),
            };
            let cmd = Command::VirtualMousePath { points };
            apply_command(server, cmd, &args)
        }
        "get_snapshot" => {
            if let Some(base) = &server.http_base {
                match proxy_get(base, "/api/snapshot") {
                    Ok(v) => mcp_tool_result(&serde_json::to_string(&v).unwrap()),
                    Err(e) => mcp_tool_error(&format!("{e}"))
                }
            } else {
                let pub_ = server.api.snapshot();
                mcp_tool_result(&serde_json::to_string(&json!({ "rev": pub_.rev, "dynamic": pub_.dynamic.clone(), "geometry": pub_.geometry.clone() })).unwrap())
            }
        }
        "get_objects" => {
            if let Some(base) = &server.http_base {
                match proxy_get(base, "/api/objects") {
                    Ok(v) => mcp_tool_result(&serde_json::to_string(&v).unwrap()),
                    Err(e) => mcp_tool_error(&format!("{e}"))
                }
            } else {
                let objects = server.api.objects();
                mcp_tool_result(&serde_json::to_string(&json!({ "objects": objects })).unwrap())
            }
        }
        "get_stats" => {
            if let Some(base) = &server.http_base {
                match proxy_get(base, "/api/stats") {
                    Ok(v) => mcp_tool_result(&serde_json::to_string(&v).unwrap()),
                    Err(e) => mcp_tool_error(&format!("{e}"))
                }
            } else {
                let stats = server.api.stats();
                mcp_tool_result(&serde_json::to_string(&json!({ "stats": stats })).unwrap())
            }
        }
        _ => mcp_tool_error(&format!("Неизвестный инструмент \"{name}\". Доступные: get_commands, add_object, rename_object, remove_object, move_object, add_metric_def, chat_post, document_open, document_save_as, get_snapshot, get_objects, get_stats, execute_command, virtual_mouse_move, virtual_mouse_down, virtual_mouse_up, virtual_mouse_click, virtual_mouse_drag, virtual_mouse_curve, virtual_mouse_path, virtual_keyboard_type, virtual_input_state, virtual_draw_shape.")),
    }
}

pub fn apply_command(server: &McpServer, cmd: Command, _args: &Value) -> Value {
    let cmd_str = serde_json::to_string(&cmd).unwrap();
    if let Some(base) = &server.http_base {
        match proxy_post_command(base, &cmd_str) {
            Ok(body) => {
                if let Some(tx) = &server.mcp_tx {
                    let _ = tx.send(McpEvent {
                        cmd: cmd_str,
                        result: body.clone(),
                    });
                }
                mcp_tool_result(&body)
            }
            Err(e) => {
                let err_str = format!("прокси HTTP: {e}");
                if let Some(tx) = &server.mcp_tx {
                    let _ = tx.send(McpEvent {
                        cmd: cmd_str,
                        result: err_str.clone(),
                    });
                }
                mcp_tool_error(&err_str)
            }
        }
    } else {
        match server.api.apply(cmd) {
            Ok(applied) => {
                if let Some(tx) = &server.mcp_tx {
                    let _ = tx.send(McpEvent {
                        cmd: cmd_str,
                        result: format!("OK ревизия {}", applied.rev),
                    });
                }
                mcp_tool_result(
                    &serde_json::to_string(&json!({"ok": true, "rev": applied.rev})).unwrap(),
                )
            }
            Err(e) => {
                let err_str = format!("{e:?}");
                if let Some(tx) = &server.mcp_tx {
                    let _ = tx.send(McpEvent {
                        cmd: cmd_str,
                        result: err_str.clone(),
                    });
                }
                mcp_tool_error(&err_str)
            }
        }
    }
}
