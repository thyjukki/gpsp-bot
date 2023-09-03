use crate::util::extract_urls;

/// Enum created to handle incoming bot commands and their arguments
/// * ping
/// * roll
/// * download
/// * search
pub enum BotCommand {
    Ping,
    Roll,
    Download,
    Search,
    Noop
}

/// Map bot command to corresponding string
impl BotCommand {
    pub fn to_string(&self) -> &str {
        match self {
            BotCommand::Ping => "ping",
            BotCommand::Roll => "tuplilla",
            BotCommand::Search => "s ",
            // string versions below only for debugging purposes
            BotCommand::Download => "download",
            BotCommand::Noop => "noop"
        }
    }
}

fn get_arg(input: &str, cmd: &BotCommand) -> String {
    let trimmed = input.trim();
    let cmd_str = cmd.to_string();
    return trimmed[cmd_str.len()..].trim().to_string();
}

/// Parse command from input string and return tuple of command and vector of arguments
pub fn parse_command(input: &str) -> (BotCommand, Vec<String>) {
    let trimmed = input.trim();
    let urls = extract_urls(trimmed);
    return if trimmed.starts_with(BotCommand::Ping.to_string()) {
        (BotCommand::Ping, vec![])
    } else if trimmed.starts_with(BotCommand::Roll.to_string()) {
        (BotCommand::Roll, vec![get_arg(trimmed, &BotCommand::Roll)])
    } else if trimmed.starts_with(BotCommand::Search.to_string()) {
        (BotCommand::Search, vec![get_arg(trimmed, &BotCommand::Search)])
    } else if urls.len() > 0 {
        let url = &urls[0];
        (BotCommand::Download, vec![url.clone(), trimmed.replace(urls[0].as_str(), "").trim().to_string()])
    } else {
        (BotCommand::Noop, vec![])
    };
}