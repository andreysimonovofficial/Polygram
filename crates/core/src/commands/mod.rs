use crate::engine::Engine;
use crate::{command::Applied, command::Command, error::Result};

mod auto_layout;
mod edges;
mod layout;
mod metric_defs;
mod metric_entries;
mod objects;
mod reroute_all;

pub use auto_layout::handle_auto_layout;
pub use edges::{handle_connect, handle_disconnect};
pub use layout::handle_move_object;
pub use metric_defs::{handle_add_metric_def, handle_remove_metric_def, handle_rename_metric_def, handle_set_metric_def_unit};
pub use metric_entries::{
    handle_add_metric_entry, handle_remove_metric_entry, handle_set_fact, handle_set_max,
    handle_set_required, handle_set_value,
};
pub use objects::{handle_add_object, handle_remove_object, handle_rename_object, handle_set_object_parent};
pub use reroute_all::handle_reroute_all;

pub fn exec(engine: &mut Engine, cmd: Command) -> Result<crate::command::Applied> {
    match cmd {
        Command::AddObject {
            id,
            name,
            kind,
            at,
            parent,
            author,
        } => handle_add_object(engine, id, name, kind, at, parent, author),
        Command::RemoveObject { object } => handle_remove_object(engine, &object),
        Command::RenameObject { object, name } => handle_rename_object(engine, &object, name),
        Command::SetObjectParent { object, parent } => {
            handle_set_object_parent(engine, &object, parent)
        }
        Command::MoveObject { object, at } => handle_move_object(engine, &object, at),
        Command::AddMetricDef {
            id,
            name,
            kind,
            unit,
            api,
        } => handle_add_metric_def(engine, id, name, kind, unit, api),
        Command::RemoveMetricDef { metric_def } => handle_remove_metric_def(engine, &metric_def),
        Command::RenameMetricDef { metric_def, name } => {
            handle_rename_metric_def(engine, &metric_def, name)
        }
        Command::SetMetricDefUnit { metric_def, unit } => {
            handle_set_metric_def_unit(engine, &metric_def, unit)
        }
        Command::AddMetricEntry {
            object,
            dir,
            id,
            metric_def,
            name: _,
            value,
            required,
            max,
            fact,
        } => handle_add_metric_entry(
            engine,
            &object,
            dir,
            id,
            &metric_def,
            value,
            required,
            max,
            fact,
        ),
        Command::RemoveMetricEntry { object, entry } => {
            handle_remove_metric_entry(engine, &object, &entry)
        }
        Command::SetValue {
            object,
            entry,
            value,
        } => handle_set_value(engine, &object, &entry, value),
        Command::SetRequired {
            object,
            entry,
            required,
        } => handle_set_required(engine, &object, &entry, required),
        Command::SetMax { object, entry, max } => handle_set_max(engine, &object, &entry, max),
        Command::SetFact {
            object,
            entry,
            fact,
        } => handle_set_fact(engine, &object, &entry, fact),
        Command::Connect {
            from_object,
            from_entry,
            to_object,
            to_entry,
            throughput,
        } => handle_connect(
            engine,
            &from_object,
            &from_entry,
            &to_object,
            &to_entry,
            throughput,
        ),
        Command::Disconnect {
            from_object,
            from_entry,
            to_object,
            to_entry,
        } => handle_disconnect(engine, &from_object, &from_entry, &to_object, &to_entry),
        Command::AutoLayout => handle_auto_layout(engine),
        Command::RerouteAll => handle_reroute_all(engine),

        // Админ-команды не модифицируют ядро, они управляют жизненным циклом API-сервера.
        Command::RestartServer => Ok(Applied { rev: 0, created: None, at: None }),
        Command::ChangePort { .. } => Ok(Applied { rev: 0, created: None, at: None }),
        Command::Shutdown => Ok(Applied { rev: 0, created: None, at: None }),
        Command::ReloadConfig => Ok(Applied { rev: 0, created: None, at: None }),
    }
}
