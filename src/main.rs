use poise::serenity_prelude::{
    self as serenity, ChannelType, ChannelId, CreateEmbed, CreateMessage, EditMessage,
    GetMessages, GuildId, MessageId, UserId,
};
use std::collections::HashMap;
use std::env::var;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

type Error = Box<dyn std::error::Error + Send + Sync>;

/// How long we keep watching an original message for edits/deletes after
/// posting our reply.
const TRACK_DURATION: Duration = Duration::from_secs(60 * 60 * 24);

/// What we remember about an original message: the id of the reply we posted,
/// and the original's content the last time we acted on it. The content lets us
/// ignore the redundant `MessageUpdate` events Discord emits for our own
/// embed-suppression edits and for late embed unfurls — without it those echoes
/// would re-edit the reply in a feedback loop.
#[derive(Clone)]
struct TrackedReply {
    reply: MessageId,
    content: String,
}

type Tracked = Arc<Mutex<HashMap<MessageId, TrackedReply>>>;

pub struct Data {
    /// Maps an original (author) message id to the reply we posted for it, so we
    /// can update or delete that reply if the author later edits or removes
    /// their message. Entries are dropped after `TRACK_DURATION`.
    tracked: Tracked,
    /// Guards the one-time startup crawl against repeated `Ready` events.
    loaded: AtomicBool,
}

/// Current unix time in seconds.
fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Record an original→reply pairing and schedule its removal once the
/// original is older than `TRACK_DURATION`. `created_unix` is the original
/// message's creation time.
fn track(
    tracked: &Tracked,
    original: MessageId,
    reply: MessageId,
    content: String,
    created_unix: i64,
) {
    tracked
        .lock()
        .unwrap()
        .insert(original, TrackedReply { reply, content });

    let remaining = created_unix + TRACK_DURATION.as_secs() as i64 - now_unix();
    if remaining <= 0 {
        return;
    }
    let tracked = tracked.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(remaining as u64)).await;
        tracked.lock().unwrap().remove(&original);
    });
}

/// Extract the matching links from `content`, strip their tracking query
/// string, rewrite them to an embed-friendly host, and render each as small
/// subtext (`-# `).
fn rewrite_links(content: &str) -> Vec<String> {
    content
        .split_whitespace()
        .filter(|word| {
            word.starts_with("https://twitter.com/")
                || word.starts_with("https://x.com/")
                || word.starts_with("https://www.instagram.com")
                || word.starts_with("https://instagram.com")
        })
        .map(|word| {
            let url = word.split('?').next().unwrap_or(word);
            let url = url
                .replace("https://twitter.com/", "https://twittpr.com/")
                .replace("https://x.com/", "https://twittpr.com/")
                .replace("instagram.com", "vxinstagram.com");
            format!("-# {url}")
        })
        .collect()
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let token = var("DISCORD_TOKEN")
        .expect("Missing `DISCORD_TOKEN` env var, see README for more information.");
    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    let framework = poise::Framework::builder()
        .setup(move |_ctx, _ready, _framework| {
            Box::pin(async move {
                Ok(Data {
                    tracked: Arc::new(Mutex::new(HashMap::new())),
                    loaded: AtomicBool::new(false),
                })
            })
        })
        .options(poise::FrameworkOptions {
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler(ctx, event, framework, data))
            },
            ..Default::default()
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;

    client.unwrap().start().await.unwrap();
}

/// Scan recent history in every visible text channel and rebuild the tracking
/// table. Our replies carry no reply-reference, so each original is paired with
/// the earliest later bot message whose content matches `rewrite_links(...)`.
async fn load_tracked(
    ctx: &serenity::Context,
    tracked: &Tracked,
    bot_id: UserId,
    guild_ids: Vec<GuildId>,
) {
    let cutoff = now_unix() - TRACK_DURATION.as_secs() as i64;
    let mut restored = 0u32;

    for guild_id in guild_ids {
        let channels = match guild_id.channels(&ctx.http).await {
            Ok(channels) => channels,
            Err(_) => continue,
        };

        for channel in channels.values() {
            if channel.kind != ChannelType::Text {
                continue;
            }

            // Page back through history until we cross the cutoff.
            let mut messages: Vec<serenity::Message> = Vec::new();
            let mut before: Option<MessageId> = None;
            loop {
                let mut builder = GetMessages::new().limit(100);
                if let Some(before) = before {
                    builder = builder.before(before);
                }
                let batch = match channel.id.messages(&ctx.http, builder).await {
                    Ok(batch) => batch,
                    Err(_) => break,
                };
                let Some(oldest) = batch.last() else {
                    break;
                };
                let reached_cutoff = oldest.timestamp.unix_timestamp() < cutoff;
                before = Some(oldest.id);
                messages.extend(
                    batch
                        .into_iter()
                        .filter(|m| m.timestamp.unix_timestamp() >= cutoff),
                );
                if reached_cutoff {
                    break;
                }
            }

            // Pair originals with their replies in chronological order.
            messages.sort_by_key(|m| m.id);
            let mut claimed = vec![false; messages.len()];
            for i in 0..messages.len() {
                if messages[i].author.id == bot_id {
                    continue;
                }
                let links = rewrite_links(&messages[i].content);
                if links.is_empty() {
                    continue;
                }
                let expected = links.join("\n");
                for j in (i + 1)..messages.len() {
                    if claimed[j] || messages[j].author.id != bot_id {
                        continue;
                    }
                    if messages[j].content == expected {
                        claimed[j] = true;
                        track(
                            tracked,
                            messages[i].id,
                            messages[j].id,
                            messages[i].content.clone(),
                            messages[i].timestamp.unix_timestamp(),
                        );
                        restored += 1;
                        break;
                    }
                }
            }
        }
    }

    println!("Restored {restored} tracked message(s) from history");
}

async fn event_handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Ready { data_about_bot, .. } => {
            println!("Logged in as {}", data_about_bot.user.name);

            // Rebuild the tracking table from the last `TRACK_DURATION` of
            // history so edits/deletes still work after a restart. Run once.
            if !data.loaded.swap(true, Ordering::SeqCst) {
                let ctx = ctx.clone();
                let tracked = data.tracked.clone();
                let bot_id = data_about_bot.user.id;
                let guild_ids: Vec<GuildId> =
                    data_about_bot.guilds.iter().map(|g| g.id).collect();
                tokio::spawn(async move {
                    load_tracked(&ctx, &tracked, bot_id, guild_ids).await;
                });
            }
        }
        serenity::FullEvent::Message { new_message } => {
            let links = rewrite_links(&new_message.content);
            if !links.is_empty() {
                let msg = links.join("\n");

                // Keep the user's original message, but hide the embed Discord
                // generates for the original (broken) links.
                let suppress = new_message
                    .channel_id
                    .edit_message(
                        &ctx.http,
                        new_message.id,
                        EditMessage::new().suppress_embeds(true),
                    )
                    .await;
                if let Err(why) = suppress {
                    println!("Error suppressing embeds: {why:?}");
                }

                // Post the rewritten links so Discord unfurls the fixed embed.
                let builder = CreateMessage::new().content(msg);
                match new_message.channel_id.send_message(&ctx.http, builder).await {
                    Ok(reply) => {
                        // Track the original so we can mirror later edits/deletes,
                        // then forget it after TRACK_DURATION.
                        track(
                            &data.tracked,
                            new_message.id,
                            reply.id,
                            new_message.content.clone(),
                            new_message.timestamp.unix_timestamp(),
                        );
                    }
                    Err(why) => println!("Error posting replacement embed: {why:?}"),
                }
            }

            if new_message.author.id == 1035968772412014592 {
                let create_embeds: Vec<CreateEmbed> = new_message
                    .embeds
                    .iter()
                    .map(|embed| embed.clone().into())
                    .collect();
                let builder = CreateMessage::new()
                    .content(new_message.content.clone())
                    .embeds(create_embeds);
                let channel_id = ChannelId::new(1078237568631578654);
                let transfer_msg = channel_id.send_message(&ctx.http, builder).await;
                if let Err(why) = transfer_msg {
                    println!("Error sending transfer message: {why:?}");
                }
            }
        }
        serenity::FullEvent::MessageUpdate { event, .. } => {
            // Only react to genuine content edits of a message we're tracking.
            let Some(content) = &event.content else {
                return Ok(());
            };
            // Look up the reply and bail unless the content actually changed.
            // Discord re-sends `MessageUpdate` (with content) for our own
            // suppress edits and for late embed unfurls; acting on those echoes
            // would re-edit the reply in a loop. Record the new content so the
            // follow-up echoes are recognised as no-ops.
            let reply_id = {
                let mut tracked = data.tracked.lock().unwrap();
                let Some(entry) = tracked.get_mut(&event.id) else {
                    return Ok(());
                };
                if &entry.content == content {
                    return Ok(());
                }
                entry.content = content.clone();
                entry.reply
            };

            let links = rewrite_links(content);
            if links.is_empty() {
                // The author removed or changed the link to something we don't
                // handle: delete our reply and restore the original's embeds.
                if let Err(why) = event.channel_id.delete_message(&ctx.http, reply_id).await {
                    println!("Error deleting stale reply: {why:?}");
                }
                let _ = event
                    .channel_id
                    .edit_message(
                        &ctx.http,
                        event.id,
                        EditMessage::new().suppress_embeds(false),
                    )
                    .await;
                data.tracked.lock().unwrap().remove(&event.id);
            } else {
                // The link changed: re-suppress the new embed on the original
                // and refresh our reply.
                let _ = event
                    .channel_id
                    .edit_message(
                        &ctx.http,
                        event.id,
                        EditMessage::new().suppress_embeds(true),
                    )
                    .await;
                if let Err(why) = event
                    .channel_id
                    .edit_message(
                        &ctx.http,
                        reply_id,
                        EditMessage::new().content(links.join("\n")),
                    )
                    .await
                {
                    println!("Error updating reply: {why:?}");
                }
            }
        }
        serenity::FullEvent::MessageDelete {
            channel_id,
            deleted_message_id,
            ..
        } => {
            let entry = data.tracked.lock().unwrap().remove(deleted_message_id);
            if let Some(entry) = entry {
                if let Err(why) = channel_id.delete_message(&ctx.http, entry.reply).await {
                    println!("Error deleting reply for removed message: {why:?}");
                }
            }
        }
        _ => {}
    }
    Ok(())
}
