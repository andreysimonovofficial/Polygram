//! Агент анализа UI-скриншота → фигуры Design Tool.
//! Отправляет снимок экрана в LLM (Gemini или локальный Qwen),
//! получает описание элементов интерфейса и преобразует в FreeShape.

use crate::design_format::{DrawTool, FreeShape, Rgba, TextAlign};
use crate::free_canvas::FreeCanvas;
use base64::Engine as _;
use serde::Deserialize;

/// Провайдер LLM для анализа скриншота.
#[derive(Clone, Debug, PartialEq)]
pub enum LlmProvider {
    Gemini { api_key: String },
    Qwen { endpoint: String, api_key: Option<String> },
}

impl Default for LlmProvider {
    fn default() -> Self {
        LlmProvider::Gemini { api_key: String::new() }
    }
}

/// Элемент интерфейса, обнаруженный LLM.
#[derive(Clone, Debug, Deserialize)]
pub struct UiElement {
    pub kind: String, // "rect", "text", "ellipse", "line"
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub fill: Option<[u8; 4]>,
    pub stroke: Option<[u8; 4]>,
    pub stroke_width: Option<f32>,
    pub text: Option<String>,
    pub font_size: Option<f32>,
    pub text_align: Option<String>,
}

/// Результат анализа.
pub struct AnalysisResult {
    pub elements: Vec<UiElement>,
    pub image_size: (u32, u32),
}

/// Агент: скриншот UI → фигуры.
pub struct UiAgent {
    pub provider: LlmProvider,
}

impl UiAgent {
    pub fn new(provider: LlmProvider) -> Self {
        Self { provider }
    }

    /// Проанализировать изображение и вернуть список элементов (блокирующий вызов).
    pub fn analyze(&self, img: &image::RgbaImage) -> Result<AnalysisResult, String> {
        let (width, height) = img.dimensions();
        let png_bytes = Self::encode_png(img);
        let base64_img = base64::engine::general_purpose::STANDARD.encode(&png_bytes);
        let prompt = Self::build_prompt(width, height);

        match &self.provider {
            LlmProvider::Gemini { api_key } => Self::call_gemini(api_key, &base64_img, &prompt),
            LlmProvider::Qwen { endpoint, api_key } => {
                Self::call_qwen(endpoint, api_key.as_deref(), &base64_img, &prompt)
            }
        }
    }

    /// Применить результат анализа к холсту.
    pub fn apply_to_canvas(canvas: &mut FreeCanvas, result: &AnalysisResult) {
        canvas.save_undo_state();
        canvas.doc.shapes.clear();
        canvas.selected.clear();

        for (i, elem) in result.elements.iter().enumerate() {
            let shape = Self::element_to_shape(elem, i);
            canvas.doc.shapes.push(shape);
        }
        canvas.touch();
    }

    /// Кодировать изображение в PNG → base64.
    fn encode_png(img: &image::RgbaImage) -> Vec<u8> {
        let mut buf = Vec::new();
        let _ = img.write_to(
            &mut std::io::Cursor::new(&mut buf),
            image::ImageFormat::Png,
        );
        buf
    }

    /// Построить промпт для LLM.
    fn build_prompt(width: u32, height: u32) -> String {
        format!(
            r#"Ты — агент анализа UI-скриншота для инструмента дизайна Polygram.
Посмотри на изображение ({}×{} px) и найди все элементы интерфейса:
прямоугольники (кнопки, карточки, панели, фоновые блоки), текст (заголовки, метки, подписи),
эллипсы (кружки, значки, аватары), линии (разделители, границы, соединители).

Верни ответ СТРОГО в формате JSON массива объектов. Каждый объект:
{{
  "kind": "rect" | "text" | "ellipse" | "line",
  "x": число (левая граница в пикселях),
  "y": число (верхняя граница в пикселях),
  "w": число (ширина),
  "h": число (высота),
  "fill": [R, G, B, A] или null,
  "stroke": [R, G, B, A] или null,
  "stroke_width": число или null,
  "text": строка или null (для kind="text"),
  "font_size": число или null,
  "text_align": "left" | "center" | "right" или null
}}

Правила:
- Координаты в пикселях от левого верхнего угла изображения.
- Цвета в формате [R, G, B, A] от 0 до 255.
- Для rect с fill — прямоугольник с заливкой.
- Для text — текстовая фигура с содержимым.
- Для ellipse — эллипс с заливкой/обводкой.
- Для line — линия от (x, y) до (x+w, y+h).
- Не выдумывай элементы, которых нет. Только реально видимые.
- Объединяй соседние одинаковые блоки в один rect.
- Верни ТОЛЬКО JSON, без markdown-разметки и пояснений."#,
            width, height
        )
    }

    /// Вызов Gemini API (блокирующий).
    fn call_gemini(
        api_key: &str,
        base64_img: &str,
        prompt: &str,
    ) -> Result<AnalysisResult, String> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent?key={}",
            api_key
        );

        let request = serde_json::json!({
            "contents": [{
                "parts": [
                    {"text": prompt},
                    {"inline_data": {
                        "mime_type": "image/png",
                        "data": base64_img
                    }}
                ]
            }],
            "generation_config": {
                "temperature": 0.1,
                "max_output_tokens": 4096
            }
        });

        let client = reqwest::blocking::Client::new();
        let resp = client
            .post(&url)
            .json(&request)
            .send()
            .map_err(|e| format!("Gemini request error: {e}"))?;

        let status = resp.status();
        let body: serde_json::Value = resp
            .json()
            .map_err(|e| format!("Gemini response parse error: {e}"))?;

        if !status.is_success() {
            return Err(format!("Gemini API error {}: {:?}", status, body));
        }

        let text = body
            .get("candidates")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("content"))
            .and_then(|c| c.get("parts"))
            .and_then(|p| p.get(0))
            .and_then(|p| p.get("text"))
            .and_then(|t| t.as_str())
            .ok_or_else(|| "Gemini: no text in response".to_owned())?;

        let elements = Self::parse_json_response(text)?;
        Ok(AnalysisResult { elements, image_size: (0, 0) })
    }

    /// Вызов Qwen (OpenAI-совместимый API, блокирующий).
    fn call_qwen(
        endpoint: &str,
        api_key: Option<&str>,
        base64_img: &str,
        prompt: &str,
    ) -> Result<AnalysisResult, String> {
        let url = if endpoint.ends_with('/') {
            format!("{}chat/completions", endpoint)
        } else {
            format!("{}/chat/completions", endpoint)
        };

        let request = serde_json::json!({
            "model": "qwen-vl-plus",
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text", "text": prompt},
                    {"type": "image_url", "image_url": {
                        "url": format!("data:image/png;base64,{}", base64_img)
                    }}
                ]
            }],
            "temperature": 0.1,
            "max_tokens": 4096
        });

        let client = reqwest::blocking::Client::new();
        let mut req_builder = client.post(&url).json(&request);
        if let Some(key) = api_key {
            req_builder = req_builder.bearer_auth(key);
        }

        let resp = req_builder
            .send()
            .map_err(|e| format!("Qwen request error: {e}"))?;

        let status = resp.status();
        let body: serde_json::Value = resp
            .json()
            .map_err(|e| format!("Qwen response parse error: {e}"))?;

        if !status.is_success() {
            return Err(format!("Qwen API error {}: {:?}", status, body));
        }

        let text = body
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .ok_or_else(|| "Qwen: no content in response".to_owned())?;

        let elements = Self::parse_json_response(text)?;
        Ok(AnalysisResult { elements, image_size: (0, 0) })
    }

    /// Распарсить JSON-ответ LLM в список элементов.
    fn parse_json_response(text: &str) -> Result<Vec<UiElement>, String> {
        // Удаляем возможные markdown-обёртки
        let cleaned = text
            .trim()
            .strip_prefix("```json")
            .unwrap_or(text.trim())
            .strip_suffix("```")
            .unwrap_or(text.trim())
            .trim();

        let elements: Vec<UiElement> = serde_json::from_str(cleaned)
            .map_err(|e| format!("JSON parse error: {e}. Response: {}", &cleaned[..cleaned.len().min(200)]))?;
        Ok(elements)
    }

    /// Преобразовать элемент LLM в FreeShape.
    fn element_to_shape(elem: &UiElement, index: usize) -> FreeShape {
        let mut shape = FreeShape::default();
        shape.id = format!("ui-{}", index);
        shape.x = elem.x;
        shape.y = elem.y;
        shape.w = elem.w.max(1.0);
        shape.h = elem.h.max(1.0);

        match elem.kind.as_str() {
            "rect" => {
                shape.tool = DrawTool::Rect;
                shape.fill = elem.fill.map(|c| Rgba::from_rgba(c[0], c[1], c[2], c[3]));
                if let Some(s) = elem.stroke {
                    shape.stroke = Rgba::from_rgba(s[0], s[1], s[2], s[3]);
                }
                shape.stroke_width = elem.stroke_width.unwrap_or(1.0);
            }
            "text" => {
                shape.tool = DrawTool::Text;
                shape.fill = elem.fill.map(|c| Rgba::from_rgba(c[0], c[1], c[2], c[3]));
                shape.text = elem.text.clone().unwrap_or_default();
                shape.font_size = elem.font_size.unwrap_or(14.0);
                shape.text_color = elem
                    .fill
                    .map(|c| Rgba::from_rgba(c[0], c[1], c[2], c[3]))
                    .unwrap_or_else(|| Rgba::from_rgb(230, 230, 235));
                shape.text_align = match elem.text_align.as_deref() {
                    Some("center") => TextAlign::Center,
                    Some("right") => TextAlign::Right,
                    _ => TextAlign::Left,
                };
                if let Some(s) = elem.stroke {
                    shape.stroke = Rgba::from_rgba(s[0], s[1], s[2], s[3]);
                }
                shape.stroke_width = elem.stroke_width.unwrap_or(0.0);
            }
            "ellipse" => {
                shape.tool = DrawTool::Ellipse;
                shape.fill = elem.fill.map(|c| Rgba::from_rgba(c[0], c[1], c[2], c[3]));
                if let Some(s) = elem.stroke {
                    shape.stroke = Rgba::from_rgba(s[0], s[1], s[2], s[3]);
                }
                shape.stroke_width = elem.stroke_width.unwrap_or(1.0);
            }
            "line" => {
                shape.tool = DrawTool::Line;
                shape.fill = None;
                if let Some(s) = elem.stroke {
                    shape.stroke = Rgba::from_rgba(s[0], s[1], s[2], s[3]);
                }
                shape.stroke_width = elem.stroke_width.unwrap_or(2.0);
            }
            _ => {
                shape.tool = DrawTool::Rect;
                shape.fill = elem.fill.map(|c| Rgba::from_rgba(c[0], c[1], c[2], c[3]));
                if let Some(s) = elem.stroke {
                    shape.stroke = Rgba::from_rgba(s[0], s[1], s[2], s[3]);
                }
                shape.stroke_width = elem.stroke_width.unwrap_or(1.0);
            }
        }

        shape
    }
}
