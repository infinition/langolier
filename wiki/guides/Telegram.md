# Telegram

An exported assistant can answer in a Telegram chat. The bridge uses long polling, so no
port is opened on your machine and no domain is needed.

## Setting it up

1. Talk to [@BotFather](https://t.me/BotFather) on Telegram and create a bot. It gives
   you a token.
2. Paste the token into the assistant profile, under its Telegram section.
3. Press the test button. It confirms the token reaches a real bot.
4. Export the assistant, then run it. The bridge starts with the server.

## Who may talk to it

The whitelist accepts numeric ids or `@handles`, comma separated. Leave it empty and
anyone who finds the bot can talk to it.

## How conversations map

One Telegram chat maps to one conversation, so the assistant remembers the thread.
`/new` starts another.

Caps and source privacy apply exactly as they do on the web page: a token cap reached in
Telegram stops the same way, and hidden source names stay hidden.

## What to keep in mind

Messages travel through Telegram's servers. Whatever your assistant answers there leaves
your machine, citations included. Keep it to content that may go there.

For a test or a second instance, `LANGOLIER_DISABLE_TELEGRAM=1` keeps the server from
polling the bot, so two copies do not steal each other's updates.
