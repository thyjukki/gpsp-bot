use crate::util::extract_urls;
use enum_iterator::Sequence;

/// Enum created to handle incoming bot commands and their arguments
/// * ping
/// * roll
/// * download
/// * search
#[derive(Debug, PartialEq, Sequence, Clone)]
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
            BotCommand::Search => "s",
            // string versions below only for debugging purposes
            BotCommand::Download => "dl",
            BotCommand::Noop => "noop"
        }
    }
    pub fn to_description(&self) -> &str {
        match self {
            BotCommand::Roll => "Dubz",
            BotCommand::Search => "Search for a video from youtube",
            BotCommand::Download => "Download a video from a link",
            _ => ""
        }
    }
}

fn get_arg(input: &str, cmd: &BotCommand) -> String {
    // telegram command might be of form /command@botname <args>
    // so we need to trim the whole first word that might start with command
    let words_vec: Vec<&str> = input.trim().split_whitespace().collect();
    return if words_vec[0].starts_with(cmd.to_string()) {
        words_vec[1..].join(" ")
    } else {
        words_vec.join(" ")
    };
}

/// Parse command from input string and return tuple of command and vector of arguments
pub fn parse_command(input: &str) -> (BotCommand, Vec<String>) {
    let trimmed = input.trim();
    let urls = extract_urls(trimmed);
    return if trimmed.starts_with(BotCommand::Ping.to_string()) {
        (BotCommand::Ping, vec![])
    } else if trimmed.starts_with(BotCommand::Roll.to_string()) && get_arg(trimmed, &BotCommand::Roll).len() > 0 {
        (BotCommand::Roll, vec![get_arg(trimmed, &BotCommand::Roll)])
    } else if trimmed.starts_with(BotCommand::Search.to_string()) && get_arg(trimmed, &BotCommand::Search).len() > 0 {
        (BotCommand::Search, vec![get_arg(trimmed, &BotCommand::Search)])
    } else if urls.len() > 0 {
        let url = &urls[0];
        (BotCommand::Download, vec![url.clone(), trimmed.replace(urls[0].as_str(), "").trim().to_string()])
    } else {
        (BotCommand::Noop, vec![])
    };
}