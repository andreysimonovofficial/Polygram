use crate::app::PolygramApp;
use polygram_core::Command;
use serde_json::{to_string, Value};

impl PolygramApp {
    pub fn apply_cmd(&mut self, cmd: Command) {
        let cmd_str = to_string(&cmd).unwrap_or_default();
        let api_base = self.connection.api_base.clone();
        let client = self.connection.api_client.clone();

        let cmd_label = match &cmd {
            Command::AddObject { name, kind, at, parent, .. } => {
                format!("AddObject(name={name}, kind={kind:?}, at={at:?}, parent={parent:?})")
            }
            Command::RemoveObject { object } => format!("RemoveObject(object={object})"),
            Command::RenameObject { object, name } => format!("RenameObject(object={object}, name={name})"),
            Command::MoveObject { object, at } => format!("MoveObject(object={object}, at={at:?})"),
            Command::SetObjectParent { object, parent } => {
                format!("SetObjectParent(object={object}, parent={parent:?})")
            }
            Command::AddMetricDef { name, kind, unit, .. } => {
                format!("AddMetricDef(name={name}, kind={kind:?}, unit={unit})")
            }
            Command::RemoveMetricDef { metric_def } => format!("RemoveMetricDef(metric_def={metric_def})"),
            Command::RenameMetricDef { metric_def, name } => {
                format!("RenameMetricDef(metric_def={metric_def}, name={name})")
            }
            Command::SetMetricDefUnit { metric_def, unit } => {
                format!("SetMetricDefUnit(metric_def={metric_def}, unit={unit})")
            }
            Command::AddMetricEntry { object, dir, id, metric_def, name, .. } => {
                format!("AddMetricEntry(object={object}, dir={dir:?}, id={id:?}, metric_def={metric_def}, name={name})")
            }
            Command::RemoveMetricEntry { object, entry } => {
                format!("RemoveMetricEntry(object={object}, entry={entry})")
            }
            Command::SetValue { object, entry, value } => {
                format!("SetValue(object={object}, entry={entry}, value={value:?})")
            }
            Command::SetRequired { object, entry, required } => {
                format!("SetRequired(object={object}, entry={entry}, required={required:?})")
            }
            Command::SetMax { object, entry, max } => {
                format!("SetMax(object={object}, entry={entry}, max={max:?})")
            }
            Command::SetFact { object, entry, fact } => {
                format!("SetFact(object={object}, entry={entry}, fact={fact:?})")
            }
            Command::Connect { from_object, from_entry, to_object, to_entry, throughput } => {
                format!("Connect({from_object}.{from_entry} → {to_object}.{to_entry}, t={throughput})")
            }
            Command::Disconnect { from_object, from_entry, to_object, to_entry } => {
                format!("Disconnect({from_object}.{from_entry} → {to_object}.{to_entry})")
            }
            Command::AutoLayout => "AutoLayout".to_owned(),
            Command::RerouteAll => "RerouteAll".to_owned(),
            Command::RestartServer => "RestartServer".to_owned(),
            Command::ChangePort { port } => format!("ChangePort(port={port})"),
            Command::Shutdown => "Shutdown".to_owned(),
            Command::ReloadConfig => "ReloadConfig".to_owned(),
        };

        self.add_event_log("info", "command", &format!("Отправлена команда: {cmd_label}"), Some(&cmd_str));

        std::thread::spawn(move || {
            let result = client.post(format!("{api_base}/api/commands")).json(&cmd).send();

            match result {
                Ok(resp) => {
                    let status = resp.status();
                    let body: Value = resp.json().unwrap_or_default();
                    let ok = body
                        .get("ok")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(status.is_success());
                    let result_str = if ok {
                        format!("OK (HTTP {status})")
                    } else {
                        let error = body
                            .get("error")
                            .and_then(|v| v.as_str())
                            .unwrap_or("неизвестная ошибка");
                        format!("Ошибка HTTP {status}: {error}")
                    };
                    eprintln!("Команда {cmd_str}: {result_str}");
                }
                Err(e) => {
                    eprintln!("Не удалось отправить команду в API: {e}");
                }
            }
        });

        if self.commands.command_log.len() > 200 {
            self.commands.command_log.remove(0);
        }
    }

    pub fn apply_batch(&mut self, cmds: Vec<Command>) {
        let cmd_str = to_string(&cmds).unwrap_or_default();
        let api_base = self.connection.api_base.clone();
        let client = self.connection.api_client.clone();

        let labels: Vec<String> = cmds.iter().map(|cmd| match cmd {
            Command::AddObject { name, kind, at, parent, .. } => {
                format!("AddObject(name={name}, kind={kind:?}, at={at:?}, parent={parent:?})")
            }
            Command::RemoveObject { object } => format!("RemoveObject(object={object})"),
            Command::RenameObject { object, name } => format!("RenameObject(object={object}, name={name})"),
            Command::MoveObject { object, at } => format!("MoveObject(object={object}, at={at:?})"),
            Command::SetObjectParent { object, parent } => {
                format!("SetObjectParent(object={object}, parent={parent:?})")
            }
            Command::AddMetricDef { name, kind, unit, .. } => {
                format!("AddMetricDef(name={name}, kind={kind:?}, unit={unit})")
            }
            Command::RemoveMetricDef { metric_def } => format!("RemoveMetricDef(metric_def={metric_def})"),
            Command::RenameMetricDef { metric_def, name } => {
                format!("RenameMetricDef(metric_def={metric_def}, name={name})")
            }
            Command::SetMetricDefUnit { metric_def, unit } => {
                format!("SetMetricDefUnit(metric_def={metric_def}, unit={unit})")
            }
            Command::AddMetricEntry { object, dir, id, metric_def, name, .. } => {
                format!("AddMetricEntry(object={object}, dir={dir:?}, id={id:?}, metric_def={metric_def}, name={name})")
            }
            Command::RemoveMetricEntry { object, entry } => {
                format!("RemoveMetricEntry(object={object}, entry={entry})")
            }
            Command::SetValue { object, entry, value } => {
                format!("SetValue(object={object}, entry={entry}, value={value:?})")
            }
            Command::SetRequired { object, entry, required } => {
                format!("SetRequired(object={object}, entry={entry}, required={required:?})")
            }
            Command::SetMax { object, entry, max } => {
                format!("SetMax(object={object}, entry={entry}, max={max:?})")
            }
            Command::SetFact { object, entry, fact } => {
                format!("SetFact(object={object}, entry={entry}, fact={fact:?})")
            }
            Command::Connect { from_object, from_entry, to_object, to_entry, throughput } => {
                format!("Connect({from_object}.{from_entry} → {to_object}.{to_entry}, t={throughput})")
            }
            Command::Disconnect { from_object, from_entry, to_object, to_entry } => {
                format!("Disconnect({from_object}.{from_entry} → {to_object}.{to_entry})")
            }
            Command::AutoLayout => "AutoLayout".to_owned(),
            Command::RerouteAll => "RerouteAll".to_owned(),
            Command::RestartServer => "RestartServer".to_owned(),
            Command::ChangePort { port } => format!("ChangePort(port={port})"),
            Command::Shutdown => "Shutdown".to_owned(),
            Command::ReloadConfig => "ReloadConfig".to_owned(),
        }).collect();

        let detail = format!("batch[{}]", labels.join("; "));
        self.add_event_log("info", "command", "Отправлена батч-команда", Some(&detail));

        std::thread::spawn(move || {
            let result = client
                .post(format!("{api_base}/api/commands/batch"))
                .json(&cmds)
                .send();

            match result {
                Ok(resp) => {
                    let status = resp.status();
                    let body: Value = resp.json().unwrap_or_default();
                    let ok = body
                        .get("ok")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(status.is_success());
                    let result_str = if ok {
                        format!("OK (HTTP {status})")
                    } else {
                        let error = body
                            .get("error")
                            .and_then(|v| v.as_str())
                            .unwrap_or("неизвестная ошибка");
                        format!("Ошибка HTTP {status}: {error}")
                    };
                    eprintln!("Команды {cmd_str}: {result_str}");
                }
                Err(e) => {
                    eprintln!("Не удалось отправить команды в API: {e}");
                }
            }
        });
    }
}
