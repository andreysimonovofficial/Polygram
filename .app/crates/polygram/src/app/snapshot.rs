use crate::app::PolygramApp;
use egui::ColorImage;
use image;
use polygram_core::{geometry::Geometry, types::Direction, Published};
use serde_json::from_str;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures_util::StreamExt;

impl PolygramApp {
    /// Обработка pending_move: если сервер не применил MoveObject (CellOccupied из-за буфера 2 кванта),
    /// откатываем клиентское предсказание после короткого таймаута.
    pub fn poll_snapshot(&mut self) {
        if let Some((ref obj_id, rev_at_move)) = self.drag.pending_move {
            if self.connection.last_rev > rev_at_move {
                self.drag.predicted_positions.remove(obj_id);
                self.drag.pending_move = None;
                self.drag.pending_move_age = 0;
            } else {
                self.drag.pending_move_age += 1;
            }
        }

        while let Ok(new_snapshot) = self.connection.snapshot_rx.try_recv() {
            let new_rev = new_snapshot.rev;
            if new_rev != self.connection.last_rev {
                self.connection.last_rev = new_rev;
                // Если есть неподтверждённый реордер, наложим его на снимок от сервера,
                // чтобы локальные изменения не scomparли при обновлении снимка.
                let snapshot = if let Some((ref object, dir, from_ix, to_ix)) =
                    self.drag.pending_reorder
                {
                    let mut geometry = new_snapshot.geometry.clone();
                    if let Some(obj) = geometry.objects.iter_mut().find(|o| o.id == *object) {
                        let entries = match dir {
                            Direction::Request => &mut obj.requests,
                            Direction::Result => &mut obj.results,
                        };
                        if from_ix < entries.len() && to_ix < entries.len() && from_ix != to_ix {
                            let entry = entries.remove(from_ix);
                            entries.insert(to_ix, entry);
                        }
                    }
                    Arc::new(polygram_core::Published {
                        geometry,
                        ..new_snapshot.as_ref().clone()
                    })
                } else {
                    new_snapshot
                };
                self.connection.snapshot = Some(snapshot);
                self.connection.api_connected = true;
                self.connection.api_error = None;
                self.last_status = format!("Ревизия {}", self.connection.last_rev);
                self.add_connection_log(true, &format!("Снимок получен, ревизия {}", new_rev));
                if let Some((ref obj_id, rev_at_move)) = self.drag.pending_move {
                    if new_rev > rev_at_move {
                        self.drag.predicted_positions.remove(obj_id);
                        self.drag.pending_move = None;
                        self.drag.pending_move_age = 0;
                    }
                }
                // Сбрасываем pending_reorder, когда сервер подтвердил новую ревизию.
                if self.drag.pending_reorder.is_some() {
                    self.drag.pending_reorder = None;
                }
            }
        }
    }

    pub fn try_connect_once(&mut self) {
        match self.connection.try_connect_once() {
            crate::app::connection::ConnectionResult::Connected { rev } => {
                self.last_status = format!("Ревизия {}", rev);
                self.add_connection_log(true, &format!("Подключено к API, ревизия {}", rev));
            }
            crate::app::connection::ConnectionResult::Failed { error } => {
                self.last_status = format!("API недоступен: {error}");
                self.add_connection_log(false, &format!("Ошибка подключения: {error}"));
            }
        }
    }

    pub fn start_ws_client(
        api_base: String,
        tx: mpsc::Sender<Arc<Published>>,
    ) {
        let ws_url = api_base.replace("http://", "ws://").replace("https://", "wss://") + "/api/ws";
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!("Не удалось создать tokio runtime для WS клиента: {e}");
                    return;
                }
            };
            rt.block_on(async {
                loop {
                    match connect_async(&ws_url).await {
                        Ok((ws_stream, _)) => {
                            eprintln!("WebSocket подключён: {ws_url}");
                            let (_, mut read) = ws_stream.split();
                            while let Some(msg) = read.next().await {
                                match msg {
                                    Ok(Message::Text(text)) => {
                                        if let Ok(published) = from_str::<Published>(&text) {
                                            let _ = tx.send(Arc::new(published));
                                        }
                                    }
                                    Ok(Message::Binary(bin)) => {
                                        if let Ok(published) = from_str::<Published>(&String::from_utf8_lossy(&bin)) {
                                            let _ = tx.send(Arc::new(published));
                                        }
                                    }
                                    Ok(Message::Close(_)) => {
                                        eprintln!("WebSocket закрыт сервером");
                                        break;
                                    }
                                    Err(e) => {
                                        eprintln!("WebSocket ошибка: {e}");
                                        break;
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Не удалось подключиться к WebSocket: {e}");
                        }
                    }
                    // Переподключение через секунду при разрыве.
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            });
        });
    }

    pub fn geometry_with_predictions(&self, published: &Published) -> Geometry {
        crate::app::departments::geometry_with_predictions(published, &self.drag)
    }
}

/// Сохранить снимок экрана egui ColorImage в PNG.
pub fn save_screenshot(image: &ColorImage, dir: &PathBuf) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;

    let ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    let path = dir.join(format!("screenshot_{}.png", ts));

    let width = image.width() as u32;
    let height = image.height() as u32;
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for pixel in &image.pixels {
        let [r, g, b, a] = pixel.to_array();
        rgba.push(r);
        rgba.push(g);
        rgba.push(b);
        rgba.push(a);
    }

    let img = image::RgbaImage::from_raw(width, height, rgba)
        .ok_or("Failed to create RgbaImage from screenshot pixels")?;
    img.save(&path)
        .map_err(|e| format!("Failed to save screenshot PNG: {}", e))?;

    Ok(path)
}
