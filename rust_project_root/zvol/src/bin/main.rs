//! zvol CLI — direct PipeWire volume/mute control.
//!
//! Usage:
//!   zvol list                        list default sink + source
//!   zvol get <sink|source>           show volume (L/R) + mute
//!   zvol set <sink|source> <L> <R>   set left/right volume (0-100, 0-1, or 0%-100%)
//!   zvol mute <sink|source> <on|off> mute/unmute
//!
//! Values can be `100`, `50`, `0.5`, or `50%`; values >1.0 are treated as percent.
//! If only one value is given for `set`, both channels are set to it.
//!
//! This binary shells out to nothing. It links directly to libpipewire-0.3.so.

use std::io::{self, Write};

use zvol::{parse_volume, pct, AudioNode, ZvolContext, ZvolError};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // tracing disabled in CLI for now to avoid pulling tracing-subscriber.
    let _ = ();

    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        print_usage(&mut io::stderr());
        return Ok(());
    }

    let cmd = args[0].to_ascii_lowercase();
    let rest = &args[1..];    match cmd.as_str() {
        "list" | "ls" => {
            print_list()?;
        }
        "get" | "show" | "g" => {
            let (kind, _rest) = parse_kind(rest)?;
            print_node(&get_node(&kind)?);
        }
        "set" | "vol" | "v" => {
            let (kind, rest) = parse_kind(rest)?;
            if rest.is_empty() {
                print_usage(&mut io::stderr());
                return Ok(());
            }
            let mut values: Vec<f32> = rest
                .iter()
                .map(|s| parse_volume(s).map_err(|e| format!("bad volume value {s}: {e}")))
                .collect::<Result<Vec<_>, _>>()?;
            if values.is_empty() {
                print_usage(&mut io::stderr());
                return Ok(());
            }
            if values.len() == 1 {
                set_channel(&kind, 0, values[0])?;
                set_channel(&kind, 1, values[0])?;
            } else {
                set_channel(&kind, 0, values.remove(0))?;
                set_channel(&kind, 1, values.remove(0))?;
            }
            print_node(&get_node(&kind)?);
        }
        "mute" => {
            let (kind, rest) = parse_kind(rest)?;
            if rest.is_empty() {
                print_usage(&mut io::stderr());
                return Ok(());
            }
            let target = rest[0].to_ascii_lowercase();
            let mute = match target.as_str() {
                "on" | "1" | "true" | "yes" => true,
                "off" | "0" | "false" | "no" => false,
                _ => {
                    eprintln!("bad mute target: {target}");
                    return Ok(());
                }
            };
            set_mute(&kind, mute)?;
            print_node(&get_node(&kind)?);
        }
        _ => {
            print_usage(&mut io::stderr());
        }
    }
    Ok(())
}

fn parse_kind(rest: &[String]) -> Result<(String, &[String]), Box<dyn std::error::Error>> {
    if rest.is_empty() {
        print_usage(&mut io::stderr());
        return Ok(("sink".into(), &[] as &[String]));
    }
    let k = rest[0].to_ascii_lowercase();
    let rest = &rest[1..];
    let kind: String = match k.as_str() {
        "sink" | "s" => "sink".into(),
        "source" | "mic" | "m" => "source".into(),
        _ => {
            print_usage(&mut io::stderr());
            return Ok(("sink".into(), &[] as &[String]));
        }
    };
    Ok((kind, rest))
}

fn get_node(kind: &str) -> Result<AudioNode, ZvolError> {
    let ctx = ZvolContext::new().map_err(|e| {
        eprintln!("failed to connect to pipewire: {e}");
        std::process::exit(1);
    })?;
    match kind {
        "sink" | "s" => ctx.sink(),
        "source" | "mic" | "m" => ctx.source(),
        _ => unreachable!(),
    }
}

fn set_channel(kind: &str, idx: usize, vol: f32) -> Result<(), ZvolError> {
    let ctx = ZvolContext::new().map_err(|e| {
        eprintln!("failed to connect to pipewire: {e}");
        std::process::exit(1);
    })?;
    match kind {
        "sink" | "s" => ctx.set_sink_channel(idx, vol),
        "source" | "mic" | "m" => ctx.set_source_channel(idx, vol),
        _ => unreachable!(),
    }
}

fn set_mute(kind: &str, mute: bool) -> Result<(), ZvolError> {
    let ctx = ZvolContext::new().map_err(|e| {
        eprintln!("failed to connect to pipewire: {e}");
        std::process::exit(1);
    })?;
    match kind {
        "sink" | "s" => ctx.set_sink_mute(mute),
        "source" | "mic" | "m" => ctx.set_source_mute(mute),
        _ => unreachable!(),
    }
}

fn print_list() -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx = match ZvolContext::new() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("failed to connect to pipewire: {e}");
            std::process::exit(1);
        }
    };
    let sink = match ctx.sink() {
        Ok(n) => n,
        Err(e) => {
            eprintln!("failed to get default sink: {e}");
            std::process::exit(1);
        }
    };
    let source = match ctx.source() {
        Ok(n) => n,
        Err(e) => {
            eprintln!("failed to get default source: {e}");
            std::process::exit(1);
        }
    };
    println!(
        "sink:   {}  L={}% R={}%  mute={}",
        sink.name,
        pct(sink.volume.first().copied().unwrap_or(0.0)),
        pct(sink.volume.get(1).copied().unwrap_or(0.0)),
        sink.mute
    );
    println!(
        "source: {}  L={}% R={}%  mute={}",
        source.name,
        pct(source.volume.first().copied().unwrap_or(0.0)),
        pct(source.volume.get(1).copied().unwrap_or(0.0)),
        source.mute
    );
    ctx.shutdown();
    Ok(())
}

fn print_node(node: &AudioNode) {
    let left = node.volume.first().copied().unwrap_or(0.0);
    let right = node.volume.get(1).copied().unwrap_or(0.0);
    println!(
        "{}: {}  L={}% R={}%  mute={}",
        node.media_class,
        node.name,
        pct(left),
        pct(right),
        node.mute
    );
}

fn print_usage(err: &mut impl Write) {
    let _ = writeln!(
        err,
        "usage: zvol <command> [args...]

commands:
  list | ls                              show default sink + source
  get  <sink|source>                     show volume (L/R) + mute
  get  sink          (or 'get s')        show default sink
  get  source        (or 'get mic')      show default source (mic)
  set  <sink|source> <L> <R>             set left/right volume
  set  sink 50 100                       set sink L=50%, R=100%
  set  source 70                         set both source channels to 70%
  set  sink 0.35                         set sink channels to 35% (0-1 range)
  set  sink 35% 60%                      percent form
  mute <sink|source> <on|off>            mute/unmute
  mute sink on                            mute the sink
  mute source off                         unmute the mic

volume values: 0-100, 0-1, or 0%-100%. Values >1.0 are treated as percent."
    );
}
