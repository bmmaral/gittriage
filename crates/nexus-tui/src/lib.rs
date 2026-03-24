//! Minimal read-only TUI over a pre-built [`PlanDocument`].
//! Inspection and export only — no dashboard charts, no live mutation of scores.

use anyhow::{bail, Result};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use nexus_core::{ClusterPlan, ClusterStatus, PlanDocument};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, TableState, Wrap,
};
use ratatui::{DefaultTerminal, Frame};
use std::collections::HashSet;
use std::io::{self, IsTerminal};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SortKey {
    Label,
    CanonicalDesc,
    RiskDesc,
    StatusAmbiguousFirst,
}

impl SortKey {
    fn next(self) -> Self {
        match self {
            Self::Label => Self::CanonicalDesc,
            Self::CanonicalDesc => Self::RiskDesc,
            Self::RiskDesc => Self::StatusAmbiguousFirst,
            Self::StatusAmbiguousFirst => Self::Label,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Label => "label",
            Self::CanonicalDesc => "canonical↓",
            Self::RiskDesc => "risk↓",
            Self::StatusAmbiguousFirst => "ambiguous first",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Clusters,
    Evidence,
    Help,
    PinHint,
}

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
            .draw(|f: &mut Frame| {
                app.render(f);
            })
            .map_err(|e| anyhow::anyhow!(e))?;
        let ev = event::read()?;
        if let Event::Key(key) = ev {
            if key.kind == KeyEventKind::Release {
                continue;
            }
            if app.handle_key(key.code) {
                break;
            }
        }
    }
    Ok(())
}

struct App {
    plan: PlanDocument,
    config: TuiConfig,
    sort: SortKey,
    filter_applied: String,
    filter_editing: bool,
    filter_buffer: String,
    ordered: Vec<usize>,
    table_state: TableState,
    screen: Screen,
    evidence_list_state: ListState,
    evidence_lines: Vec<String>,
    status: String,
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
            screen: Screen::Clusters,
            evidence_list_state: ListState::default(),
            evidence_lines: Vec::new(),
            status: String::new(),
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

        match self.sort {
            SortKey::Label => {
                v.sort_by(|&a, &b| {
                    self.plan.clusters[a]
                        .cluster
                        .label
                        .to_lowercase()
                        .cmp(&self.plan.clusters[b].cluster.label.to_lowercase())
                });
            }
            SortKey::CanonicalDesc => {
                v.sort_by(|&a, &b| {
                    self.plan.clusters[b]
                        .cluster
                        .scores
                        .canonical
                        .partial_cmp(&self.plan.clusters[a].cluster.scores.canonical)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            SortKey::RiskDesc => {
                v.sort_by(|&a, &b| {
                    self.plan.clusters[b]
                        .cluster
                        .scores
                        .risk
                        .partial_cmp(&self.plan.clusters[a].cluster.scores.risk)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            SortKey::StatusAmbiguousFirst => {
                v.sort_by(|&a, &b| {
                    status_rank(&self.plan.clusters[a].cluster.status)
                        .cmp(&status_rank(&self.plan.clusters[b].cluster.status))
                        .then_with(|| {
                            self.plan.clusters[a]
                                .cluster
                                .label
                                .to_lowercase()
                                .cmp(&self.plan.clusters[b].cluster.label.to_lowercase())
                        })
                });
            }
        }

        self.ordered = v;
        let sel = self.table_state.selected().unwrap_or(0);
        let max = self.ordered.len().saturating_sub(1);
        let new_sel = sel.min(max);
        self.table_state.select(if self.ordered.is_empty() {
            None
        } else {
            Some(new_sel)
        });
    }

    fn selected_cluster(&self) -> Option<&ClusterPlan> {
        let i = self.table_state.selected()?;
        let idx = *self.ordered.get(i)?;
        self.plan.clusters.get(idx)
    }

    fn render(&mut self, f: &mut Frame) {
        match self.screen {
            Screen::Clusters => self.render_clusters(f),
            Screen::Evidence => self.render_evidence(f),
            Screen::Help => self.render_help(f),
            Screen::PinHint => self.render_pin(f),
        }
    }

    fn render_clusters(&mut self, f: &mut Frame) {
        let area = f.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(8),
                Constraint::Length(10),
                Constraint::Length(2),
            ])
            .split(area);

        let header_cells = ["Label", "Canon", "Health", "Recov", "Pub", "Risk", "Status"]
            .into_iter()
            .map(|h| Cell::from(h).style(Style::default().add_modifier(Modifier::BOLD)));
        let header = Row::new(header_cells).height(1).bottom_margin(0);

        let rows: Vec<Row> = self
            .ordered
            .iter()
            .map(|&idx| {
                let cp = &self.plan.clusters[idx];
                let c = &cp.cluster;
                let st = match c.status {
                    ClusterStatus::Resolved => "OK",
                    ClusterStatus::Ambiguous => "AMB",
                    ClusterStatus::ManualReview => "REV",
                };
                let pin = c
                    .canonical_clone_id
                    .as_ref()
                    .map(|id| self.config.config_pins.contains(id))
                    .unwrap_or(false);
                let label = if pin {
                    format!("* {}", truncate(&c.label, 22))
                } else {
                    truncate(&c.label, 24)
                };
                Row::new(vec![
                    Cell::from(label),
                    Cell::from(format!("{:.0}", c.scores.canonical)),
                    Cell::from(format!("{:.0}", c.scores.usability)),
                    Cell::from(format!("{:.0}", c.scores.recoverability)),
                    Cell::from(format!("{:.0}", c.scores.oss_readiness)),
                    Cell::from(format!("{:.0}", c.scores.risk)),
                    Cell::from(st),
                ])
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Percentage(28),
                Constraint::Length(5),
                Constraint::Length(6),
                Constraint::Length(6),
                Constraint::Length(5),
                Constraint::Length(5),
                Constraint::Length(6),
            ],
        )
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(format!(
            " Nexus clusters — sort:{}  n:{}  rules v{} ",
            self.sort.as_str(),
            self.ordered.len(),
            self.plan.scoring_rules_version
        )))
        .row_highlight_style(Style::new().reversed())
        .highlight_symbol("> ");

        f.render_stateful_widget(table, chunks[0], &mut self.table_state);

        let detail_block = Block::default().borders(Borders::ALL).title(" Selection ");
        let inner = detail_block.inner(chunks[1]);
        f.render_widget(detail_block, chunks[1]);

        let detail_text = self.detail_paragraph();
        let p = Paragraph::new(detail_text).wrap(Wrap { trim: true });
        f.render_widget(p, inner);

        let filter_line = if self.filter_editing {
            format!("[filter edit] {}▏", self.filter_buffer)
        } else {
            format!(
                "filter: \"{}\"  |  q quit  s sort  / edit filter  f clear  e evidence  ? help  p pin TOML  o export JSON",
                if self.filter_applied.is_empty() {
                    "(none)"
                } else {
                    self.filter_applied.as_str()
                }
            )
        };
        let footer = Paragraph::new(Line::from(vec![
            Span::styled(&self.status, Style::new().fg(Color::Yellow)),
            Span::raw(" "),
            Span::raw(filter_line),
        ]))
        .block(Block::default().borders(Borders::ALL).title(" Keys "));
        f.render_widget(footer, chunks[2]);
    }

    fn detail_paragraph(&self) -> Vec<Line<'static>> {
        let Some(cp) = self.selected_cluster() else {
            return vec![Line::from("(no clusters — run nexus scan)")];
        };
        let c = &cp.cluster;
        let mut lines = vec![
            Line::from(vec![
                Span::styled("key: ", Style::default().add_modifier(Modifier::DIM)),
                Span::raw(c.cluster_key.clone()),
            ]),
            Line::from(vec![
                Span::styled("confidence: ", Style::default().add_modifier(Modifier::DIM)),
                Span::raw(format!("{:.2}", c.confidence)),
            ]),
            Line::from(vec![
                Span::styled(
                    "canonical clone: ",
                    Style::default().add_modifier(Modifier::DIM),
                ),
                Span::raw(c.canonical_clone_id.clone().unwrap_or_else(|| "—".into())),
            ]),
            Line::from(vec![
                Span::styled("actions: ", Style::default().add_modifier(Modifier::DIM)),
                Span::raw(format!("{}", cp.actions.len())),
            ]),
        ];
        for e in c.evidence.iter().take(4) {
            let t = truncate(
                &format!("{}  {:+.0}  {}", e.kind, e.score_delta, e.detail),
                120,
            );
            lines.push(Line::from(Span::raw(t)));
        }
        if c.evidence.len() > 4 {
            lines.push(Line::from(Span::styled(
                format!("… {} more (press e)", c.evidence.len() - 4),
                Style::default().add_modifier(Modifier::DIM),
            )));
        }
        lines
    }

    fn render_evidence(&mut self, f: &mut Frame) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Evidence — Esc back ");
        let items: Vec<ListItem> = self
            .evidence_lines
            .iter()
            .map(|s| ListItem::new(Line::from(s.as_str())))
            .collect();
        let list = List::new(items)
            .block(block)
            .highlight_style(Style::new().reversed());
        f.render_stateful_widget(list, f.area(), &mut self.evidence_list_state);
    }

    fn render_help(&self, f: &mut Frame) {
        let text = vec![
            Line::from("Nexus TUI — local inspection over the same planner as score/plan/report."),
            Line::from(""),
            Line::from("q        Quit"),
            Line::from("j / ↓    Down"),
            Line::from("k / ↑    Up"),
            Line::from("s        Cycle sort (label, canonical, risk, ambiguous-first)"),
            Line::from(
                "/        Edit filter substring (label + cluster_key); Enter apply, Esc cancel",
            ),
            Line::from("f        Clear filter"),
            Line::from("e        Full evidence list for selection"),
            Line::from("p        Show nexus.toml snippet to pin canonical clone"),
            Line::from("o        Write full plan JSON to ./nexus-plan-tui-export.json"),
            Line::from("?        This help (Esc closes)"),
            Line::from(""),
            Line::from("This is not a dashboard: no charts, no background services."),
        ];
        let p = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title(" Help — Esc "));
        f.render_widget(p, f.area());
    }

    fn render_pin(&self, f: &mut Frame) {
        let Some(cp) = self.selected_cluster() else {
            let p = Paragraph::new("No cluster selected.").block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Pin — any key "),
            );
            f.render_widget(p, centered(f.area(), 50, 8));
            return;
        };
        let Some(cid) = &cp.cluster.canonical_clone_id else {
            let p = Paragraph::new("No canonical clone id on this cluster.").block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Pin — any key "),
            );
            f.render_widget(p, centered(f.area(), 52, 6));
            return;
        };
        let body = format!(
            "Add under [planner] in nexus.toml:\n\ncanonical_pins = [\"{cid}\"]\n\n(merge with existing array if you already pin clones)"
        );
        let p = Paragraph::new(body).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Pin hint — any key dismiss "),
        );
        f.render_widget(p, centered(f.area(), 64, 9));
    }

    /// Returns true if should quit.
    fn handle_key(&mut self, code: KeyCode) -> bool {
        self.status.clear();
        match self.screen {
            Screen::Help => {
                if matches!(code, KeyCode::Esc | KeyCode::Char('q')) {
                    self.screen = Screen::Clusters;
                }
                return false;
            }
            Screen::PinHint => {
                self.screen = Screen::Clusters;
                return false;
            }
            Screen::Evidence => {
                match code {
                    KeyCode::Esc => self.screen = Screen::Clusters,
                    KeyCode::Char('j') | KeyCode::Down => {
                        self.evidence_list_state.select_next();
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        self.evidence_list_state.select_previous();
                    }
                    _ => {}
                }
                return false;
            }
            Screen::Clusters => {}
        }

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

        match code {
            KeyCode::Char('q') => return true,
            KeyCode::Char('?') => self.screen = Screen::Help,
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.sort = self.sort.next();
                self.rebuild_ordered();
            }
            KeyCode::Char('/') => {
                self.filter_editing = true;
                self.filter_buffer = self.filter_applied.clone();
            }
            KeyCode::Char('f') | KeyCode::Char('F') => {
                self.filter_applied.clear();
                self.rebuild_ordered();
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                if let Some(cp) = self.selected_cluster().cloned() {
                    self.evidence_lines = cp
                        .cluster
                        .evidence
                        .iter()
                        .map(|e| {
                            format!(
                                "{}  {:+.1}  {}  {}",
                                e.kind, e.score_delta, e.subject_id, e.detail
                            )
                        })
                        .collect();
                    self.evidence_list_state.select(Some(0));
                    self.screen = Screen::Evidence;
                }
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                self.screen = Screen::PinHint;
            }
            KeyCode::Char('o') | KeyCode::Char('O') => {
                match serde_json::to_string_pretty(&self.plan) {
                    Ok(json) => match std::fs::write("nexus-plan-tui-export.json", json) {
                        Ok(()) => {
                            self.status = "wrote nexus-plan-tui-export.json".into();
                        }
                        Err(e) => {
                            self.status = format!("export failed: {e}");
                        }
                    },
                    Err(e) => {
                        self.status = format!("serialize failed: {e}");
                    }
                }
            }
            KeyCode::Down | KeyCode::Char('j') => self.next_row(),
            KeyCode::Up | KeyCode::Char('k') => self.prev_row(),
            _ => {}
        }
        false
    }

    fn next_row(&mut self) {
        if self.ordered.is_empty() {
            return;
        }
        let i = self.table_state.selected().unwrap_or(0);
        let n = self.ordered.len();
        self.table_state.select(Some((i + 1).min(n - 1)));
    }

    fn prev_row(&mut self) {
        if self.ordered.is_empty() {
            return;
        }
        let i = self.table_state.selected().unwrap_or(0);
        self.table_state.select(Some(i.saturating_sub(1)));
    }
}

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
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i + 2 >= max {
            break;
        }
        out.push(ch);
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_core::{ClusterRecord, ClusterStatus, ScoreBundle};

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

    #[test]
    fn sort_risk_desc_ordering() {
        let plan = PlanDocument {
            schema_version: 1,
            scoring_rules_version: 4,
            generated_at: chrono::Utc::now(),
            generated_by: "test".into(),
            clusters: vec![
                dummy_cluster_plan("low", 80.0, 10.0, ClusterStatus::Resolved),
                dummy_cluster_plan("high", 80.0, 90.0, ClusterStatus::Resolved),
            ],
        };
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
}
