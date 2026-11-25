# Horizon (performance fork)

A personal fork of [horizon-rust/horizon](https://github.com/horizon-rust/horizon). Strips Horizon's own 
additions (launcher, telemetry, agent control, Discord, friends, cosmetics) to find out how fast Minecraft 
can go. Just a binary now, Windows and Linux AMD64. 

## TODO
- Performance not as good as a block game should be, even on maxperf profile 128 render distance w/ unlimited frame rate tops 
out around 400 FPS. Native GPU calls like nvidium mod (but for radeon) somehow? 
  - Loads insanely fast (32 ms!), but in-game performance does not completely dominate Java as expected (it does use a lot less memory, but this is kind of implicit).
  - Frame rates are super unstable. On 128 render distance, jumps from mid 100s to 400s randomly when standing still.
- Just from a once over on the code, allocations can definitely be reduced.
- Voxy/seedgen generation?
- Sodium optimizations merged into this code?
- Since this was designed to mirror vanilla, it's probably leaving a lot on the table intentionally.

 The human(🤸) genome is written in DNA(🐢), and as we all know 🧠 DNA(🐢) is very memory unsafe 🤢⚠ and unperformant 🐢. Not to mention 🙊 that the human(🤸) genome wasn't written ✏ with any computer 🖥 supervision ⚠ for 1.8 million 😱 years ⚠⚠🤮! No automated safety checks 📋 or anything! Which brings me to the conclusion; we need to rewrite ✏ the human(🤸) genome in Rust(🚀). Rust(🚀) is blazingly 🚀 fast 🚀, and memory ✅ safe 🚀, too 🚀. Rewriting the human(🤸) genome 🚀🚀 in Rust(🚀) will prevent humans 🤸 from encountering 👾 cancer 😱 (undefined behavior ❌) or from sneezing or coughing 🤧😒 (buffering ⏳) 🚀🚀🚀🚀. Not to mention 🚀 🙊🙊🙊 that humans(🤸) will become immortal 🚀 because 💁 there will be no more 😱 death ‼ (segmentation faults). Thank you 😊 for coming 🙇 to my TED talk 🧸. 

 
## Build and run

Same as the upstream you need a regular Minecraft install first. Then:

```sh
cargo run --release -p horizon-prepare -- --complete       # once, and after a Minecraft update
cargo build --profile fast -p horizon-game                # optimized, quick to link
target/fast/horizon-client --resources .local/horizon-client/26.3 --game-dir run
```

Linux needs `libasound2-dev` / `alsa-lib-devel` and `libudev-dev` / `systemd-devel`; `--no-default-features` builds without audio and controllers. 
The dedicated server is the same binary (`--dedicated`).


## License

Horizon is released under the [GNU LGPL, version 3 or later](LICENSE). Code merged upstream before its move from
MIT stays available under the [MIT licence](LICENSES/MIT.txt). Dependencies keep their own licences: see the
[notices](THIRD_PARTY_NOTICES.md). Horizon was founded by Ludovic Marie and built by the Horizon Team and
[its contributors](https://github.com/horizon-rust/horizon/graphs/contributors).

NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT. Minecraft is a trademark
of Mojang Synergies AB; its code and assets belong to Mojang and Microsoft under the
[Minecraft EULA](https://www.minecraft.net/en-us/eula).
