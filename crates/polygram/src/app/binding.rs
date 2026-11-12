use crate::app::PolygramApp;
use serde_json::Value;

impl PolygramApp {
    pub fn post_binding(
        &mut self,
        object_id: String,
        entry_id: String,
        connector: String,
        cell: String,
        interval_secs: u64,
    ) {
        self.bindings.post_binding(
            object_id,
            entry_id,
            connector,
            cell,
            interval_secs,
            &self.connection.api_client,
            &self.connection.api_base,
        );
    }

    pub fn delete_binding(&mut self, object_id: String, entry_id: String) {
        self.bindings
            .delete_binding(object_id, entry_id, &self.connection.api_client, &self.connection.api_base);
    }

    pub fn get_binding_statuses(&self) -> Value {
        self.bindings
            .get_binding_statuses(&self.connection.api_client, &self.connection.api_base)
    }

    pub fn get_bindings(&self) -> Vec<Value> {
        self.bindings
            .get_bindings(&self.connection.api_client, &self.connection.api_base)
    }

    pub fn refresh_api_bindings(&mut self) {
        self.bindings
            .refresh_api_bindings(&self.connection.api_client, &self.connection.api_base);
    }
}
