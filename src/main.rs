use poise::serenity_prelude::{
    self as serenity, ChannelId, CreateEmbed, CreateMessage, CreateWebhook, ExecuteWebhook,
};
use std::env::var;
use std::sync::atomic::AtomicU32;

type Error = Box<dyn std::error::Error + Send + Sync>;

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
                let msg = new_message
                    .content
                    .replace("https://twitter.com/", "https://fxtwitter.com/")
                    .replace("https://x.com/", "https://fixupx.com/")
                    .replace("https://www.tiktok.com/", "https://vm.dstn.to/")
                    .replace("https://vm.tiktok.com/", "https://vm.dstn.to/");

                let member = match new_message
                    .guild_id
                    .unwrap()
                    .member(&ctx.http, new_message.author.id)
                    .await
                {
                    Ok(member) => member,
                    Err(why) => {
                        println!("Error getting member: {why:?}");
                        return Ok(());
                    }
                };

                let name = member.display_name();
                let avatar = member.user.avatar_url().unwrap_or_default();

                let webhook = CreateWebhook::new("gengar");
                let webhook = new_message
                    .channel_id
                    .create_webhook(&ctx.http, webhook)
                    .await;

                let webhook = match webhook {
                    Ok(webhook) => webhook,
                    Err(why) => {
                        println!("Error creating webhook: {why:?}");
                        return Ok(());
                    }
                };

                let builder = ExecuteWebhook::new()
                    .content(msg)
                    .username(name)
                    .avatar_url(avatar);

                let delete_msg = new_message.delete(&ctx).await;
                if let Err(why) = delete_msg {
                    println!("Error deleting message: {why:?}");
                }

                let post_webhook = webhook.execute(&ctx.http, false, builder).await;

                if let Err(why) = post_webhook {
                    println!("Error posting webhook: {why:?}");
                }

                let delete_webhook = webhook.delete(&ctx.http).await;
                if let Err(why) = delete_webhook {
                    println!("Error deleting webhook: {why:?}");
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
