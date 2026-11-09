pub struct CommandState {
    pub show_command_palette: bool,
    pub command_input: String,
    pub command_log: Vec<crate::app::CommandLogEntry>,
}

impl CommandState {
    pub fn new() -> Self {
        Self {
            show_command_palette: false,
            command_input: String::new(),
            command_log: Vec::new(),
        }
    }
}
