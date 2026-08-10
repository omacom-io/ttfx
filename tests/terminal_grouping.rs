use ttfx::engine::character::CharId;
use ttfx::engine::terminal::{CharacterFilter, CharacterGroup, Terminal, TerminalConfig};
use ttfx::utils::geometry::Coord;

const GROUPINGS: [CharacterGroup; 10] = [
    CharacterGroup::ColumnLeftToRight,
    CharacterGroup::ColumnRightToLeft,
    CharacterGroup::RowTopToBottom,
    CharacterGroup::RowBottomToTop,
    CharacterGroup::DiagonalBottomLeftToTopRight,
    CharacterGroup::DiagonalTopRightToBottomLeft,
    CharacterGroup::DiagonalTopLeftToBottomRight,
    CharacterGroup::DiagonalBottomRightToTopLeft,
    CharacterGroup::CenterToOutside,
    CharacterGroup::OutsideToCenter,
];

fn reference_grouping(
    terminal: &Terminal,
    filter: CharacterFilter,
    grouping: CharacterGroup,
) -> Vec<Vec<CharId>> {
    let mut all = terminal.collect_characters(filter);
    all.sort_by_key(|&id| {
        let c = terminal.arena[id.0 as usize].input_coord;
        (c.row, c.column)
    });
    let coord = |id: &CharId| terminal.arena[id.0 as usize].input_coord;
    match grouping {
        CharacterGroup::ColumnLeftToRight | CharacterGroup::ColumnRightToLeft => {
            let mut groups = Vec::new();
            for key in 0..=terminal.canvas.right {
                let group = all.iter().copied().filter(|id| coord(id).column == key).collect::<Vec<_>>();
                if !group.is_empty() {
                    groups.push(group);
                }
            }
            if grouping == CharacterGroup::ColumnRightToLeft {
                groups.reverse();
            }
            groups
        }
        CharacterGroup::RowBottomToTop | CharacterGroup::RowTopToBottom => {
            let mut groups = Vec::new();
            for key in 0..=terminal.canvas.top {
                let group = all.iter().copied().filter(|id| coord(id).row == key).collect::<Vec<_>>();
                if !group.is_empty() {
                    groups.push(group);
                }
            }
            if grouping == CharacterGroup::RowTopToBottom {
                groups.reverse();
            }
            groups
        }
        CharacterGroup::DiagonalBottomLeftToTopRight | CharacterGroup::DiagonalTopRightToBottomLeft => {
            let mut groups = Vec::new();
            for key in 0..=(terminal.canvas.top + terminal.canvas.right) {
                let group = all
                    .iter()
                    .copied()
                    .filter(|id| {
                        let c = coord(id);
                        c.row + c.column == key
                    })
                    .collect::<Vec<_>>();
                if !group.is_empty() {
                    groups.push(group);
                }
            }
            if grouping == CharacterGroup::DiagonalTopRightToBottomLeft {
                groups.reverse();
            }
            groups
        }
        CharacterGroup::DiagonalTopLeftToBottomRight | CharacterGroup::DiagonalBottomRightToTopLeft => {
            let mut groups = Vec::new();
            for key in
                (terminal.canvas.left - terminal.canvas.top)..=(terminal.canvas.right - terminal.canvas.bottom)
            {
                let group = all
                    .iter()
                    .copied()
                    .filter(|id| {
                        let c = coord(id);
                        c.column - c.row == key
                    })
                    .collect::<Vec<_>>();
                if !group.is_empty() {
                    groups.push(group);
                }
            }
            if grouping == CharacterGroup::DiagonalBottomRightToTopLeft {
                groups.reverse();
            }
            groups
        }
        CharacterGroup::CenterToOutside | CharacterGroup::OutsideToCenter => {
            let mut distances: Vec<(i64, Vec<CharId>)> = Vec::new();
            for id in all {
                let c = coord(&id);
                let distance = (c.column - terminal.canvas.text_center.column).abs()
                    + (c.row - terminal.canvas.text_center.row).abs();
                if let Some((_, group)) = distances.iter_mut().find(|(key, _)| *key == distance) {
                    group.push(id);
                } else {
                    distances.push((distance, vec![id]));
                }
            }
            distances.sort_by_key(|(distance, _)| *distance);
            if grouping == CharacterGroup::OutsideToCenter {
                distances.reverse();
            }
            distances.into_iter().map(|(_, group)| group).collect()
        }
    }
}

#[test]
fn direct_buckets_match_scan_grouping_order_and_bounds() {
    let mut terminal = Terminal::new(
        "A C\n DE\nF G",
        TerminalConfig {
            canvas_width: 8,
            canvas_height: 6,
            ignore_terminal_dimensions: true,
            ..Default::default()
        },
    )
    .unwrap();

    for coord in [
        Coord::new(0, 0),
        Coord::new(-5, 2),
        Coord::new(2, 99),
        Coord::new(1, 3),
        Coord::new(1, 3),
        Coord::new(99, 99),
    ] {
        terminal.add_character("+", coord);
    }

    let filters = [
        CharacterFilter::default(),
        CharacterFilter {
            input_chars: true,
            inner_fill_chars: true,
            outer_fill_chars: true,
            added_chars: true,
        },
        CharacterFilter {
            input_chars: false,
            inner_fill_chars: false,
            outer_fill_chars: false,
            added_chars: true,
        },
        CharacterFilter {
            input_chars: false,
            inner_fill_chars: false,
            outer_fill_chars: false,
            added_chars: false,
        },
    ];

    for filter in filters {
        for grouping in GROUPINGS {
            assert_eq!(
                terminal.get_characters_grouped(filter, grouping),
                reference_grouping(&terminal, filter, grouping),
                "grouping {grouping:?} diverged"
            );
        }
    }
}
