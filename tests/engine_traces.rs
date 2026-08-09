//! Engine state-machine traces vs the reference implementation
//! (tools/goldens/gen_engine_traces.py). Each scenario replays identically and
//! the full event + state log must match line for line.

use std::collections::BTreeSet;

use ttfx::engine::animation::{SyncMetric, VisualParams};
use ttfx::engine::character::CharId;
use ttfx::engine::ctx::{Clock, EngineCtx, NoopHooks};
use ttfx::engine::events::{CallerKey, Event, EventAction};
use ttfx::engine::terminal::{CharacterFilter, CharacterSort, TerminalConfig};
use ttfx::utils::easing::Easing;
use ttfx::utils::geometry::Coord;
use ttfx::utils::graphics::{Color, ColorPair, Gradient};
use ttfx::utils::rng::Rng;

fn make_ctx() -> EngineCtx {
    let config = TerminalConfig {
        canvas_width: 20,
        canvas_height: 10,
        ignore_terminal_dimensions: true,
        frame_rate: 0,
        ..Default::default()
    };
    let mut ctx = EngineCtx::new(
        "abcdef\nghijkl",
        config,
        Rng::seeded(0),
        Clock::virtual_with_frame_rate(60),
    )
    .unwrap();
    ctx.event_log = Some(Vec::new());
    ctx
}

fn chars(ctx: &mut EngineCtx, n: usize) -> Vec<CharId> {
    let filter = CharacterFilter::default();
    let mut rng = Rng::seeded(0); // sort is non-random; rng unused
    ctx.terminal.get_characters(&mut rng, filter, CharacterSort::TopToBottomLeftToRight)[..n].to_vec()
}

fn esc(s: &str) -> String {
    s.replace('\x1b', "\\e")
}

fn snapshot(ctx: &mut EngineCtx, log: &mut Vec<String>, tick: i64, ids: &[CharId]) {
    // event log entries accumulated inside ctx get flushed first
    log.append(ctx.event_log.as_mut().unwrap());
    for &id in ids {
        let ch = &ctx.terminal.arena[id.0 as usize];
        let ap = ch.motion.active_path.clone().unwrap_or_else(|| "-".to_string());
        let sc = ch.animation.active_scene.clone().unwrap_or_else(|| "-".to_string());
        let active = if ch.is_active() { "True" } else { "False" };
        log.push(format!(
            "tick={tick} char={} coord={},{} layer={} path={ap} scene={sc} vis={} active={active}",
            ch.character_id,
            ch.motion.current_coord.column,
            ch.motion.current_coord.row,
            ch.layer,
            esc(&ch.animation.current_character_visual.formatted_symbol),
        ));
    }
}

fn run_ticks(ctx: &mut EngineCtx, log: &mut Vec<String>, ids: &[CharId], n: i64, start: i64) {
    let mut active: BTreeSet<CharId> = ids.iter().copied().collect();
    for tick in start..start + n {
        let snapshot_ids: Vec<CharId> = active.iter().copied().collect();
        for id in snapshot_ids {
            ctx.tick(&mut NoopHooks, id);
        }
        let inactive: Vec<CharId> =
            active.iter().copied().filter(|&id| !ctx.terminal.arena[id.0 as usize].is_active()).collect();
        for id in inactive {
            active.remove(&id);
        }
        snapshot(ctx, log, tick, ids);
    }
}

fn flush(ctx: &mut EngineCtx, log: &mut Vec<String>) {
    log.append(ctx.event_log.as_mut().unwrap());
}

fn scenario_motion_basic(log: &mut Vec<String>) {
    log.push("=== scenario_motion_basic ===".into());
    let mut ctx = make_ctx();
    let ids = chars(&mut ctx, 2);
    let (a, b) = (ids[0], ids[1]);
    {
        let motion = &mut ctx.terminal.arena[a.0 as usize].motion;
        motion.new_path(0.7, None, None, 0, false, "pa").unwrap();
        let pa = motion.paths.get_mut("pa").unwrap();
        pa.new_waypoint(Coord::new(15, 8), None, "").unwrap();
        pa.new_waypoint(Coord::new(18, 2), Some(vec![Coord::new(1, 1)]), "").unwrap();
        let motion = &mut ctx.terminal.arena[b.0 as usize].motion;
        motion.new_path(1.3, Some(Easing::OutBack), None, 0, false, "pb").unwrap();
        motion.paths.get_mut("pb").unwrap().new_waypoint(Coord::new(3, 9), None, "").unwrap();
    }
    ctx.activate_path(&mut NoopHooks, a, "pa");
    ctx.activate_path(&mut NoopHooks, b, "pb");
    run_ticks(&mut ctx, log, &ids, 30, 0);
    flush(&mut ctx, log);
}

fn scenario_hold_and_loop(log: &mut Vec<String>) {
    log.push("=== scenario_hold_and_loop ===".into());
    let mut ctx = make_ctx();
    let ids = chars(&mut ctx, 2);
    let (a, b) = (ids[0], ids[1]);
    {
        let motion = &mut ctx.terminal.arena[a.0 as usize].motion;
        motion.new_path(2.0, None, None, 3, false, "hold").unwrap();
        motion.paths.get_mut("hold").unwrap().new_waypoint(Coord::new(10, 5), None, "").unwrap();
        let motion = &mut ctx.terminal.arena[b.0 as usize].motion;
        motion.new_path(2.0, None, None, 0, true, "looper").unwrap();
        let pb = motion.paths.get_mut("looper").unwrap();
        pb.new_waypoint(Coord::new(6, 3), None, "").unwrap();
        pb.new_waypoint(Coord::new(9, 6), None, "").unwrap();
    }
    ctx.activate_path(&mut NoopHooks, a, "hold");
    ctx.activate_path(&mut NoopHooks, b, "looper");
    run_ticks(&mut ctx, log, &ids, 20, 0);
    flush(&mut ctx, log);
}

fn scenario_chained_paths_and_events(log: &mut Vec<String>) {
    log.push("=== scenario_chained_paths_and_events ===".into());
    let mut ctx = make_ctx();
    let ids = chars(&mut ctx, 1);
    let a = ids[0];
    {
        let motion = &mut ctx.terminal.arena[a.0 as usize].motion;
        motion.new_path(1.5, None, None, 0, false, "p1").unwrap();
        motion.paths.get_mut("p1").unwrap().new_waypoint(Coord::new(5, 5), None, "").unwrap();
        motion.new_path(1.5, None, Some(2), 0, false, "p2").unwrap();
        motion.paths.get_mut("p2").unwrap().new_waypoint(Coord::new(10, 2), None, "").unwrap();
        motion.new_path(1.5, None, None, 0, false, "p3").unwrap();
        motion.paths.get_mut("p3").unwrap().new_waypoint(Coord::new(1, 1), None, "").unwrap();
    }
    ctx.chain_paths(a, &["p1".into(), "p2".into(), "p3".into()], false).unwrap();
    ctx.register_event(
        a,
        Event::PathComplete,
        CallerKey::Path("p3".into()),
        EventAction::SetCoordinate(Coord::new(19, 9)),
    )
    .unwrap();
    ctx.register_event(a, Event::PathHolding, CallerKey::Path("p1".into()), EventAction::SetLayer(7)).unwrap();
    ctx.activate_path(&mut NoopHooks, a, "p1");
    run_ticks(&mut ctx, log, &ids, 25, 0);
    flush(&mut ctx, log);
}

fn scenario_scenes(log: &mut Vec<String>) {
    log.push("=== scenario_scenes ===".into());
    let mut ctx = make_ctx();
    let ids = chars(&mut ctx, 3);
    let (a, b, c) = (ids[0], ids[1], ids[2]);
    {
        let ch = &mut ctx.terminal.arena[a.0 as usize];
        let uses = ch.uses_input_preexisting_colors;
        ch.animation.new_scene(false, None, None, "plain", uses);
        let scene = ch.animation.scenes.get_mut("plain").unwrap();
        scene
            .add_frame(
                "X",
                2,
                VisualParams {
                    colors: Some(ColorPair::new(Some(Color::from_hex("ff0000").unwrap()), None)),
                    ..Default::default()
                },
            )
            .unwrap();
        scene
            .add_frame(
                "Y",
                3,
                VisualParams {
                    colors: Some(ColorPair::new(
                        Some(Color::from_hex("00ff00").unwrap()),
                        Some(Color::from_xterm(21)),
                    )),
                    ..Default::default()
                },
            )
            .unwrap();
        scene.add_frame("Z", 1, VisualParams { bold: true, ..Default::default() }).unwrap();
    }
    ctx.activate_scene(&mut NoopHooks, a, "plain");
    {
        let ch = &mut ctx.terminal.arena[b.0 as usize];
        let uses = ch.uses_input_preexisting_colors;
        ch.animation.new_scene(true, None, None, "looping", uses);
        let scene = ch.animation.scenes.get_mut("looping").unwrap();
        scene.add_frame("1", 2, VisualParams::default()).unwrap();
        scene.add_frame("2", 2, VisualParams::default()).unwrap();
    }
    ctx.activate_scene(&mut NoopHooks, b, "looping");
    {
        let ch = &mut ctx.terminal.arena[c.0 as usize];
        let uses = ch.uses_input_preexisting_colors;
        ch.animation.new_scene(false, None, Some(Easing::InOutCubic), "eased", uses);
        let grad = Gradient::with_steps(
            &[Color::from_hex("000000").unwrap(), Color::from_hex("ffffff").unwrap()],
            8,
            false,
        )
        .unwrap();
        let scene = ch.animation.scenes.get_mut("eased").unwrap();
        scene
            .apply_gradient_to_symbols(&["*".into(), "+".into(), "o".into()], 2, Some(&grad), None)
            .unwrap();
    }
    ctx.activate_scene(&mut NoopHooks, c, "eased");
    run_ticks(&mut ctx, log, &ids, 24, 0);
    flush(&mut ctx, log);
}

fn scenario_synced_scene(log: &mut Vec<String>) {
    log.push("=== scenario_synced_scene ===".into());
    let mut ctx = make_ctx();
    let ids = chars(&mut ctx, 2);
    for (idx, (sync, pid)) in [(SyncMetric::Step, "sp"), (SyncMetric::Distance, "dp")].iter().enumerate() {
        let id = ids[idx];
        let scene_id = format!("sync_{pid}");
        {
            let ch = &mut ctx.terminal.arena[id.0 as usize];
            ch.motion.new_path(0.9, None, None, 0, false, pid).unwrap();
            let path = ch.motion.paths.get_mut(pid).unwrap();
            path.new_waypoint(Coord::new(16, 9), None, "").unwrap();
            path.new_waypoint(Coord::new(2, 2), None, "").unwrap();
            let uses = ch.uses_input_preexisting_colors;
            ch.animation.new_scene(false, Some(*sync), None, &scene_id, uses);
            let scene = ch.animation.scenes.get_mut(&scene_id).unwrap();
            for sym in ["a", "b", "c", "d", "e", "f", "g", "h"] {
                scene.add_frame(sym, 1, VisualParams::default()).unwrap();
            }
        }
        ctx.activate_path(&mut NoopHooks, id, pid);
        ctx.activate_scene(&mut NoopHooks, id, &scene_id);
    }
    run_ticks(&mut ctx, log, &ids, 30, 0);
    flush(&mut ctx, log);
}

fn scenario_scene_events_and_resume(log: &mut Vec<String>) {
    log.push("=== scenario_scene_events_and_resume ===".into());
    let mut ctx = make_ctx();
    let ids = chars(&mut ctx, 1);
    let a = ids[0];
    {
        let ch = &mut ctx.terminal.arena[a.0 as usize];
        let uses = ch.uses_input_preexisting_colors;
        ch.animation.new_scene(false, None, None, "s1", uses);
        let s1 = ch.animation.scenes.get_mut("s1").unwrap();
        s1.add_frame("A", 3, VisualParams::default()).unwrap();
        s1.add_frame("B", 3, VisualParams::default()).unwrap();
        ch.animation.new_scene(false, None, None, "s2", uses);
        ch.animation.scenes.get_mut("s2").unwrap().add_frame("C", 2, VisualParams::default()).unwrap();
        ch.motion.new_path(1.0, None, None, 0, false, "mover").unwrap();
        ch.motion.paths.get_mut("mover").unwrap().new_waypoint(Coord::new(8, 8), None, "").unwrap();
    }
    ctx.register_event(a, Event::SceneComplete, CallerKey::Scene("s1".into()), EventAction::ActivateScene("s2".into()))
        .unwrap();
    ctx.register_event(a, Event::SceneComplete, CallerKey::Scene("s2".into()), EventAction::ActivatePath("mover".into()))
        .unwrap();
    ctx.activate_scene(&mut NoopHooks, a, "s1");
    run_ticks(&mut ctx, log, &ids, 2, 0);
    ctx.activate_scene(&mut NoopHooks, a, "s1");
    run_ticks(&mut ctx, log, &ids, 20, 2);
    flush(&mut ctx, log);
}

#[test]
fn engine_traces_match_python() {
    let fixture = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/engine_traces.txt"))
        .expect("run tools/goldens/gen_engine_traces.py first");
    let expected: Vec<&str> = fixture.lines().collect();

    let mut log: Vec<String> = Vec::new();
    scenario_motion_basic(&mut log);
    scenario_hold_and_loop(&mut log);
    scenario_chained_paths_and_events(&mut log);
    scenario_scenes(&mut log);
    scenario_synced_scene(&mut log);
    scenario_scene_events_and_resume(&mut log);

    let mut mismatches = 0;
    for i in 0..expected.len().max(log.len()) {
        let e = expected.get(i).copied().unwrap_or("<missing>");
        let a = log.get(i).map(String::as_str).unwrap_or("<missing>");
        if e != a {
            if mismatches < 8 {
                eprintln!("line {i}:\n  expected: {e}\n    actual: {a}");
            }
            mismatches += 1;
        }
    }
    assert_eq!(mismatches, 0, "{mismatches} trace lines diverge (of {})", expected.len());
}
