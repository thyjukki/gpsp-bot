use json::JsonValue;

use std::time::Duration;
use std::thread;
use log::{debug, error, info};
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::sync::Semaphore;
use tokio::time;
use tokio::task;

use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;

mod telegram_client;
use telegram_client::*;

mod util;
mod bot_commands;
use bot_commands::*;
use util::*;


struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {

        let text = (msg.clone()).content;

        if has_command_prefix_or_postfix(text.as_str()) {
            let stripped = remove_command_prefix_and_postfix(text.as_str());
            let command = parse_command(&stripped);

            match command {
                (BotCommand::Ping, _) => {
                    msg.channel_id.say(&ctx.http, &"pong").await.expect("pong failed");
                },
                (BotCommand::Roll, args) => {
                    let act = args[0].to_string();
                    let noppa1 = noppa();
                    let noppa2 = noppa();
                    let got_dubz = noppa1 == noppa2;
                    let maybe_worded_handle = if !got_dubz {
                        Some(task::spawn(better_wording(act.to_string())))
                    } else {
                        None
                    };
                    let mut msg_content = format!("Noppa 1: {}", noppa1);
                    let mut noppa1_msg = msg.reply(&ctx.http, &msg_content).await.expect("noppa failed");
                    msg.channel_id.broadcast_typing(&ctx.http).await.expect("typing failed");

                    time::sleep(Duration::from_secs(noppa() as u64)).await;
                    msg_content = format!("Noppa 1: {}\nNoppa 2: {}", noppa1, noppa2);

                    noppa1_msg.edit(&ctx, |m| m.content(msg_content)).await.expect("noppa1_edit failed");
                    //msg.channel_id.broadcast_typing(&ctx.http).await.expect("typing failed");

                    time::sleep(Duration::from_secs(2)).await;
                    let dubz_worded = if !got_dubz { maybe_worded_handle.unwrap().await.unwrap() } else { None };
                    msg_content = format!("Noppa 1: {}\nNoppa 2: {}\n{}", noppa1, noppa2,
                                          if got_dubz {
                                              format!("Tuplat tuli, {}. 😎", act.trim())
                                          } else {
                                              format!("Ei tuplia, {}. 😿", dubz_worded.unwrap())
                                          });
                    noppa1_msg.edit(&ctx, |m| m.content(msg_content)).await.expect("noppa1_edit failed");

                },
                (BotCommand::Download, args) => {
                    let (done_sender, done_receiver): (oneshot::Sender<()>, oneshot::Receiver<()>) = oneshot::channel();
                    let send_chat_action_handle = task::spawn(chat_action_discord_loop(done_receiver, msg.clone(), ctx.clone()));
                    let discord_max_vid_size_in_m = 8;
                    let download_video_handle = task::spawn(download_video(args[0].to_string(), discord_max_vid_size_in_m));
                    debug!("Downloading video from URL '{}'", stripped);
                    let download_video_handle_consumed = download_video_handle.await;
                    if download_video_handle_consumed.is_ok() {
                        let video_location = download_video_handle_consumed.unwrap().unwrap();
                        let t = truncate_video(&video_location, &discord_max_vid_size_in_m.clone()).unwrap();
                        debug!("cutted video: {}", t);
                        msg.channel_id.send_files(&ctx.http, vec![t.as_str()], |m| m.content("")).await.expect("Sending file to discord failed");
                        msg.channel_id.delete_message(&ctx.http, msg.id).await.expect("Deleting message failed");
                        let _ = done_sender.send(());
                        send_chat_action_handle.await.expect("Send chat action panicked");
                    }
                },
                (BotCommand::Search, args) => {
                    let (done_sender, done_receiver): (oneshot::Sender<()>, oneshot::Receiver<()>) = oneshot::channel();
                    let send_chat_action_handle = task::spawn(chat_action_discord_loop(done_receiver, msg.clone(), ctx.clone()));
                    let discord_max_vid_size_in_m = 8;
                    let download_video_handle = task::spawn(download_video(format!("ytsearch:\"{}\"" , args[0]), discord_max_vid_size_in_m));
                    debug!("Downloading video from URL '{}'", stripped);
                    let download_video_handle_consumed = download_video_handle.await;
                    if download_video_handle_consumed.is_ok() {
                        let video_location = download_video_handle_consumed.unwrap().unwrap();
                        let t = truncate_video(&video_location, &discord_max_vid_size_in_m.clone()).unwrap();
                        debug!("cutted video: {}", t);
                        msg.channel_id.send_files(&ctx.http, vec![t.as_str()], |m| m.content("")).await.expect("Sending file to discord failed");
                        msg.channel_id.delete_message(&ctx.http, msg.id).await.expect("Deleting message failed");
                        let _ = done_sender.send(());
                        send_chat_action_handle.await.expect("Send chat action panicked");
                    }
                },
                (BotCommand::Noop, _) => {}
            };
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}



async fn chat_action_telegram_loop(mut rx_done: oneshot::Receiver<()>, token: String, chat_id: i64) {
    loop {
        match rx_done.try_recv() {
            Err(oneshot::error::TryRecvError::Empty) => {
                send_chat_action(
                    &token,
                    &SendChatAction {
                        chat_id: &chat_id,
                        action: "upload_video",
                    },
                ).await;
                time::sleep(Duration::from_secs(4)).await;
            }
            _ => {
                debug!("Ending action loop");
                return
            }
        }

    }
}

async fn chat_action_discord_loop(mut rx_done: oneshot::Receiver<()>, msg: Message, ctx: Context) {
    loop {
        match rx_done.try_recv() {
            Err(oneshot::error::TryRecvError::Empty) => {
                msg.channel_id.broadcast_typing(&ctx.http).await.expect("typing failed");
                time::sleep(Duration::from_secs(4)).await;
            }
            _ => {
                debug!("Ending action loop");
                return
            }
        }

    }
}

async fn complain_telegram(token: &str, chat_id: &i64, message_id: Option<i64>) {
    send_message(
        &token,
        &SendMessage {
            chat_id: &chat_id,
            reply_to_message_id: message_id,
            text: "Hyvä linkki...",
        },
    )
    .await;
}

async fn send_video_and_delete_message(
    token: &str,
    chat_id: &i64,
    message_id: &i64,
    video_location: &str,
    reply_to_message_id: Option<i64>,
) {
    let dimensions = get_video_dimensions(video_location).unwrap_or((0, 0));
    let video = SendVideo {
        chat_id,
        reply_to_message_id,
        video_location,
        width: dimensions.0,
        height: dimensions.1,
    };
    let _r = send_video(token, &video).await;

    let delete = DeleteMessage {
        chat_id,
        message_id: &message_id,
    };
    delete_message(token, &delete).await;
}

async fn roll_telegram_die(
    token: String,
    chat_id: i64
) -> bool {
    let noppa1 = send_dice(&token,
        &SendDice {
            chat_id: &chat_id,
            disable_notification: &true
        }
    ).await.expect("Sending dice 1 failed");

    let seconds = Duration::from_secs(2);  
    thread::sleep(seconds.clone());

    let noppa2 = send_dice(&token,
        &SendDice {
            chat_id: &chat_id,
            disable_notification: &true
        }
    ).await.expect("Sending dice 2 failed");
    let noppa1_value = noppa1["result"]["dice"]["value"].as_i64().unwrap_or_default();
    let noppa2_value = noppa2["result"]["dice"]["value"].as_i64().unwrap_or_default();

    debug!("rolled {} and {}", noppa1_value, noppa2_value);

    // tg roll animation takes roughly 4 seconds
    let seconds = Duration::from_secs(5);  
    thread::sleep(seconds.clone());
    
    noppa1_value == noppa2_value
}

async fn handle_telegram_video_download(
    stripped: String,
    token: &str,
    chat_id: &i64,
    message_id: Option<i64>,
    reply_to_message_id: Option<i64>,
    url: &str,
    leftovers: &str,
    _is_private_conversation: bool
) {
    debug!("dl start");

    let (done_sender, done_receiver) = oneshot::channel();

    let send_chat_action_handle = task::spawn(chat_action_telegram_loop(done_receiver, token.clone().to_string(), chat_id.clone()));

    let telegram_max_vid_size_in_m = 50;
    let download_video_handle = task::spawn(download_video(url.to_string(), telegram_max_vid_size_in_m));

    //let leftovers = stripped.replace(&url, "");
    let whitelisted_chats: Vec<i64> = get_config_value(EnvVariable::OpenAiChats)
        .split(";")
        .map(|id| id.parse::<i64>().unwrap_or_default())
        .collect();
    // TODO - this whitelisting is not in use right now
    let _openai_allowed_in_this_chat = whitelisted_chats.contains(chat_id);
    let parse_cut_args_handle = task::spawn(parse_cut_args(leftovers.to_string()));

    debug!("Downloading video from URL '{}'", stripped);

    let sending_video_succeeded = match tokio::join!(download_video_handle, parse_cut_args_handle) {
        (Ok(download_video_handle_consumed), Ok(parse_cut_args_handle_consumed)) => {
            match (download_video_handle_consumed, parse_cut_args_handle_consumed) {
                (None, _) => false,
                (Some(video_location), None) => { 
                    send_video_and_delete_message(token, chat_id, &message_id.unwrap_or_default(), &video_location, reply_to_message_id).await;
                    delete_file(&video_location);
                    true
                },
                (Some(video_location), Some(cut_args)) => {
                    if let Some(cut_video_location) = cut_video(&video_location, &cut_args.0, cut_args.1){
                        send_video_and_delete_message(token, chat_id, &message_id.unwrap_or_default(), &cut_video_location.as_str(), reply_to_message_id).await;
                        delete_file(&video_location);
                        delete_file(&cut_video_location);
                        true
                    } else {
                        false
                    }
                },
            }
        },
        (Ok(download_video_handle_consumed), Err(_)) => {
            match download_video_handle_consumed {
                Some(video_location) => {
                    send_video_and_delete_message(token, chat_id, &message_id.unwrap_or_default(), &video_location, reply_to_message_id).await;
                    delete_file(&video_location);
                    true
                },
                None => false,
            }
        },
        (Err(_), _) => {
            debug!("Downloading video has failed");
            false
        }
    };

    if !sending_video_succeeded {
        complain_telegram(&token, &chat_id, message_id).await;
    }


    let _ = done_sender.send(());
    send_chat_action_handle.await.expect("Send chat action panicked");
}

async fn handle_telegram_update(update: &JsonValue) {
    let token = get_config_value(EnvVariable::TelegramToken);
    if let JsonValue::Object(ref message) = update {
        let maybe_chat_id = message["message"]["chat"]["id"].as_i64();
        if maybe_chat_id.is_none() {
            debug!("Encountered update with no message.chat.id object");
            return;
        }
        let chat_id = maybe_chat_id.unwrap();
        let reply_to_message_id = message["message"]["reply_to_message"]["message_id"].as_i64();
        let message_id = message["message"]["message_id"].as_i64();
        let _is_private_conversation =
            message["message"]["chat"]["type"].as_str() == Some("private");

        if let Some(text) = message["message"]["text"].as_str() {
            if has_command_prefix_or_postfix(text) {
                let stripped = remove_command_prefix_and_postfix(text);
                let command = parse_command(&stripped);
                debug!("command: {}", command.0.to_string());
                debug!("args: {}", command.1.join(", "));

                match command {
                    (BotCommand::Ping, _) => {
                        send_message(
                            &token,
                            &SendMessage {
                                chat_id: &chat_id,
                                text: "pong",
                                reply_to_message_id: None,
                            },
                        )
                        .await;
                    },
                    (BotCommand::Download, args) => {
                        handle_telegram_video_download(
                            stripped.to_string(),
                            &token,
                            &chat_id,
                            message_id,
                            reply_to_message_id,
                            args[0].as_str(),
                            args[1].as_str(),
                            _is_private_conversation
                        )
                        .await;
                    },
                    (BotCommand::Roll, args) => {
                        let roll_handle = task::spawn(roll_telegram_die(token.clone().to_string(), chat_id.clone()));
                        let worded_handle = task::spawn(better_wording(args[0].to_string()));
                        match tokio::join!(roll_handle, worded_handle) {
                            (Ok(roll_handle_consumed), Ok(worded_handle_consumed)) => {
                                match (roll_handle_consumed, worded_handle_consumed) {
                                    (true, _) => {
                                        debug!("{:?}, {:?}", true, args[0]);
                                        send_message(&token,
                                            &SendMessage {
                                                chat_id: &chat_id,
                                                text: &format!("Tuplat tuli, {} 😎", args[0].trim()).to_string(),
                                                reply_to_message_id: None,
                                            }).await;
                                    }
                                    (false, Some(worded_handle_consumed_value)) => {
                                        debug!("{:?}, {:?}", false, worded_handle_consumed_value);
                                        send_message(&token,
                                            &SendMessage {
                                                chat_id: &chat_id,
                                                text: &format!("Ei tuplia, {} 😢", worded_handle_consumed_value).to_string(),
                                                reply_to_message_id: None,
                                            }).await;
                                    },
                                    (false, None) => {
                                        error!("openai wording failed");
                                    },
                                }
                            }
                            (_, _) => {
                                error!("rolling or openai wording failed");
                            },
                        }
                    }
                    (BotCommand::Search, args) => {
                        let (done_sender, done_receiver): (oneshot::Sender<()>, oneshot::Receiver<()>) = oneshot::channel();
                        let send_chat_action_handle = task::spawn(chat_action_telegram_loop(done_receiver, token.clone().to_string(), chat_id.clone()));
                        let telegram_max_vid_size_m = 50;
                        let download_video_handle = task::spawn(download_video(format!("ytsearch:\"{}\"" , args[0]), telegram_max_vid_size_m));
                        debug!("Downloading video from URL '{}'", stripped);
                        let download_video_handle_consumed = download_video_handle.await;
                        if download_video_handle_consumed.is_ok() {
                            let video_location = download_video_handle_consumed.unwrap().unwrap();
                            let t = truncate_video(&video_location, &telegram_max_vid_size_m.clone()).unwrap();
                            debug!("cutted video: {}", t);
                            send_video_and_delete_message(&token, &chat_id, &message_id.unwrap_or_default(), &t, reply_to_message_id).await;
                            let _ = done_sender.send(());
                            send_chat_action_handle.await.expect("Send chat action panicked");
                        }
                    }
                    (BotCommand::Noop, _) => {
                        debug!("no command");
                    }
                };
            }
        } else {
            debug!("no text content");
        }
    }
}

async fn telegram_update_loop(token: &str) -> ! {
    let mut last_update_id = 0;
    let max_threads = 2;
    let semaphore = Arc::new(Semaphore::new(max_threads));
    let failed_request_grace_period = Duration::from_millis(2000);

    loop {
        let t = get_updates(
            &token,
            &GetUpdates {
                timeout: &60,
                offset: &(&last_update_id + 1),
            },
        )
        .await;
        let parsed = t.unwrap();

        let ok = parsed["ok"].as_bool().unwrap_or_default();
        if !ok {
            time::sleep(failed_request_grace_period).await;
            continue;
        }

        let result = match &parsed["result"] {
            JsonValue::Array(arr) => arr,
            _ => panic!("'result' field is not an array"),
        };

        for update in result.clone() {
            if let Some(update_id) = update["update_id"].as_i64() {
                let update_id = update_id;

                let semaphore_permit = semaphore
                    .clone()
                    .acquire_owned()
                    .await
                    .expect("Semaphore acquire error");

                tokio::spawn(async move {
                    handle_telegram_update(&update).await;
                    drop(semaphore_permit);
                });

                last_update_id = update_id;
            } else {
                error!("'update_id' field is missing or not an integer");
                continue;
            }
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    match get_platform() {
        Platform::Telegram => {
            let token = get_config_value(EnvVariable::TelegramToken);
            info!("Telegram bot running!");
            telegram_update_loop(&token).await;
        }
        Platform::Discord => {
            info!("Discord bot running");
            let token = get_config_value(EnvVariable::DiscordToken);
            debug!("DISCORD TOKEN: {}", token);
            // Set gateway intents, which decides what events the bot will be notified about
            let intents = GatewayIntents::GUILD_MESSAGES
                | GatewayIntents::DIRECT_MESSAGES
                | GatewayIntents::GUILD_MESSAGE_TYPING
                | GatewayIntents::DIRECT_MESSAGE_TYPING
                | GatewayIntents::MESSAGE_CONTENT;

            // Create a new instance of the Client, logging in as a bot. This will
            // automatically prepend your bot token with "Bot ", which is a requirement
            // by Discord for bot users.
            let mut client =
                Client::builder(&token, intents).event_handler(Handler).await.expect("Err creating client");

            // Finally, start a single shard, and start listening to events.
            //
            // Shards will automatically attempt to reconnect, and will perform
            // exponential backoff until it reconnects.
            if let Err(why) = client.start().await {
                println!("Client error: {:?}", why);
            }
        }
    }
}
