use poise::serenity_prelude::{
    self as serenity, ChannelId, CreateEmbed, CreateMessage, EditMessage,
};
use std::env::var;
use std::sync::atomic::AtomicU32;

// Types used by all command functions
type Error = Box<dyn std::error::Error + Send + Sync>;

// Custom user data passed to all command functions
pub struct Data {
    _poise_mentions: AtomicU32,
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
                    _poise_mentions: AtomicU32::new(0),
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

async fn event_handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    _data: &Data,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Ready { data_about_bot, .. } => {
            println!("Logged in as {}", data_about_bot.user.name);
        }
        serenity::FullEvent::Message { new_message } => {
            if new_message.content.contains("https://twitter.com/")
                || new_message.content.contains("https://x.com/")
                || new_message.content.contains("https://www.tiktok.com/")
                || new_message.content.contains("https://vm.tiktok.com/")
            {
                let mut edited_msg = new_message.clone();
                match edited_msg
                    .edit(&ctx, EditMessage::new().suppress_embeds(true))
                    .await
                {
                    Ok(_) => (),
                    Err(e) => println!("{}", e),
                }

                let message_items: Vec<&str> = new_message.content.split(" ").collect();
                let mut links = "".to_owned();
                for item in message_items {
                    match item {
                        i if i.starts_with("https://twitter.com/") => {
                            links.push_str(
                                format!("[O]({})", i.replace("twitter", "fxtwitter")).as_str(),
                            );
                        }
                        i if i.starts_with("https://x.com/") => {
                            links.push_str(
                                format!("[O]({})", i.replace("x.com", "fixupx.com")).as_str(),
                            );
                        }
                        i if i.starts_with("https://www.tiktok.com/") => {
                            links.push_str(
                                format!("[O]({})", i.replace("www.tiktok.com", "www.vxtiktok.com"))
                                    .as_str(),
                            );
                        }
                        i if i.starts_with("https://www.tiktok.com/") => {
                            links.push_str(
                                format!("[O]({})", i.replace("vm.tiktok.com", "vm.vxtiktok.com"))
                                    .as_str(),
                            );
                        }
                        _ => {}
                    }
                }
                let builder = CreateMessage::new().content(links);
                let link_msg = new_message
                    .channel_id
                    .send_message(&ctx.http, builder)
                    .await;

                if let Err(why) = link_msg {
                    println!("Error sending message: {why:?}");
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
        _ => {}
    }
    Ok(())
}
