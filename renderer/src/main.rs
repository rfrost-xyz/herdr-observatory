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
    std::io::stdin().take(8193).read_to_string(&mut text)?;
    if text.len() > 8192 {
        return Err("input too large".into());
    }
    let mut args = vec![
        "ttfx",
        "--ignore-terminal-dimensions",
        "--canvas-width",
        "120",
        "--canvas-height",
        "36",
        "--anchor-text",
        "nw",
        "--seed",
        "42",
        &effect,
    ];
    let colours: Vec<String> = std::env::args().skip(2).collect();
    let defaults = vec![
        "7aa2f7".into(),
        "c0caf5".into(),
        "9ece6a".into(),
        "e0af68".into(),
    ];
    let colours = if colours.len() == 4 {
        colours
    } else {
        defaults
    };
    if colours
        .iter()
        .any(|c| c.len() != 6 || !c.chars().all(|x| x.is_ascii_hexdigit()))
    {
        return Err("invalid palette".into());
    }
    if effect == "vhstape" {
        args.extend([
            "--total-glitch-time",
            "90",
            "--glitch-line-colors",
            &colours[0],
            &colours[1],
            "--glitch-wave-colors",
            &colours[2],
            &colours[0],
            "--noise-colors",
            &colours[0],
            &colours[1],
        ]);
    }
    if effect == "decrypt" {
        args.extend([
            "--typing-speed",
            "30",
            "--ciphertext-colors",
            &colours[0],
            &colours[2],
            &colours[3],
        ]);
    }
    args.extend(["--final-gradient-stops", &colours[0], &colours[1]]);
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
    // Only complete animations are accepted by the caller. Budget exhaustion fails closed.
    let mut bytes = 0;
    for _ in 0..5000 {
        let Some(frame) = renderer.next_frame(&mut ctx) else {
            out.write_all(b"END\n")?;
            return Ok(());
        };
        bytes += frame.len();
        if bytes > 128 * 1024 * 1024 {
            return Err("frame budget exceeded".into());
        }
        writeln!(out, "{}", frame.len())?;
        out.write_all(frame.as_bytes())?;
        out.write_all(b"\n")?;
    }
    Err("animation did not finish within budget".into())
}
