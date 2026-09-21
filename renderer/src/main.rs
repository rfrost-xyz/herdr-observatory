//! Bounded frame adapter using ttfx as a library, without a terminal or shell.
use clap::Parser;
use std::io::{Read, Write};
use ttfx::engine::ctx::{Clock, EngineCtx};
use ttfx::utils::rng::Rng;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let effect = std::env::args().nth(1).unwrap_or_else(|| "decrypt".into());
    if !["decrypt", "vhstape", "crumble"].contains(&effect.as_str()) {
        return Err("invalid effect".into());
    }
    let mut text = String::new();
    std::io::stdin().take(2049).read_to_string(&mut text)?;
    if text.len() > 2048 {
        return Err("input too large".into());
    }
    let mut args = vec![
        "ttfx",
        "--no-color",
        "--ignore-terminal-dimensions",
        "--canvas-width",
        "100",
        "--canvas-height",
        "6",
        "--anchor-text",
        "nw",
        "--seed",
        "42",
        &effect,
    ];
    if effect == "vhstape" {
        args.extend(["--total-glitch-time", "90"]);
    }
    if effect == "decrypt" {
        args.extend(["--typing-speed", "12"]);
    }
    let cli = ttfx::cli::Cli::try_parse_from(args)?;
    let mut ctx = EngineCtx::new(
        &text,
        cli.terminal_config(),
        Rng::seeded(42),
        Clock::virtual_with_frame_rate(30),
    )?;
    let mut renderer = cli.effect.unwrap().build_effect();
    renderer.build(&mut ctx)?;
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    // Bound CPU and wire size; downsample longer effects and finish with original text in the UI.
    for i in 0..360 {
        let Some(frame) = renderer.next_frame(&mut ctx) else {
            break;
        };
        if i % 3 == 0 {
            writeln!(out, "{}", frame.len())?;
            out.write_all(frame.as_bytes())?;
            out.write_all(b"\n")?;
        }
    }
    Ok(())
}
