# Scripty: Speech to Text for Discord!

Welcome to Scripty's GitHub repo! This repo contains the bot's entire source code.

## What is Scripty?

In a nutshell, Scripty is a speech to text bot for Discord voice chats.

## Invite the Bot

The bot is pretty much feature-complete and should be working... hopefully.

https://scripty.imaskeleton.me/invite

## Features/TODO

| Feature | Done |
| --- | --- |
| Audio Receive | yes |
| Audio Processing | yes |
| Speech To Text | yes 🎉 |
| Send to Chat | yes |
| Database | yes |

## Self-hosting

No support for self-hosting will be given.

Building the bot requires Nightly Rust, with [`libdeepspeech.so`](https://github.com/mozilla/DeepSpeech) in your `LD_LIBRARY_PATH` 
and `LIBRARY_PATH` environment variables.

You will also need to clone https://github.com/tazz4843/deepspeech-rs and
point `scripty_audio_utils/Cargo.toml` to the directory where you cloned it
(this fork adds forced implementations for Send + Sync on all types except one).
```bash
LIBRARY_PATH="/path/to/libdeepspeech/" RUSTFLAGS="-Ctarget-cpu=native" cargo build --release
```

### It doesn't work on Windows!
Yeah I know. Windows support is not planned, nor will any PRs for it be accepted.
If you make one, it will be closed and **not** merged.

Most contributors disappear after a short time, leaving me to maintain everything.
I can't do that for multiple OSes, especially considering I have no Windows devices
to test on.


## More Info

If you'd like to know more about the bot, feel free to join its Discord server!

[![discord invite](https://img.shields.io/discord/675390855716274216?logo=discord&style=for-the-badge)](https://discord.gg/xSpNJSjNhq)
