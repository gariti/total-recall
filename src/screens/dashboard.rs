//! Dashboard screen — Space Invaders arcade aesthetic.
//!
//! Canvas + Braille markers for smooth vector-style aliens in a marching
//! formation grid, with cannon at bottom and classic arcade animations.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    symbols::Marker,
    text::{Line, Span},
    widgets::{
        canvas::{Canvas, Circle, Line as CanvasLine, Rectangle},
        Block, Borders, Paragraph,
    },
    Frame,
};
use std::sync::Arc;

use crate::models::agent::{Agent, AgentStatus};
use crate::services::Theme;

/// Dashboard screen state.
pub struct DashboardScreen {
    theme: Arc<Theme>,
    pub selected: usize,
    tick_count: u64,
    /// Tick when selection last changed (for bullet animation).
    bullet_start_tick: u64,
}

impl DashboardScreen {
    pub fn new(theme: Arc<Theme>) -> Self {
        Self {
            theme,
            selected: 0,
            tick_count: 0,
            bullet_start_tick: 0,
        }
    }

    /// Advance animation frame.
    pub fn tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);
    }

    /// Draw the dashboard given the current agents.
    pub fn draw(&self, f: &mut Frame, area: Rect, agents: &[Agent], active_count: usize, status_msg: &str) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(12),    // Canvas arena
                Constraint::Length(5),  // Selected agent detail
                Constraint::Length(2),  // Help bar
            ])
            .split(area);

        self.draw_canvas(f, chunks[0], agents, active_count, status_msg);
        self.draw_detail(f, chunks[1], agents);
        self.draw_help(f, chunks[2], agents.is_empty());
    }

    fn draw_canvas(&self, f: &mut Frame, area: Rect, agents: &[Agent], active_count: usize, status_msg: &str) {
        let tick = self.tick_count;
        let bullet_start = self.bullet_start_tick;
        let theme = self.theme.clone();
        let selected = self.selected;
        let agent_data: Vec<(AgentStatus, String)> = agents
            .iter()
            .map(|a| (a.status.clone(), a.agent_type.clone()))
            .collect();

        let canvas = Canvas::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.color8))
                    .title(Line::from(vec![
                        Span::styled(
                            " TOTAL RECALL ",
                            Style::default()
                                .fg(theme.color6)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!(" {} ACTIVE ", active_count),
                            Style::default().fg(if active_count > 0 {
                                theme.color2
                            } else {
                                theme.color8
                            }),
                        ),
                    ]))
                    .title_bottom(Line::from(Span::styled(
                        format!(" {} ", status_msg),
                        Style::default().fg(theme.color8),
                    )).right_aligned()),
            )
            .marker(Marker::Braille)
            .x_bounds([0.0, 200.0])
            .y_bounds([0.0, 100.0])
            .paint(move |ctx| {
                // === 1. Starfield background ===
                let stars: [(f64, f64, u64); 15] = [
                    (12.0, 92.0, 0), (45.0, 88.0, 3), (78.0, 95.0, 5),
                    (110.0, 91.0, 1), (155.0, 87.0, 4), (180.0, 93.0, 2),
                    (30.0, 78.0, 6), (92.0, 82.0, 1), (140.0, 76.0, 3),
                    (170.0, 80.0, 5), (8.0, 70.0, 2), (60.0, 73.0, 4),
                    (125.0, 68.0, 0), (185.0, 72.0, 6), (50.0, 65.0, 3),
                ];
                for &(sx, sy, seed) in &stars {
                    if (tick + seed) % 7 != 0 {
                        ctx.draw(&Circle { x: sx, y: sy, radius: 0.3, color: theme.color8 });
                    }
                }

                // === 2. Ground line ===
                ctx.draw(&CanvasLine {
                    x1: 0.0, y1: 5.0, x2: 200.0, y2: 5.0,
                    color: theme.color8,
                });

                // === 3. Shield bunkers ===
                for &bx in &[40.0, 80.0, 120.0, 160.0] {
                    // Base
                    ctx.draw(&Rectangle {
                        x: bx - 6.0, y: 15.0, width: 12.0, height: 3.0,
                        color: theme.color2,
                    });
                    // Middle
                    ctx.draw(&Rectangle {
                        x: bx - 4.0, y: 18.0, width: 8.0, height: 3.0,
                        color: theme.color2,
                    });
                    // Top
                    ctx.draw(&Rectangle {
                        x: bx - 2.0, y: 21.0, width: 4.0, height: 2.0,
                        color: theme.color2,
                    });
                }

                // === 4. Player cannon ===
                let cannon_x = 100.0;
                let cannon_y = 5.0;
                draw_cannon(ctx, cannon_x, cannon_y, &theme);

                // === 5-7. Formation grid ===
                let mut selected_pos: Option<(f64, f64)> = None;

                if !agent_data.is_empty() {
                    let count = agent_data.len();
                    let max_cols = 5usize;
                    let col_spacing = 30.0;
                    let row_spacing = 15.0;
                    let formation_top = 85.0;
                    let formation_bottom = 30.0;

                    // Side-to-side drift
                    let drift = ((tick % 20) as f64 / 20.0 * std::f64::consts::TAU).sin() * 8.0;

                    // Build rows
                    let mut rows: Vec<Vec<usize>> = Vec::new();
                    let mut idx = 0;
                    while idx < count {
                        let row_count = max_cols.min(count - idx);
                        let row: Vec<usize> = (idx..idx + row_count).collect();
                        rows.push(row);
                        idx += row_count;
                    }

                    let total_rows = rows.len();
                    for (row_idx, row) in rows.iter().enumerate() {
                        let row_y = if total_rows == 1 {
                            75.0
                        } else {
                            let span = f64::min(formation_top - formation_bottom, row_spacing * (total_rows - 1) as f64);
                            let top_y = 75.0 + span / 2.0;
                            top_y - row_idx as f64 * row_spacing
                        };

                        let row_width = (row.len() - 1) as f64 * col_spacing;
                        let row_start_x = 100.0 - row_width / 2.0;

                        for (col_idx, &agent_idx) in row.iter().enumerate() {
                            let ax = row_start_x + col_idx as f64 * col_spacing + drift;
                            let ay = row_y;
                            let is_selected = agent_idx == selected;
                            let (ref status, ref name) = agent_data[agent_idx];

                            let color = match status {
                                AgentStatus::Starting => theme.color3,
                                AgentStatus::Active => theme.color2,
                                AgentStatus::Idle => theme.color3,
                                AgentStatus::Complete => theme.color6,
                                AgentStatus::Failed | AgentStatus::Killed => theme.color1,
                            };

                            // Alien type based on index
                            draw_alien(ctx, ax, ay, agent_idx % 3, status, tick, color);

                            // Name label below alien
                            let display_name: String = name.chars().take(6).collect();
                            ctx.print(ax - 3.0, ay - 10.0, Line::from(Span::styled(
                                display_name,
                                Style::default().fg(if is_selected {
                                    theme.foreground
                                } else {
                                    theme.color8
                                }).add_modifier(if is_selected { Modifier::BOLD } else { Modifier::empty() }),
                            )));

                            if is_selected {
                                selected_pos = Some((ax, ay));
                            }
                        }
                    }
                }

                // === 8. Reticle on selected alien ===
                if let Some((rx, ry)) = selected_pos {
                    draw_reticle(ctx, rx, ry, tick, theme.color6);
                }

                // === 9. Bullet animation ===
                if let Some((tx, ty)) = selected_pos {
                    let bullet_age = tick.wrapping_sub(bullet_start);
                    if bullet_age < 8 {
                        let t = bullet_age as f64 / 8.0;
                        let bx = cannon_x + (tx - cannon_x) * t;
                        let by = (cannon_y + 11.0) + (ty - (cannon_y + 11.0)) * t;
                        ctx.draw(&Rectangle {
                            x: bx - 0.5, y: by, width: 1.0, height: 3.0,
                            color: theme.color6,
                        });
                    }
                }

                // === 10. "YOU" label + empty state ===
                ctx.print(cannon_x - 2.0, 1.0, Line::from(Span::styled(
                    "YOU",
                    Style::default()
                        .fg(theme.color6)
                        .add_modifier(Modifier::BOLD),
                )));

                if agent_data.is_empty() {
                    ctx.print(60.0, 50.0, Line::from(Span::styled(
                        "Ins to spawn",
                        Style::default().fg(theme.color8),
                    )));
                }
            });

        f.render_widget(canvas, area);
    }

    fn draw_detail(&self, f: &mut Frame, area: Rect, agents: &[Agent]) {
        if agents.is_empty() {
            let empty = Paragraph::new("").block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(self.theme.color8)),
            );
            f.render_widget(empty, area);
            return;
        }

        let agent = match agents.get(self.selected) {
            Some(a) => a,
            None => return,
        };

        let status_color = match agent.status {
            AgentStatus::Starting => self.theme.color3,
            AgentStatus::Active => self.theme.color2,
            AgentStatus::Idle => self.theme.color3,
            AgentStatus::Complete => self.theme.color6,
            AgentStatus::Failed | AgentStatus::Killed => self.theme.color1,
        };

        let project_name = agent
            .project_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?");

        let last_action = agent.last_tool.as_deref().unwrap_or(match agent.status {
            AgentStatus::Starting => "booting...",
            AgentStatus::Complete => "done",
            AgentStatus::Failed => "crashed",
            AgentStatus::Killed => "killed",
            _ => "waiting...",
        });

        let detail = Paragraph::new(vec![
            Line::from(vec![
                Span::styled(
                    format!("  ► {}", agent.agent_type),
                    Style::default()
                        .fg(self.theme.foreground)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("  {} {}", agent.status.icon(), agent.status.label()),
                    Style::default().fg(status_color),
                ),
                Span::styled(
                    format!(
                        "  {}  {}  {}msg",
                        project_name,
                        agent.time_since_activity(),
                        agent.message_count,
                    ),
                    Style::default().fg(self.theme.color8),
                ),
            ]),
            Line::from(vec![Span::styled(
                format!("    \"{}\"", truncate(&agent.task_prompt, 70)),
                Style::default().fg(self.theme.color7),
            )]),
            Line::from(vec![Span::styled(
                format!("    Last: {}", last_action),
                Style::default().fg(self.theme.color5),
            )]),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(self.theme.color8)),
        );
        f.render_widget(detail, area);
    }

    fn draw_help(&self, f: &mut Frame, area: Rect, empty: bool) {
        let help = if empty {
            Paragraph::new(Line::from(vec![
                Span::raw(" "),
                Span::styled("Ins", Style::default().fg(self.theme.color6)),
                Span::styled(" Spawn  ", Style::default().fg(self.theme.color7)),
                Span::styled("Tab", Style::default().fg(self.theme.color6)),
                Span::styled(" Sessions  ", Style::default().fg(self.theme.color7)),
                Span::styled("Esc", Style::default().fg(self.theme.color6)),
                Span::styled(" Quit", Style::default().fg(self.theme.color7)),
            ]))
        } else {
            Paragraph::new(Line::from(vec![
                Span::raw(" "),
                Span::styled("Ins", Style::default().fg(self.theme.color6)),
                Span::styled(" Spawn  ", Style::default().fg(self.theme.color7)),
                Span::styled("←→", Style::default().fg(self.theme.color6)),
                Span::styled(" Select  ", Style::default().fg(self.theme.color7)),
                Span::styled("Enter", Style::default().fg(self.theme.color6)),
                Span::styled(" Focus  ", Style::default().fg(self.theme.color7)),
                Span::styled("F1", Style::default().fg(self.theme.color6)),
                Span::styled(" Attach  ", Style::default().fg(self.theme.color7)),
                Span::styled("Del", Style::default().fg(self.theme.color6)),
                Span::styled(" Kill  ", Style::default().fg(self.theme.color7)),
                Span::styled("Bksp", Style::default().fg(self.theme.color6)),
                Span::styled(" Remove  ", Style::default().fg(self.theme.color7)),
                Span::styled("Tab", Style::default().fg(self.theme.color6)),
                Span::styled(" Sessions  ", Style::default().fg(self.theme.color7)),
                Span::styled("Esc", Style::default().fg(self.theme.color6)),
                Span::styled(" Quit", Style::default().fg(self.theme.color7)),
            ]))
        };
        f.render_widget(help, area);
    }

    pub fn move_left(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.bullet_start_tick = self.tick_count;
        }
    }

    pub fn move_right(&mut self, agent_count: usize) {
        if agent_count > 0 && self.selected < agent_count - 1 {
            self.selected += 1;
            self.bullet_start_tick = self.tick_count;
        }
    }

    pub fn clamp_selection(&mut self, agent_count: usize) {
        if agent_count == 0 {
            self.selected = 0;
        } else if self.selected >= agent_count {
            self.selected = agent_count - 1;
        }
    }

    pub fn selected(&self) -> Option<usize> {
        Some(self.selected)
    }
}

/// Draw the player cannon at bottom center.
fn draw_cannon(
    ctx: &mut ratatui::widgets::canvas::Context<'_>,
    x: f64,
    ground_y: f64,
    theme: &Theme,
) {
    let color = theme.color6;
    let y = ground_y;

    // Base
    ctx.draw(&Rectangle {
        x: x - 6.0, y, width: 12.0, height: 4.0, color,
    });
    // Mid section
    ctx.draw(&Rectangle {
        x: x - 3.0, y: y + 4.0, width: 6.0, height: 3.0, color,
    });
    // Barrel
    ctx.draw(&Rectangle {
        x: x - 1.0, y: y + 7.0, width: 2.0, height: 4.0, color,
    });
}

/// Draw an alien sprite. `alien_type`: 0=squid, 1=crab, 2=octopus.
fn draw_alien(
    ctx: &mut ratatui::widgets::canvas::Context<'_>,
    x: f64,
    y: f64,
    alien_type: usize,
    status: &AgentStatus,
    tick: u64,
    color: ratatui::style::Color,
) {
    match status {
        AgentStatus::Failed | AgentStatus::Killed => {
            draw_alien_explosion(ctx, x, y, tick, color);
            return;
        }
        AgentStatus::Complete => {
            draw_alien_saucer(ctx, x, y, tick, color);
            return;
        }
        AgentStatus::Starting => {
            if tick % 2 != 0 {
                return; // Flicker
            }
        }
        _ => {}
    }

    let frame = (tick / 2 % 2) as usize;

    // Idle: static frame 1, gentle y-bob
    let (draw_frame, bob) = match status {
        AgentStatus::Idle => {
            let bob = ((tick % 8) as f64 * std::f64::consts::PI / 4.0).sin() * 1.5;
            (1, bob)
        }
        _ => (frame, 0.0),
    };

    let ay = y + bob;

    match alien_type {
        0 => draw_squid(ctx, x, ay, draw_frame, color),
        1 => draw_crab(ctx, x, ay, draw_frame, color),
        _ => draw_octopus(ctx, x, ay, draw_frame, color),
    }

    // Spark dots for active
    if *status == AgentStatus::Active {
        let spark_offset = (tick % 5) as f64;
        ctx.draw(&Circle {
            x: x - 2.0 + spark_offset,
            y: ay + 8.0 + spark_offset * 0.3,
            radius: 0.3,
            color,
        });
    }
}

/// Squid alien — narrow body + tentacles.
fn draw_squid(
    ctx: &mut ratatui::widgets::canvas::Context<'_>,
    x: f64,
    y: f64,
    frame: usize,
    color: ratatui::style::Color,
) {
    // Body
    ctx.draw(&Rectangle {
        x: x - 3.0, y: y + 2.0, width: 6.0, height: 5.0, color,
    });
    // Eyes
    ctx.draw(&Circle { x: x - 1.5, y: y + 5.0, radius: 0.6, color });
    ctx.draw(&Circle { x: x + 1.5, y: y + 5.0, radius: 0.6, color });

    // Tentacles — 4 diagonal lines, toggle between splayed/tucked
    let splay = if frame == 0 { 3.0 } else { 1.5 };
    ctx.draw(&CanvasLine { x1: x - 2.0, y1: y + 2.0, x2: x - 2.0 - splay, y2: y - 2.0, color });
    ctx.draw(&CanvasLine { x1: x - 0.5, y1: y + 2.0, x2: x - 0.5 - splay * 0.5, y2: y - 2.0, color });
    ctx.draw(&CanvasLine { x1: x + 0.5, y1: y + 2.0, x2: x + 0.5 + splay * 0.5, y2: y - 2.0, color });
    ctx.draw(&CanvasLine { x1: x + 2.0, y1: y + 2.0, x2: x + 2.0 + splay, y2: y - 2.0, color });
}

/// Crab alien — wide body + claws.
fn draw_crab(
    ctx: &mut ratatui::widgets::canvas::Context<'_>,
    x: f64,
    y: f64,
    frame: usize,
    color: ratatui::style::Color,
) {
    // Body
    ctx.draw(&Rectangle {
        x: x - 5.0, y: y + 1.0, width: 10.0, height: 5.0, color,
    });
    // Eyes
    ctx.draw(&Circle { x: x - 2.0, y: y + 4.5, radius: 0.6, color });
    ctx.draw(&Circle { x: x + 2.0, y: y + 4.5, radius: 0.6, color });

    // Claws — flip up/down per frame
    let claw_dy = if frame == 0 { 2.0 } else { -1.0 };
    // Left claw
    ctx.draw(&CanvasLine { x1: x - 5.0, y1: y + 3.0, x2: x - 8.0, y2: y + 3.0 + claw_dy, color });
    ctx.draw(&CanvasLine { x1: x - 8.0, y1: y + 3.0 + claw_dy, x2: x - 7.0, y2: y + 4.5 + claw_dy, color });
    // Right claw
    ctx.draw(&CanvasLine { x1: x + 5.0, y1: y + 3.0, x2: x + 8.0, y2: y + 3.0 + claw_dy, color });
    ctx.draw(&CanvasLine { x1: x + 8.0, y1: y + 3.0 + claw_dy, x2: x + 7.0, y2: y + 4.5 + claw_dy, color });

    // Legs
    ctx.draw(&CanvasLine { x1: x - 3.0, y1: y + 1.0, x2: x - 4.0, y2: y - 1.0, color });
    ctx.draw(&CanvasLine { x1: x + 3.0, y1: y + 1.0, x2: x + 4.0, y2: y - 1.0, color });
}

/// Octopus alien — round body + hanging legs.
fn draw_octopus(
    ctx: &mut ratatui::widgets::canvas::Context<'_>,
    x: f64,
    y: f64,
    frame: usize,
    color: ratatui::style::Color,
) {
    // Round body
    ctx.draw(&Circle { x, y: y + 4.0, radius: 4.0, color });
    // Eyes
    ctx.draw(&Circle { x: x - 1.5, y: y + 5.0, radius: 0.5, color });
    ctx.draw(&Circle { x: x + 1.5, y: y + 5.0, radius: 0.5, color });

    // 4 hanging legs that sway
    let sway = if frame == 0 { 1.0 } else { -1.0 };
    ctx.draw(&CanvasLine { x1: x - 3.0, y1: y + 1.0, x2: x - 3.0 + sway, y2: y - 3.0, color });
    ctx.draw(&CanvasLine { x1: x - 1.0, y1: y + 0.5, x2: x - 1.0 + sway, y2: y - 3.0, color });
    ctx.draw(&CanvasLine { x1: x + 1.0, y1: y + 0.5, x2: x + 1.0 - sway, y2: y - 3.0, color });
    ctx.draw(&CanvasLine { x1: x + 3.0, y1: y + 1.0, x2: x + 3.0 - sway, y2: y - 3.0, color });
}

/// Explosion animation for dead/killed aliens.
fn draw_alien_explosion(
    ctx: &mut ratatui::widgets::canvas::Context<'_>,
    x: f64,
    y: f64,
    tick: u64,
    color: ratatui::style::Color,
) {
    let phase = (tick / 2) % 4;
    let radius = (phase as f64 + 1.0) * 2.5;

    if phase < 3 {
        // Radiating line segments
        for i in 0..8 {
            let angle = i as f64 * std::f64::consts::PI / 4.0;
            let inner = radius * 0.3;
            ctx.draw(&CanvasLine {
                x1: x + angle.cos() * inner,
                y1: y + angle.sin() * inner,
                x2: x + angle.cos() * radius,
                y2: y + angle.sin() * radius,
                color,
            });
        }
    } else {
        // Debris dots
        for i in 0..6 {
            let angle = i as f64 * std::f64::consts::PI / 3.0 + 0.5;
            let dist = radius * 0.8;
            ctx.draw(&Circle {
                x: x + angle.cos() * dist,
                y: y + angle.sin() * dist,
                radius: 0.4,
                color,
            });
        }
    }
}

/// Saucer shape for completed aliens.
fn draw_alien_saucer(
    ctx: &mut ratatui::widgets::canvas::Context<'_>,
    x: f64,
    y: f64,
    tick: u64,
    color: ratatui::style::Color,
) {
    // Saucer body — wide ellipse approximated as rectangle + dome
    ctx.draw(&Rectangle {
        x: x - 7.0, y: y + 1.0, width: 14.0, height: 3.0, color,
    });
    // Dome
    ctx.draw(&Rectangle {
        x: x - 3.0, y: y + 4.0, width: 6.0, height: 3.0, color,
    });
    // Dome top
    ctx.draw(&CanvasLine { x1: x - 3.0, y1: y + 7.0, x2: x, y2: y + 9.0, color });
    ctx.draw(&CanvasLine { x1: x + 3.0, y1: y + 7.0, x2: x, y2: y + 9.0, color });

    // Pulsing glow circle
    let glow_r = 2.0 + ((tick % 6) as f64 * std::f64::consts::PI / 3.0).sin() * 1.5;
    ctx.draw(&Circle { x, y: y + 5.0, radius: glow_r, color });
}

/// Targeting reticle — pulsing circle + crosshair.
fn draw_reticle(
    ctx: &mut ratatui::widgets::canvas::Context<'_>,
    x: f64,
    y: f64,
    tick: u64,
    color: ratatui::style::Color,
) {
    let pulse = if tick % 6 < 3 { 0.0 } else { 1.5 };
    let r = 10.0 + pulse;
    let gap = 3.0;

    // Pulsing circle
    ctx.draw(&Circle { x, y: y + 3.0, radius: r, color });

    // Crosshair lines with center gap
    let cy = y + 3.0;
    // Left
    ctx.draw(&CanvasLine { x1: x - r, y1: cy, x2: x - gap, y2: cy, color });
    // Right
    ctx.draw(&CanvasLine { x1: x + gap, y1: cy, x2: x + r, y2: cy, color });
    // Down
    ctx.draw(&CanvasLine { x1: x, y1: cy - r, x2: x, y2: cy - gap, color });
    // Up
    ctx.draw(&CanvasLine { x1: x, y1: cy + gap, x2: x, y2: cy + r, color });
}

fn truncate(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() > max {
        format!("{}...", chars[..max].iter().collect::<String>())
    } else {
        s.to_string()
    }
}
