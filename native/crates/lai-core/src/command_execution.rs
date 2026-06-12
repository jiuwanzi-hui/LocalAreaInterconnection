use crate::virtual_adapter_plan::NetworkCommand;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CommandExecutionStatus {
    Planned,
    SkippedNeedsConfirmation,
    SkippedNeedsElevation,
    Succeeded,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CommandExecutionPreview {
    pub platform: String,
    pub requires_elevation: bool,
    pub confirmed: bool,
    pub can_execute_now: bool,
    pub admin_hint: String,
    pub next_action: String,
    pub commands: Vec<CommandExecutionRecord>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CommandExecutionRecord {
    pub command: String,
    pub purpose: String,
    pub status: CommandExecutionStatus,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub error: Option<String>,
    pub next_action: Option<String>,
}

pub fn create_command_execution_preview(
    commands: &[NetworkCommand],
    requires_elevation: bool,
    confirmed: bool,
    elevated: Option<bool>,
) -> CommandExecutionPreview {
    let can_execute_now = confirmed && (!requires_elevation || elevated == Some(true));
    let status = if !confirmed {
        CommandExecutionStatus::SkippedNeedsConfirmation
    } else if requires_elevation && elevated != Some(true) {
        CommandExecutionStatus::SkippedNeedsElevation
    } else {
        CommandExecutionStatus::Planned
    };
    let next_action = if !confirmed {
        "Review the commands, then rerun with --yes true from an Administrator terminal.".to_owned()
    } else if requires_elevation && elevated != Some(true) {
        "Open Windows Terminal or PowerShell as Administrator and rerun the same command."
            .to_owned()
    } else {
        "Execute commands in order and then run adapter/network diagnostics.".to_owned()
    };

    CommandExecutionPreview {
        platform: "windows".to_owned(),
        requires_elevation,
        confirmed,
        can_execute_now,
        admin_hint: "Virtual adapter configuration uses netsh and requires Administrator privileges on Windows.".to_owned(),
        next_action: next_action.clone(),
        commands: commands
            .iter()
            .map(|command| CommandExecutionRecord {
                command: command.command.clone(),
                purpose: command.purpose.clone(),
                status: status.clone(),
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                error: None,
                next_action: Some(next_action.clone()),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execution_preview_requires_confirmation_before_running() {
        let commands = vec![NetworkCommand {
            tool: "netsh".to_owned(),
            args: vec!["interface".to_owned()],
            command: "netsh interface".to_owned(),
            purpose: "configure adapter".to_owned(),
        }];

        let preview = create_command_execution_preview(&commands, true, false, Some(true));

        assert!(!preview.can_execute_now);
        assert_eq!(
            preview.commands[0].status,
            CommandExecutionStatus::SkippedNeedsConfirmation
        );
        assert!(preview.next_action.contains("--yes true"));
    }

    #[test]
    fn execution_preview_blocks_without_elevation() {
        let commands = vec![NetworkCommand {
            tool: "netsh".to_owned(),
            args: vec!["interface".to_owned()],
            command: "netsh interface".to_owned(),
            purpose: "configure adapter".to_owned(),
        }];

        let preview = create_command_execution_preview(&commands, true, true, Some(false));

        assert!(!preview.can_execute_now);
        assert_eq!(
            preview.commands[0].status,
            CommandExecutionStatus::SkippedNeedsElevation
        );
        assert!(preview.next_action.contains("Administrator"));
    }
}
