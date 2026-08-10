//! Effect trait and run loop (base_effect.py equivalents).

use std::io::Write;

use crate::engine::ctx::{EffectHooks, EngineCtx};
use crate::engine::error::EngineError;

/// One effect: build() once (upstream iterator __init__/build), then
/// next_frame() until None (upstream __next__/StopIteration). Every effect
/// also implements EffectHooks for its registered callbacks.
pub trait Effect: EffectHooks {
    fn build(&mut self, ctx: &mut EngineCtx) -> Result<(), EngineError>;
    fn next_frame(&mut self, ctx: &mut EngineCtx) -> Option<String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunOutcome {
    Complete,
    Interrupted,
    TerminalResized,
}

/// __main__ run loop with terminal_output(): prep canvas, stream frames,
/// always restore the cursor (even on error — RAII would not run on a raw
/// process exit, so this is explicit).
///
/// With `stop_on_resize`, a settled terminal resize also ends the pass, wiped
/// and parked at the top of the area so the caller can rebuild in place.
pub fn run_effect(
    effect: &mut dyn Effect,
    ctx: &mut EngineCtx,
    stop_on_resize: bool,
) -> Result<RunOutcome, EngineError> {
    effect.build(ctx)?;
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    ctx.terminal.prep_canvas(&mut out).map_err(io_err)?;
    let mut outcome = RunOutcome::Complete;
    let result = (|| {
        loop {
            if let Some(stop) = requested_stop(ctx, stop_on_resize) {
                outcome = stop;
                break;
            }
            let Some(frame) = effect.next_frame(ctx) else {
                break;
            };
            if let Some(stop) = requested_stop(ctx, stop_on_resize) {
                outcome = stop;
                ctx.terminal.recycle_output_string(frame);
                break;
            }
            ctx.terminal.print_frame(&mut out, &frame).map_err(io_err)?;
            ctx.terminal.recycle_output_string(frame);
        }
        Ok(())
    })();
    if outcome == RunOutcome::TerminalResized {
        // Leave the cursor hidden and parked at the top of the wiped area: the
        // rebuild redraws in place, and showing the cursor here would strobe it
        // dozens of times a second through a window drag.
        ctx.terminal.reset_canvas_area(&mut out).map_err(io_err)?;
    } else {
        ctx.terminal.restore_cursor(&mut out, "\n").map_err(io_err)?;
    }
    out.flush().ok();
    result.map(|_| outcome)
}

fn requested_stop(ctx: &mut EngineCtx, stop_on_resize: bool) -> Option<RunOutcome> {
    if crate::interrupted() {
        Some(RunOutcome::Interrupted)
    } else if stop_on_resize && ctx.terminal.resize_settled() {
        Some(RunOutcome::TerminalResized)
    } else {
        None
    }
}

/// Parity mode: write length-prefixed frames to stdout, no tty escapes.
pub fn dump_effect(
    effect: &mut dyn Effect,
    ctx: &mut EngineCtx,
    max_frames: Option<u64>,
) -> Result<u64, EngineError> {
    effect.build(ctx)?;
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let mut count: u64 = 0;
    while let Some(frame) = effect.next_frame(ctx) {
        let data = frame.as_bytes();
        writeln!(out, "{}", data.len()).map_err(io_err)?;
        out.write_all(data).map_err(io_err)?;
        out.write_all(b"\n").map_err(io_err)?;
        ctx.terminal.recycle_output_string(frame);
        count += 1;
        if max_frames.is_some_and(|m| count >= m) {
            break;
        }
    }
    out.flush().ok();
    eprintln!("frames={count}");
    Ok(count)
}

fn io_err(e: std::io::Error) -> EngineError {
    EngineError::Other(format!("io error: {e}"))
}
