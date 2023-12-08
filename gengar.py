import discord
import config
import re

from discord.ext import commands


intents = discord.Intents.all()
bot = commands.AutoShardedBot(command_prefix=commands.when_mentioned, intents=intents, help_command=None)


@bot.event
async def on_message(message: discord.Message):
    if (message.author.id != 825166835712393256) and ("https://twitter.com/" in message.content or "https://x.com/" in message.content or "www.tiktok.com" in message.content or "vm.tiktok.com" in message.content):
        tweets = list(filter(lambda word: "https://twitter.com/" in word or "https://x.com/" in word or "tiktok.com" in word, re.split(" |\n", message.content)))
        fixed_tweets = []
        for msg in tweets:
            if "twitter" in msg:
                fixed_tweets.append(f"[O]({msg.replace('twitter', 'fxtwitter')})")
            elif "www.tiktok.com" in msg:
                fixed_tweets.append(f"[O]({msg.replace('www.tiktok.com', 'www.vxtiktok.com')})")
            elif "vm.tiktok.com" in msg:
                fixed_tweets.append(f"[O]({msg.replace('vm.tiktok.com', 'vm.vxtiktok.com')})")
            else:
                fixed_tweets.append(f"[O]({msg.replace('x.com', 'fixupx.com')})")
        await message.channel.send(content=' '.join(fixed_tweets))
        await message.edit(suppress=True)
    if (message.author.id == 1035968772412014592):
        await bot.get_channel(1078237568631578654).send(content=message.content, embeds=message.embeds)

bot.run(config.TOKEN)
