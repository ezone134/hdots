//! Resource broker: "fetch what's rendered, never more."
//!
//! Every on-screen surface (packed dashboard card, expanded banner chip,
//! resting pill chip) declares the background resources it needs via
//! `needs()`. The broker unions those demands into an active set and only
//! fetches a resource while it has ≥1 consumer — a resource with zero
//! on-screen consumers is never fetched. Each resource is refreshed at the
//! FASTEST rate any of its consumers asked for (see `Rate`).
//!
//! The broker is pure bookkeeping + dispatch: it never owns threads or
//! channels. `App::broker_tick()` feeds it a fresh union on every services
//! beat; it answers *"which fetches are due right now?"* and the App routes
//! those into the existing per-resource kick functions.

use crate::shell::{Rate, Res, Shell};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

pub struct Broker {
    /// resources wanted by ≥1 on-screen consumer → fastest requested rate.
    active: HashMap<Res, Rate>,
    /// last successful dispatch time per resource (for TTL scheduling).
    last: HashMap<Res, Instant>,
    /// resources that just became wanted this recompute → fetch NOW (so
    /// adding a card or opening the dashboard warms it instantly).
    warm: HashSet<Res>,
    /// explicit config-driven TTL override (e.g. `weather.interval_s`).
    ttls: HashMap<Res, Duration>,
}

impl Broker {
    pub fn new() -> Self {
        Broker {
            active: HashMap::new(),
            last: HashMap::new(),
            warm: HashSet::new(),
            ttls: HashMap::new(),
        }
    }

    /// Let a per-resource config interval override the rate TTL
    /// (e.g. `[weather] interval_s`).
    pub fn set_ttl(&mut self, res: Res, ttl: Duration) {
        self.ttls.insert(res, ttl);
    }

    fn effective_ttl(&self, res: Res, rate: Rate) -> Option<Duration> {
        self.ttls.get(&res).copied().or_else(|| rate.ttl())
    }

    /// Re-union the active set from a fresh frame of on-screen consumers.
    /// Returns the resources that must be fetched immediately (either they
    /// were just turned on, or they're due on their TTL).
    pub fn recompute(&mut self, wants: &[(Res, Rate)]) -> Vec<Res> {
        let mut next: HashMap<Res, Rate> = HashMap::new();
        for &(res, rate) in wants {
            next.entry(res)
                .and_modify(|r| {
                    if rate < *r {
                        *r = rate;
                    }
                })
                .or_insert(rate);
        }

        let mut due = Vec::new();
        for (&res, &rate) in &next {
            if !self.active.contains_key(&res) {
                // brand-new consumer → warm it immediately.
                self.warm.insert(res);
            }
            if self.warm.remove(&res) {
                due.push(res);
                continue;
            }
            let Some(ttl) = self.effective_ttl(res, rate) else { continue };
            let stale = self
                .last
                .get(&res)
                .map(|t| t.elapsed() >= ttl)
                .unwrap_or(true);
            if stale {
                due.push(res);
            }
        }
        self.active = next;
        due
    }

    /// TTL-triggered fetches for resources that stay wanted across beats.
    /// `now` is the current instant; `wants` is the fresh consumer union.
    /// Called every services tick so long-lived resources refresh even if no
    /// card is newly added.
    pub fn tick(&mut self, now: Instant, wants: &[(Res, Rate)]) -> Vec<Res> {
        let mut due = self.recompute(wants);
        // recompute already warmed new consumers; now sweep the settled ones
        for (&res, &rate) in &self.active {
            if due.contains(&res) {
                continue;
            }
            let Some(ttl) = self.effective_ttl(res, rate) else { continue };
            let stale = self
                .last
                .get(&res)
                .map(|t| now.duration_since(*t) >= ttl)
                .unwrap_or(true);
            if stale {
                due.push(res);
            }
        }
        due.sort_by_key(|r| r.name());
        due.dedup();
        due
    }

    /// Record that `res` was dispatched just now (feeds the TTL scheduler).
    pub fn mark(&mut self, res: Res, now: Instant) {
        self.last.insert(res, now);
    }

    /// Is this resource currently wanted by at least one on-screen consumer?
    pub fn active(&self, res: Res) -> bool {
        self.active.contains_key(&res)
    }

    /// Requests from a `Shell`: the union over everything RENDERED right
    /// now. Only visible consumers count: the packed grid (never the parked
    /// tray — a parked card is disabled and must not fetch) gated on the
    /// dashboard being enabled, and banner chips gated on the chips toggle.
    /// Resting pills are on-screen in Collapsed. Widening this union later
    /// (e.g. to serve the parking tray) is a one-line change — and one that
    /// should be resisted: hidden ⇒ not fetched.
    pub fn shell_wants(shell: &Shell) -> Vec<(Res, Rate)> {
        use crate::shell::Mode;
        let mut v = Vec::new();
        if shell.mode == Mode::Expanded {
            if shell.dash_enabled {
                for c in &shell.packed_layout {
                    v.extend(c.card.needs().iter().copied());
                }
            }
            if shell.banner_chips_enabled {
                for tok in shell.cfg.banner_order() {
                    if let crate::shell::BannerToken::Chip(b) = tok {
                        v.extend(b.needs().iter().copied());
                    }
                }
            }
        }
        for p in &shell.pill_order {
            v.extend(p.needs().iter().copied());
        }
        v
    }
}