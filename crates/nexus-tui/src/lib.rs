//! Minimal read-only TUI over a pre-built [`PlanDocument`].
//! Inspection and export only — no dashboard charts, no live mutation of scores.

use anyhow::{bail, Result};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use nexus_core::{ActionType, ClusterPlan, ClusterStatus, MemberKind, PlanDocument, Priority};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Scrollbar,
    ScrollbarOrientation, ScrollbarState, Table, TableState,
};
use ratatui::{DefaultTerminal, Frame};
use std::collections::HashSet;
use std::io::{self, IsTerminal};

// ── Palette ──────────────────────────────────────────────────────────────────
const FG: Color = Color::White;
const FG_DIM: Color = Color::DarkGray;
const FG_MUTED: Color = Color::Gray;
const ACCENT: Color = Color::Cyan;
const ACCENT_DIM: Color = Color::DarkGray;
const SCORE_HIGH: Color = Color::Green;
const SCORE_MID: Color = Color::Yellow;
const SCORE_LOW: Color = Color::Red;
const STATUS_OK: Color = Color::Green;
const STATUS_AMB: Color = Color::Yellow;
const STATUS_REV: Color = Color::Red;
const BORDER: Color = Color::DarkGray;
const HIGHLIGHT_BG: Color = Color::Indexed(236); // subtle dark gray
const TITLE_FG: Color = Color::Cyan;
const KEY_FG: Color = Color::Cyan;
const BAR_FILLED: &str = "━";
const BAR_EMPTY: &str = "╌";

fn border_style() -> Style {
    Style::new().fg(BORDER)
}
fn title_style() -> Style {
    Style::new().fg(TITLE_FG).add_modifier(Modifier::BOLD)
}
fn dim() -> Style {
    Style::new().fg(FG_DIM)
}
fn muted() -> Style {
    Style::new().fg(FG_MUTED)
}
fn key_style() -> Style {
    Style::new().fg(KEY_FG).add_modifier(Modifier::BOLD)
}

fn score_color(v: f64) -> Color {
    if v >= 70.0 {
        SCORE_HIGH
    } else if v >= 40.0 {
        SCORE_MID
    } else {
        SCORE_LOW
    }
}

fn score_bar(v: f64, width: usize) -> Vec<Span<'static>> {
    let clamped = v.clamp(0.0, 100.0);
    let filled = ((clamped / 100.0) * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    let color = score_color(v);
    vec![
        Span::styled(BAR_FILLED.repeat(filled), Style::new().fg(color)),
        Span::styled(BAR_EMPTY.repeat(empty), dim()),
        Span::styled(format!(" {clamped:3.0}"), Style::new().fg(color)),
    ]
}

fn priority_style(p: &Priority) -> Style {
    match p {
        Priority::High => Style::new().fg(Color::Red).add_modifier(Modifier::BOLD),
        Priority::Medium => Style::new().fg(Color::Yellow),
        Priority::Low => Style::new().fg(FG_DIM),
    }
}

fn action_type_label(t: &ActionType) -> &'static str {
    match t {
        ActionType::MarkCanonical => "Mark Canonical",
        ActionType::ArchiveLocalDuplicate => "Archive Duplicate",
        ActionType::ReviewAmbiguousCluster => "Review Ambiguous",
        ActionType::MergeDivergedClone => "Merge Diverged",
        ActionType::CreateRemoteRepo => "Create Remote",
        ActionType::CloneLocalWorkspace => "Clone Locally",
        ActionType::AddMissingDocs => "Add Docs",
        ActionType::AddLicense => "Add License",
        ActionType::AddCi => "Add CI",
        ActionType::RunSecurityScans => "Security Scan",
        ActionType::GenerateSbom => "Generate SBOM",
        ActionType::PublishOssCandidate => "Publish Candidate",
    }
}

// ── Sort ─────────────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SortKey {
    Label,
    CanonicalDesc,
    HealthDesc,
    RiskDesc,
    StatusAmbiguousFirst,
}

impl SortKey {
    fn next(self) -> Self {
        match self {
            Self::Label => Self::CanonicalDesc,
            Self::CanonicalDesc => Self::HealthDesc,
            Self::HealthDesc => Self::RiskDesc,
            Self::RiskDesc => Self::StatusAmbiguousFirst,
            Self::StatusAmbiguousFirst => Self::Label,
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Label => "Label A→Z",
            Self::CanonicalDesc => "Canonical ↓",
            Self::HealthDesc => "Health ↓",
            Self::RiskDesc => "Risk ↓",
            Self::StatusAmbiguousFirst => "Ambiguous first",
        }
    }
}

// ── Tab / Screen ─────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Detail,
    Actions,
}

impl Tab {
    fn toggle(self) -> Self {
        match self {
            Self::Detail => Self::Actions,
            Self::Actions => Self::Detail,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Overlay {
    None,
    Evidence,
    Help,
    PinHint,
}

// ── Public API ───────────────────────────────────────────────────────────────
pub struct TuiConfig {
    pub config_pins: HashSet<String>,
}

pub fn run(plan: PlanDocument, config: TuiConfig) -> Result<()> {
    if !io::stdout().is_terminal() {
        bail!("`nexus tui` requires an interactive terminal (stdout is not a TTY)");
    }
    let mut terminal = ratatui::try_init().map_err(|e| anyhow::anyhow!(e))?;
    let res = run_inner(&mut terminal, plan, config);
    let _ = ratatui::try_restore();
    res
}

fn run_inner(terminal: &mut DefaultTerminal, plan: PlanDocument, config: TuiConfig) -> Result<()> {
    let mut app = App::new(plan, config);
    loop {
        terminal
            .draw(|f| app.render(f))
            .map_err(|e| anyhow::anyhow!(e))?;
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Release {
                continue;
            }
            if app.handle_key(key.code, key.modifiers) {
                break;
            }
        }
    }
    Ok(())
}

// ── App state ────────────────────────────────────────────────────────────────
struct App {
    plan: PlanDocument,
    config: TuiConfig,
    sort: SortKey,
    filter_applied: String,
    filter_editing: bool,
    filter_buffer: String,
    ordered: Vec<usize>,
    table_state: TableState,
    bottom_tab: Tab,
    overlay: Overlay,
    evidence_list_state: ListState,
    evidence_lines: Vec<EvidenceLine>,
    action_list_state: ListState,
    status_msg: String,
}

struct EvidenceLine {
    kind: String,
    delta: f64,
    subject: String,
    detail: String,
}

impl App {
    fn new(plan: PlanDocument, config: TuiConfig) -> Self {
        let mut s = Self {
            plan,
            config,
            sort: SortKey::Label,
            filter_applied: String::new(),
            filter_editing: false,
            filter_buffer: String::new(),
            ordered: Vec::new(),
            table_state: TableState::default(),
            bottom_tab: Tab::Detail,
            overlay: Overlay::None,
            evidence_list_state: ListState::default(),
            evidence_lines: Vec::new(),
            action_list_state: ListState::default(),
            status_msg: String::new(),
        };
        s.rebuild_ordered();
        s
    }

    fn rebuild_ordered(&mut self) {
        let n = self.plan.clusters.len();
        let needle = self.filter_applied.to_lowercase();
        let mut v: Vec<usize> = (0..n)
            .filter(|&i| {
                if needle.is_empty() {
                    return true;
                }
                let c = &self.plan.clusters[i].cluster;
                c.label.to_lowercase().contains(&needle)
                    || c.cluster_key.to_lowercase().contains(&needle)
            })
            .collect();

        let clusters = &self.plan.clusters;
        match self.sort {
            SortKey::Label => v.sort_by(|a, b| {
                clusters[*a]
                    .cluster
                    .label
                    .to_lowercase()
                    .cmp(&clusters[*b].cluster.label.to_lowercase())
            }),
            SortKey::CanonicalDesc => v.sort_by(|a, b| {
                clusters[*b]
                    .cluster
                    .scores
                    .canonical
                    .partial_cmp(&clusters[*a].cluster.scores.canonical)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            SortKey::HealthDesc => v.sort_by(|a, b| {
                clusters[*b]
                    .cluster
                    .scores
                    .usability
                    .partial_cmp(&clusters[*a].cluster.scores.usability)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            SortKey::RiskDesc => v.sort_by(|a, b| {
                clusters[*b]
                    .cluster
                    .scores
                    .risk
                    .partial_cmp(&clusters[*a].cluster.scores.risk)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            SortKey::StatusAmbiguousFirst => v.sort_by(|a, b| {
                status_rank(&clusters[*a].cluster.status)
                    .cmp(&status_rank(&clusters[*b].cluster.status))
                    .then_with(|| {
                        clusters[*a]
                            .cluster
                            .label
                            .to_lowercase()
                            .cmp(&clusters[*b].cluster.label.to_lowercase())
                    })
            }),
        }

        self.ordered = v;
        let sel = self.table_state.selected().unwrap_or(0);
        let max = self.ordered.len().saturating_sub(1);
        self.table_state.select(if self.ordered.is_empty() {
            None
        } else {
            Some(sel.min(max))
        });
    }

    fn selected_cluster(&self) -> Option<&ClusterPlan> {
        let i = self.table_state.selected()?;
        let idx = *self.ordered.get(i)?;
        self.plan.clusters.get(idx)
    }

    fn total_actions(&self) -> usize {
        self.plan.clusters.iter().map(|cp| cp.actions.len()).sum()
    }

    fn count_ambiguous(&self) -> usize {
        self.plan
            .clusters
            .iter()
            .filter(|cp| matches!(cp.cluster.status, ClusterStatus::Ambiguous))
            .count()
    }

    // ── Rendering ────────────────────────────────────────────────────────

    fn render(&mut self, f: &mut Frame) {
        let area = f.area();
        // Top status bar + main body
        let outer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(6),
                Constraint::Length(1),
            ])
            .split(area);

        self.render_header_bar(f, outer[0]);
        self.render_main(f, outer[1]);
        self.render_footer_bar(f, outer[2]);

        match self.overlay {
            Overlay::Evidence => self.render_evidence_overlay(f, area),
            Overlay::Help => self.render_help_overlay(f, area),
            Overlay::PinHint => self.render_pin_overlay(f, area),
            Overlay::None => {}
        }
    }

    fn render_header_bar(&self, f: &mut Frame, area: Rect) {
        let n_clusters = self.plan.clusters.len();
        let n_filtered = self.ordered.len();
        let n_actions = self.total_actions();
        let n_ambiguous = self.count_ambiguous();

        let mut spans = vec![
            Span::styled(
                " NEXUS ",
                Style::new()
                    .fg(Color::Black)
                    .bg(ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  ", Style::new()),
        ];

        if !self.filter_applied.is_empty() {
            spans.push(Span::styled(
                format!("{n_filtered}/{n_clusters} clusters"),
                Style::new().fg(FG),
            ));
        } else {
            spans.push(Span::styled(
                format!("{n_clusters} clusters"),
                Style::new().fg(FG),
            ));
        }

        spans.push(Span::styled("  ", Style::new()));
        spans.push(Span::styled(format!("{n_actions} actions"), muted()));

        if n_ambiguous > 0 {
            spans.push(Span::styled("  ", Style::new()));
            spans.push(Span::styled(
                format!("{n_ambiguous} ambiguous"),
                Style::new().fg(STATUS_AMB),
            ));
        }

        spans.push(Span::styled("  ", Style::new()));
        spans.push(Span::styled(
            format!("rules v{}", self.plan.scoring_rules_version),
            dim(),
        ));

        // Right-aligned sort indicator
        let sort_text = format!("sort: {} ", self.sort.label());
        let used: usize = spans.iter().map(|s| s.content.chars().count()).sum();
        let pad = (area.width as usize).saturating_sub(used + sort_text.chars().count());
        spans.push(Span::raw(" ".repeat(pad)));
        spans.push(Span::styled(sort_text, dim()));

        f.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    fn render_footer_bar(&self, f: &mut Frame, area: Rect) {
        if self.filter_editing {
            let spans = vec![
                Span::styled(
                    " FILTER ",
                    Style::new()
                        .fg(Color::Black)
                        .bg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!(" {}▏  ", self.filter_buffer)),
                Span::styled("Enter", key_style()),
                Span::raw(" apply  "),
                Span::styled("Esc", key_style()),
                Span::raw(" cancel"),
            ];
            f.render_widget(Paragraph::new(Line::from(spans)), area);
            return;
        }

        if !self.status_msg.is_empty() {
            let spans = vec![
                Span::styled(
                    " ✓ ",
                    Style::new()
                        .fg(Color::Black)
                        .bg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" {} ", self.status_msg),
                    Style::new().fg(Color::Green),
                ),
            ];
            f.render_widget(Paragraph::new(Line::from(spans)), area);
            return;
        }

        let keys: Vec<(&str, &str)> = vec![
            ("q", "quit"),
            ("↑↓", "nav"),
            ("s", "sort"),
            ("/", "filter"),
            ("f", "clear"),
            ("Tab", "panel"),
            ("e", "evidence"),
            ("a", "actions"),
            ("p", "pin"),
            ("o", "export"),
            ("?", "help"),
        ];
        let mut spans = Vec::new();
        for (i, (k, desc)) in keys.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(" │ ", dim()));
            }
            spans.push(Span::styled(format!(" {k}"), key_style()));
            spans.push(Span::styled(format!(" {desc}"), muted()));
        }
        f.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    fn render_main(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);

        self.render_cluster_table(f, chunks[0]);
        self.render_bottom_panel(f, chunks[1]);
    }

    fn render_cluster_table(&mut self, f: &mut Frame, area: Rect) {
        let header_cells = [
            " Label", "Canon", "Health", "Recov", "Pub", "Risk", "Ev", "Act", "Status",
        ]
        .into_iter()
        .map(|h| Cell::from(h).style(Style::new().fg(ACCENT).add_modifier(Modifier::BOLD)));
        let header = Row::new(header_cells).height(1);

        let rows: Vec<Row> = self
            .ordered
            .iter()
            .map(|&idx| {
                let cp = &self.plan.clusters[idx];
                let c = &cp.cluster;
                let is_pinned = c
                    .canonical_clone_id
                    .as_ref()
                    .map(|id| self.config.config_pins.contains(id))
                    .unwrap_or(false);

                let label_text = if is_pinned {
                    format!("▪ {}", truncate(&c.label, 23))
                } else {
                    format!("  {}", truncate(&c.label, 23))
                };

                let (st_text, st_color) = match c.status {
                    ClusterStatus::Resolved => ("  OK ", STATUS_OK),
                    ClusterStatus::Ambiguous => (" AMB ", STATUS_AMB),
                    ClusterStatus::ManualReview => (" REV ", STATUS_REV),
                };

                Row::new(vec![
                    Cell::from(label_text).style(Style::new().fg(FG)),
                    Cell::from(format!("{:>4.0}", c.scores.canonical))
                        .style(Style::new().fg(score_color(c.scores.canonical))),
                    Cell::from(format!("{:>4.0}", c.scores.usability))
                        .style(Style::new().fg(score_color(c.scores.usability))),
                    Cell::from(format!("{:>4.0}", c.scores.recoverability))
                        .style(Style::new().fg(score_color(c.scores.recoverability))),
                    Cell::from(format!("{:>4.0}", c.scores.oss_readiness))
                        .style(Style::new().fg(score_color(c.scores.oss_readiness))),
                    Cell::from(format!("{:>4.0}", c.scores.risk))
                        .style(Style::new().fg(score_color(100.0 - c.scores.risk))),
                    Cell::from(format!("{:>3}", c.evidence.len())).style(dim()),
                    Cell::from(format!("{:>3}", cp.actions.len())).style(dim()),
                    Cell::from(st_text).style(Style::new().fg(st_color)),
                ])
            })
            .collect();

        let filter_hint = if self.filter_applied.is_empty() {
            String::new()
        } else {
            format!("  filter: \"{}\"", self.filter_applied)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style())
            .title(Line::from(vec![
                Span::styled(" Clusters ", title_style()),
                Span::styled(filter_hint, Style::new().fg(Color::Yellow)),
                Span::raw(" "),
            ]));

        let table = Table::new(
            rows,
            [
                Constraint::Min(26),
                Constraint::Length(5),
                Constraint::Length(6),
                Constraint::Length(6),
                Constraint::Length(5),
                Constraint::Length(5),
                Constraint::Length(4),
                Constraint::Length(4),
                Constraint::Length(6),
            ],
        )
        .header(header)
        .block(block)
        .row_highlight_style(
            Style::new()
                .bg(HIGHLIGHT_BG)
                .fg(FG)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸ ");

        let n = self.ordered.len();
        f.render_stateful_widget(table, area, &mut self.table_state);

        // Scrollbar
        if n > 0 {
            let pos = self.table_state.selected().unwrap_or(0);
            let mut sb_state = ScrollbarState::new(n).position(pos);
            f.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(None)
                    .end_symbol(None)
                    .track_symbol(Some("│"))
                    .thumb_symbol("┃"),
                area.inner(ratatui::layout::Margin {
                    vertical: 1,
                    horizontal: 0,
                }),
                &mut sb_state,
            );
        }
    }

    fn render_bottom_panel(&mut self, f: &mut Frame, area: Rect) {
        match self.bottom_tab {
            Tab::Detail => self.render_detail_tab(f, area),
            Tab::Actions => self.render_actions_tab(f, area),
        }
    }

    fn render_detail_tab(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style())
            .title(Line::from(vec![
                Span::styled(
                    " Detail ",
                    if self.bottom_tab == Tab::Detail {
                        title_style()
                    } else {
                        dim()
                    },
                ),
                Span::styled(" │ ", dim()),
                Span::styled(
                    "Actions",
                    if self.bottom_tab == Tab::Actions {
                        title_style()
                    } else {
                        dim()
                    },
                ),
                Span::raw(" "),
            ]));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let Some(cp) = self.selected_cluster() else {
            f.render_widget(
                Paragraph::new(Span::styled("No clusters — run nexus scan", dim())),
                inner,
            );
            return;
        };

        let c = &cp.cluster;
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(inner);

        // Left: metadata
        let mut meta = vec![
            Line::from(vec![
                Span::styled("key       ", dim()),
                Span::styled(c.cluster_key.clone(), Style::new().fg(FG)),
            ]),
            Line::from(vec![
                Span::styled("conf      ", dim()),
                Span::styled(
                    format!("{:.2}", c.confidence),
                    Style::new().fg(if c.confidence >= 0.6 {
                        SCORE_HIGH
                    } else {
                        SCORE_LOW
                    }),
                ),
            ]),
            Line::from(vec![
                Span::styled("canonical ", dim()),
                Span::styled(
                    c.canonical_clone_id.clone().unwrap_or_else(|| "—".into()),
                    Style::new().fg(FG_MUTED),
                ),
            ]),
            Line::from(vec![
                Span::styled("remote    ", dim()),
                Span::styled(
                    c.canonical_remote_id.clone().unwrap_or_else(|| "—".into()),
                    Style::new().fg(FG_MUTED),
                ),
            ]),
            Line::from(vec![
                Span::styled("members   ", dim()),
                Span::raw(format!(
                    "{} clone(s), {} remote(s)",
                    c.members
                        .iter()
                        .filter(|m| m.kind == MemberKind::Clone)
                        .count(),
                    c.members
                        .iter()
                        .filter(|m| m.kind == MemberKind::Remote)
                        .count(),
                )),
            ]),
            Line::from(vec![
                Span::styled("evidence  ", dim()),
                Span::raw(format!("{} items", c.evidence.len())),
                Span::styled("  (press e)", dim()),
            ]),
        ];

        // Top evidence hints (first 3)
        if !c.evidence.is_empty() {
            meta.push(Line::from(""));
            for e in c.evidence.iter().take(3) {
                let delta_color = if e.score_delta > 0.0 {
                    SCORE_HIGH
                } else if e.score_delta < 0.0 {
                    SCORE_LOW
                } else {
                    FG_DIM
                };
                meta.push(Line::from(vec![
                    Span::styled(
                        format!("{:+5.0} ", e.score_delta),
                        Style::new().fg(delta_color),
                    ),
                    Span::styled(truncate(&e.kind, 28), Style::new().fg(ACCENT_DIM)),
                ]));
            }
            if c.evidence.len() > 3 {
                meta.push(Line::from(Span::styled(
                    format!("      … {} more", c.evidence.len() - 3),
                    dim(),
                )));
            }
        }

        f.render_widget(Paragraph::new(meta), cols[0]);

        // Right: score bars
        let bar_w = (cols[1].width as usize).saturating_sub(14).min(20);
        let scores = vec![
            ("Canonical  ", c.scores.canonical),
            ("Health     ", c.scores.usability),
            ("Recover    ", c.scores.recoverability),
            ("Publish    ", c.scores.oss_readiness),
            ("Risk       ", c.scores.risk),
        ];
        let mut score_lines: Vec<Line> = Vec::new();
        for (label, val) in &scores {
            let mut spans = vec![Span::styled(*label, dim())];
            spans.extend(score_bar(*val, bar_w));
            score_lines.push(Line::from(spans));
        }
        f.render_widget(Paragraph::new(score_lines), cols[1]);
    }

    fn render_actions_tab(&mut self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style())
            .title(Line::from(vec![
                Span::styled(
                    "Detail",
                    if self.bottom_tab == Tab::Detail {
                        title_style()
                    } else {
                        dim()
                    },
                ),
                Span::styled(" │ ", dim()),
                Span::styled(
                    " Actions ",
                    if self.bottom_tab == Tab::Actions {
                        title_style()
                    } else {
                        dim()
                    },
                ),
                Span::raw(" "),
            ]));

        let Some(cp) = self.selected_cluster() else {
            f.render_widget(
                Paragraph::new(Span::styled("No cluster selected", dim())).block(block),
                area,
            );
            return;
        };

        if cp.actions.is_empty() {
            f.render_widget(
                Paragraph::new(Span::styled("  No actions for this cluster", dim())).block(block),
                area,
            );
            return;
        }

        let items: Vec<ListItem> = cp
            .actions
            .iter()
            .map(|a| {
                let pri_str = match a.priority {
                    Priority::High => "HIGH",
                    Priority::Medium => " MED",
                    Priority::Low => " LOW",
                };
                let mut spans = vec![
                    Span::styled(format!(" {pri_str} "), priority_style(&a.priority)),
                    Span::styled("  ", Style::new()),
                    Span::styled(
                        format!("{:<18}", action_type_label(&a.action_type)),
                        Style::new().fg(FG),
                    ),
                    Span::styled(truncate(&a.reason, 60), muted()),
                ];
                if let Some(conf) = a.confidence {
                    spans.push(Span::styled(format!("  [{:.0}%]", conf * 100.0), dim()));
                }
                ListItem::new(Line::from(spans))
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::new().bg(HIGHLIGHT_BG).add_modifier(Modifier::BOLD))
            .highlight_symbol("▸ ");
        f.render_stateful_widget(list, area, &mut self.action_list_state);
    }

    // ── Overlays ─────────────────────────────────────────────────────────

    fn render_evidence_overlay(&mut self, f: &mut Frame, area: Rect) {
        let popup = centered_pct(area, 90, 85);
        f.render_widget(Clear, popup);

        let cluster_label = self
            .selected_cluster()
            .map(|cp| cp.cluster.label.as_str())
            .unwrap_or("?");

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(ACCENT))
            .title(Line::from(vec![
                Span::styled(format!(" Evidence — {} ", cluster_label), title_style()),
                Span::styled("  Esc close  ↑↓ scroll ", dim()),
            ]));
        let inner = block.inner(popup);
        f.render_widget(block, popup);

        let items: Vec<ListItem> = self
            .evidence_lines
            .iter()
            .map(|e| {
                let delta_color = if e.delta > 0.0 {
                    SCORE_HIGH
                } else if e.delta < 0.0 {
                    SCORE_LOW
                } else {
                    FG_DIM
                };
                let spans = vec![
                    Span::styled(format!("{:+6.1}", e.delta), Style::new().fg(delta_color)),
                    Span::styled("  ", Style::new()),
                    Span::styled(
                        format!("{:<30}", truncate(&e.kind, 30)),
                        Style::new().fg(ACCENT_DIM),
                    ),
                    Span::styled(truncate(&e.subject, 20), dim()),
                    Span::styled("  ", Style::new()),
                    Span::styled(truncate(&e.detail, 80), Style::new().fg(FG_MUTED)),
                ];
                ListItem::new(Line::from(spans))
            })
            .collect();

        let n = items.len();
        let list = List::new(items)
            .highlight_style(Style::new().bg(HIGHLIGHT_BG).add_modifier(Modifier::BOLD))
            .highlight_symbol("▸ ");
        f.render_stateful_widget(list, inner, &mut self.evidence_list_state);

        if n > 0 {
            let pos = self.evidence_list_state.selected().unwrap_or(0);
            let mut sb = ScrollbarState::new(n).position(pos);
            f.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(None)
                    .end_symbol(None),
                inner,
                &mut sb,
            );
        }
    }

    fn render_help_overlay(&self, f: &mut Frame, area: Rect) {
        let popup = centered_pct(area, 60, 70);
        f.render_widget(Clear, popup);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(ACCENT))
            .title(Span::styled(" Keyboard Reference ", title_style()));
        let inner = block.inner(popup);
        f.render_widget(block, popup);

        let bindings: Vec<(&str, &str)> = vec![
            ("q / Ctrl-c", "Quit"),
            ("j / ↓", "Move down"),
            ("k / ↑", "Move up"),
            ("g", "Jump to top"),
            ("G", "Jump to bottom"),
            ("PgUp / PgDn", "Page up / down"),
            ("s", "Cycle sort mode"),
            ("/", "Edit filter (Enter apply, Esc cancel)"),
            ("f", "Clear filter"),
            ("Tab", "Toggle Detail ↔ Actions panel"),
            ("e", "Open evidence overlay"),
            ("a", "Switch to Actions panel"),
            ("p", "Show canonical pin TOML snippet"),
            ("o", "Export plan JSON to file"),
            ("?", "This help"),
            ("Esc", "Close overlay / cancel"),
        ];

        let mut lines = vec![
            Line::from(Span::styled(
                "Nexus TUI — read-only cluster inspector",
                Style::new().fg(ACCENT).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ];
        for (key, desc) in &bindings {
            lines.push(Line::from(vec![
                Span::styled(format!("  {:<14}", key), key_style()),
                Span::styled(*desc, Style::new().fg(FG)),
            ]));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Not a dashboard. No charts, no services, no mutation.",
            dim(),
        )));
        f.render_widget(Paragraph::new(lines), inner);
    }

    fn render_pin_overlay(&self, f: &mut Frame, area: Rect) {
        let popup = centered(area, 68, 11);
        f.render_widget(Clear, popup);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(ACCENT))
            .title(Span::styled(" Pin Canonical Clone ", title_style()));
        let inner = block.inner(popup);
        f.render_widget(block, popup);

        let Some(cp) = self.selected_cluster() else {
            f.render_widget(
                Paragraph::new(Span::styled("No cluster selected.", dim())),
                inner,
            );
            return;
        };
        let Some(cid) = &cp.cluster.canonical_clone_id else {
            f.render_widget(
                Paragraph::new(Span::styled(
                    "This cluster has no canonical clone id.",
                    dim(),
                )),
                inner,
            );
            return;
        };

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled("Add under [planner] in nexus.toml:", muted())),
            Line::from(""),
            Line::from(Span::styled(
                format!("  canonical_pins = [\"{cid}\"]"),
                Style::new().fg(Color::Green).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Merge with existing array if already pinning clones.",
                dim(),
            )),
            Line::from(""),
            Line::from(Span::styled("Press any key to dismiss", dim())),
        ];
        f.render_widget(Paragraph::new(lines), inner);
    }

    // ── Input ────────────────────────────────────────────────────────────

    fn handle_key(&mut self, code: KeyCode, mods: KeyModifiers) -> bool {
        self.status_msg.clear();

        // Ctrl-c always quits
        if code == KeyCode::Char('c') && mods.contains(KeyModifiers::CONTROL) {
            return true;
        }

        // Overlays
        match self.overlay {
            Overlay::Help => {
                if matches!(code, KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?')) {
                    self.overlay = Overlay::None;
                }
                return false;
            }
            Overlay::PinHint => {
                self.overlay = Overlay::None;
                return false;
            }
            Overlay::Evidence => {
                match code {
                    KeyCode::Esc | KeyCode::Char('e') => self.overlay = Overlay::None,
                    KeyCode::Char('j') | KeyCode::Down => {
                        self.evidence_list_state.select_next();
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        self.evidence_list_state.select_previous();
                    }
                    KeyCode::Char('g') => {
                        if !self.evidence_lines.is_empty() {
                            self.evidence_list_state.select(Some(0));
                        }
                    }
                    KeyCode::Char('G') => {
                        if !self.evidence_lines.is_empty() {
                            self.evidence_list_state
                                .select(Some(self.evidence_lines.len() - 1));
                        }
                    }
                    _ => {}
                }
                return false;
            }
            Overlay::None => {}
        }

        // Filter editing mode
        if self.filter_editing {
            match code {
                KeyCode::Enter => {
                    self.filter_applied = self.filter_buffer.clone();
                    self.filter_editing = false;
                    self.rebuild_ordered();
                }
                KeyCode::Esc => {
                    self.filter_buffer = self.filter_applied.clone();
                    self.filter_editing = false;
                }
                KeyCode::Backspace => {
                    self.filter_buffer.pop();
                }
                KeyCode::Char(c) => {
                    self.filter_buffer.push(c);
                }
                _ => {}
            }
            return false;
        }

        // Main cluster view
        match code {
            KeyCode::Char('q') | KeyCode::Esc => return true,
            KeyCode::Char('?') => self.overlay = Overlay::Help,
            KeyCode::Char('s') => {
                self.sort = self.sort.next();
                self.rebuild_ordered();
                self.status_msg = format!("Sort: {}", self.sort.label());
            }
            KeyCode::Char('/') => {
                self.filter_editing = true;
                self.filter_buffer = self.filter_applied.clone();
            }
            KeyCode::Char('f') => {
                if !self.filter_applied.is_empty() {
                    self.filter_applied.clear();
                    self.rebuild_ordered();
                    self.status_msg = "Filter cleared".into();
                }
            }
            KeyCode::Tab => {
                self.bottom_tab = self.bottom_tab.toggle();
                self.action_list_state.select(Some(0));
            }
            KeyCode::Char('a') => {
                self.bottom_tab = Tab::Actions;
                self.action_list_state.select(Some(0));
            }
            KeyCode::Char('e') => self.open_evidence(),
            KeyCode::Char('p') => self.overlay = Overlay::PinHint,
            KeyCode::Char('o') => self.export_plan(),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_selection(-1),
            KeyCode::Char('g') => {
                if !self.ordered.is_empty() {
                    self.table_state.select(Some(0));
                }
            }
            KeyCode::Char('G') => {
                if !self.ordered.is_empty() {
                    self.table_state.select(Some(self.ordered.len() - 1));
                }
            }
            KeyCode::PageDown => self.move_selection(10),
            KeyCode::PageUp => self.move_selection(-10),
            _ => {}
        }
        false
    }

    fn move_selection(&mut self, delta: i32) {
        if self.ordered.is_empty() {
            return;
        }
        let cur = self.table_state.selected().unwrap_or(0) as i32;
        let max = (self.ordered.len() as i32) - 1;
        let next = (cur + delta).clamp(0, max) as usize;
        self.table_state.select(Some(next));
    }

    fn open_evidence(&mut self) {
        if let Some(cp) = self.selected_cluster().cloned() {
            self.evidence_lines = cp
                .cluster
                .evidence
                .iter()
                .map(|e| EvidenceLine {
                    kind: e.kind.clone(),
                    delta: e.score_delta,
                    subject: e.subject_id.clone(),
                    detail: e.detail.clone(),
                })
                .collect();
            self.evidence_list_state.select(Some(0));
            self.overlay = Overlay::Evidence;
        }
    }

    fn export_plan(&mut self) {
        match serde_json::to_string_pretty(&self.plan) {
            Ok(json) => match std::fs::write("nexus-plan-tui-export.json", json) {
                Ok(()) => {
                    self.status_msg = "Exported to nexus-plan-tui-export.json".into();
                }
                Err(e) => {
                    self.status_msg = format!("Export failed: {e}");
                }
            },
            Err(e) => {
                self.status_msg = format!("Serialize failed: {e}");
            }
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn status_rank(s: &ClusterStatus) -> u8 {
    match s {
        ClusterStatus::Ambiguous => 0,
        ClusterStatus::ManualReview => 1,
        ClusterStatus::Resolved => 2,
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

fn centered(area: Rect, w: u16, h: u16) -> Rect {
    let w = w.min(area.width);
    let h = h.min(area.height);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect::new(x, y, w, h)
}

fn centered_pct(area: Rect, w_pct: u16, h_pct: u16) -> Rect {
    let w = (area.width as u32 * w_pct.min(100) as u32 / 100) as u16;
    let h = (area.height as u32 * h_pct.min(100) as u32 / 100) as u16;
    centered(area, w, h)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_core::{ClusterRecord, ClusterStatus, EvidenceItem, MemberKind, ScoreBundle};

    fn dummy_cluster_plan(
        label: &str,
        canonical: f64,
        risk: f64,
        status: ClusterStatus,
    ) -> ClusterPlan {
        ClusterPlan {
            cluster: ClusterRecord {
                id: "c1".into(),
                cluster_key: format!("name:{label}"),
                label: label.into(),
                status,
                confidence: 0.9,
                canonical_clone_id: Some("clone-1".into()),
                canonical_remote_id: None,
                members: vec![],
                evidence: vec![],
                scores: ScoreBundle {
                    canonical,
                    usability: 50.0,
                    recoverability: 50.0,
                    oss_readiness: 50.0,
                    risk,
                },
            },
            actions: vec![],
        }
    }

    fn dummy_plan(clusters: Vec<ClusterPlan>) -> PlanDocument {
        PlanDocument {
            schema_version: 1,
            scoring_rules_version: 5,
            generated_at: chrono::Utc::now(),
            generated_by: "test".into(),
            clusters,
        }
    }

    #[test]
    fn sort_risk_desc_ordering() {
        let plan = dummy_plan(vec![
            dummy_cluster_plan("low", 80.0, 10.0, ClusterStatus::Resolved),
            dummy_cluster_plan("high", 80.0, 90.0, ClusterStatus::Resolved),
        ]);
        let mut app = App::new(
            plan,
            TuiConfig {
                config_pins: HashSet::new(),
            },
        );
        app.sort = SortKey::RiskDesc;
        app.rebuild_ordered();
        assert_eq!(app.ordered, vec![1, 0]);
    }

    #[test]
    fn sort_health_desc_ordering() {
        let mut a = dummy_cluster_plan("low-health", 80.0, 10.0, ClusterStatus::Resolved);
        a.cluster.scores.usability = 20.0;
        let mut b = dummy_cluster_plan("high-health", 80.0, 10.0, ClusterStatus::Resolved);
        b.cluster.scores.usability = 90.0;
        let plan = dummy_plan(vec![a, b]);
        let mut app = App::new(
            plan,
            TuiConfig {
                config_pins: HashSet::new(),
            },
        );
        app.sort = SortKey::HealthDesc;
        app.rebuild_ordered();
        assert_eq!(app.ordered, vec![1, 0]);
    }

    #[test]
    fn filter_narrows_results() {
        let plan = dummy_plan(vec![
            dummy_cluster_plan("alpha", 50.0, 10.0, ClusterStatus::Resolved),
            dummy_cluster_plan("beta", 50.0, 10.0, ClusterStatus::Resolved),
            dummy_cluster_plan("gamma", 50.0, 10.0, ClusterStatus::Resolved),
        ]);
        let mut app = App::new(
            plan,
            TuiConfig {
                config_pins: HashSet::new(),
            },
        );
        app.filter_applied = "beta".into();
        app.rebuild_ordered();
        assert_eq!(app.ordered.len(), 1);
    }

    #[test]
    fn move_selection_clamps() {
        let plan = dummy_plan(vec![
            dummy_cluster_plan("a", 50.0, 10.0, ClusterStatus::Resolved),
            dummy_cluster_plan("b", 50.0, 10.0, ClusterStatus::Resolved),
        ]);
        let mut app = App::new(
            plan,
            TuiConfig {
                config_pins: HashSet::new(),
            },
        );
        app.move_selection(100);
        assert_eq!(app.table_state.selected(), Some(1));
        app.move_selection(-100);
        assert_eq!(app.table_state.selected(), Some(0));
    }

    #[test]
    fn evidence_lines_populated_on_open() {
        let mut cp = dummy_cluster_plan("test", 80.0, 10.0, ClusterStatus::Resolved);
        cp.cluster.evidence.push(EvidenceItem {
            id: "ev-1".into(),
            subject_kind: MemberKind::Clone,
            subject_id: "clone-1".into(),
            kind: "test_signal".into(),
            score_delta: 5.0,
            detail: "some detail".into(),
        });
        let plan = dummy_plan(vec![cp]);
        let mut app = App::new(
            plan,
            TuiConfig {
                config_pins: HashSet::new(),
            },
        );
        app.open_evidence();
        assert_eq!(app.evidence_lines.len(), 1);
        assert_eq!(app.evidence_lines[0].kind, "test_signal");
        assert_eq!(app.overlay, Overlay::Evidence);
    }

    #[test]
    fn total_actions_counts_across_clusters() {
        use nexus_core::PlanAction;
        let mut cp = dummy_cluster_plan("test", 80.0, 10.0, ClusterStatus::Resolved);
        cp.actions.push(PlanAction {
            id: "a1".into(),
            priority: Priority::Medium,
            action_type: ActionType::AddLicense,
            target_kind: MemberKind::Clone,
            target_id: "clone-1".into(),
            reason: "test".into(),
            commands: vec![],
            evidence_summary: None,
            confidence: None,
            risk_note: None,
        });
        let plan = dummy_plan(vec![cp]);
        let app = App::new(
            plan,
            TuiConfig {
                config_pins: HashSet::new(),
            },
        );
        assert_eq!(app.total_actions(), 1);
    }
}
