use serde::{Deserialize, Serialize};
use sqlx::{AssertSqlSafe, PgPool};
use uuid::Uuid;
use crate::state::{AppError, AppState};
use crate::strategy_indicator::StrategyIndicator;
use crate::indicator::Indicator;
use crate::market_data::{get_history};
use crate::filter_request::FilterRequest;
use serde_json::Value;
use utoipa::ToSchema;





const COLS: &str = "id, user_id, name, description, entry_mode, exit_mode, \
    entry_threshold::float8 AS entry_threshold, exit_threshold::float8 AS exit_threshold, \
    initial_capital::float8 AS initial_capital, max_position_pct::float8 AS max_position_pct, \
    risk_per_trade_pct::float8 AS risk_per_trade_pct, stop_loss_pct::float8 AS stop_loss_pct, \
    take_profit_pct::float8 AS take_profit_pct, trailing_stop_pct::float8 AS trailing_stop_pct, \
    max_open_positions, max_drawdown_pct::float8 AS max_drawdown_pct, public";



#[derive(Debug, Clone, Serialize, sqlx::FromRow, ToSchema)]
pub struct Strategy {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub entry_mode: String,
    pub exit_mode: String,
    pub entry_threshold: Option<f64>,
    pub exit_threshold: Option<f64>,
    pub initial_capital: f64,
    pub max_position_pct: f64,
    pub risk_per_trade_pct: f64,
    pub stop_loss_pct: Option<f64>,
    pub take_profit_pct: Option<f64>,
    pub trailing_stop_pct: Option<f64>,
    pub max_open_positions: i32,
    pub max_drawdown_pct: Option<f64>,
    pub public: bool
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct StrategySettings {
    pub description: Option<String>,
    pub entry_mode: Option<String>,
    pub exit_mode: Option<String>,
    pub entry_threshold: Option<f64>,
    pub exit_threshold: Option<f64>,
    pub initial_capital: Option<f64>,
    pub max_position_pct: Option<f64>,
    pub risk_per_trade_pct: Option<f64>,
    pub stop_loss_pct: Option<f64>,
    pub take_profit_pct: Option<f64>,
    pub trailing_stop_pct: Option<f64>,
    pub max_open_positions: Option<i32>,
    pub max_drawdown_pct: Option<f64>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct NewStrategy {
    pub user_id: Uuid, 
    pub name: String,
    #[serde(flatten)]
    pub settings: StrategySettings,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateStrategy {
    pub name: Option<String>,
    #[serde(flatten)]
    pub settings: StrategySettings,
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct TestParams {
    #[schema(example = "1y")]
    pub time_frame: Option<String>, 

    #[schema(example = "1d")]
    pub bar_size: Option<String>, 
}

#[derive(Debug, Serialize, ToSchema)]
pub struct Trade {
    pub symbol: String,
    pub entry_price: f64,
    pub exit_price: f64,
    pub shares: f64,
    pub pnl: f64,
    pub reason: &'static str,
}

#[derive(Clone)]
struct Pos {
    shares: f64, 
    entry: f64,  
    peak: f64,   
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TestResult {
    pub initial_capital: f64,
    pub final_equity: f64,
    pub total_return_pct: f64,
    pub max_drawdown_pct: f64,
    pub trade_count: usize,
    pub win_rate_pct: f64,
    pub equity_curve: Vec<f64>,
    pub trades: Vec<Trade>,
}

struct Loaded {
    role: String,     
    weight: f64,
    signal: Vec<bool>, 
}

impl Strategy {
    pub async fn create(db: &PgPool, n: &NewStrategy) -> Result<Self, sqlx::Error> {
        let s = &n.settings;
        sqlx::query_as::<_, Strategy>(AssertSqlSafe(format!(
            "INSERT INTO strategies (
                user_id, name, description, entry_mode, exit_mode,
                entry_threshold, exit_threshold, initial_capital, max_position_pct,
                risk_per_trade_pct, stop_loss_pct, take_profit_pct, trailing_stop_pct,
                max_open_positions, max_drawdown_pct)
             VALUES (
                $1, $2, $3, COALESCE($4, 'all'), COALESCE($5, 'any'),
                $6::numeric, $7::numeric, COALESCE($8::numeric, 10000), COALESCE($9::numeric, 10),
                COALESCE($10::numeric, 1), $11::numeric, $12::numeric, $13::numeric,
                COALESCE($14, 10), $15::numeric)
             RETURNING {COLS}"
        )))
        .bind(n.user_id)
        .bind(&n.name)
        .bind(&s.description)
        .bind(&s.entry_mode)
        .bind(&s.exit_mode)
        .bind(s.entry_threshold)
        .bind(s.exit_threshold)
        .bind(s.initial_capital)
        .bind(s.max_position_pct)
        .bind(s.risk_per_trade_pct)
        .bind(s.stop_loss_pct)
        .bind(s.take_profit_pct)
        .bind(s.trailing_stop_pct)
        .bind(s.max_open_positions)
        .bind(s.max_drawdown_pct)
        .fetch_one(db)
        .await
    }

    pub async fn get_all(db: &PgPool, f_req: FilterRequest) -> Result<Vec<Self>, sqlx::Error> {
        let (pattern, limit, offset) = f_req.resolve();

        sqlx::query_as::<_, Strategy>(AssertSqlSafe(format!(
            "SELECT {COLS} FROM strategies
            WHERE public = false AND ($1::text IS NULL OR name ILIKE $1)
            ORDER BY name, id
            LIMIT $2 OFFSET $3"
        )))
        .bind(pattern)
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await
    }

    pub async fn get_all_by_user_id(
        db: &PgPool,
        user_id: Uuid,
        f_req: FilterRequest,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let (pattern, limit, offset) = f_req.resolve();

        sqlx::query_as::<_, Strategy>(AssertSqlSafe(format!(
            "SELECT {COLS} FROM strategies
            WHERE user_id = $1 AND ($2::text IS NULL OR name ILIKE $2)
            ORDER BY name, id
            LIMIT $3 OFFSET $4"
        )))
        .bind(user_id)
        .bind(pattern)
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await
    }

    pub async fn get_by_id(db: &PgPool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Strategy>(AssertSqlSafe(format!("SELECT {COLS} FROM strategies WHERE id = $1")))
            .bind(id)
            .fetch_optional(db)
            .await
    }

    pub async fn update(
        db: &PgPool,
        id: Uuid,
        u: &UpdateStrategy,
    ) -> Result<Option<Self>, sqlx::Error> {
        let s = &u.settings;
        sqlx::query_as::<_, Strategy>(AssertSqlSafe(format!(
            "UPDATE strategies SET
                name               = COALESCE($2, name),
                description        = COALESCE($3, description),
                entry_mode         = COALESCE($4, entry_mode),
                exit_mode          = COALESCE($5, exit_mode),
                entry_threshold    = COALESCE($6::numeric, entry_threshold),
                exit_threshold     = COALESCE($7::numeric, exit_threshold),
                initial_capital    = COALESCE($8::numeric, initial_capital),
                max_position_pct   = COALESCE($9::numeric, max_position_pct),
                risk_per_trade_pct = COALESCE($10::numeric, risk_per_trade_pct),
                stop_loss_pct      = COALESCE($11::numeric, stop_loss_pct),
                take_profit_pct    = COALESCE($12::numeric, take_profit_pct),
                trailing_stop_pct  = COALESCE($13::numeric, trailing_stop_pct),
                max_open_positions = COALESCE($14, max_open_positions),
                max_drawdown_pct   = COALESCE($15::numeric, max_drawdown_pct),
                updated_at         = now()
             WHERE id = $1
             RETURNING {COLS}"
        )))
        .bind(id)
        .bind(&u.name)
        .bind(&s.description)
        .bind(&s.entry_mode)
        .bind(&s.exit_mode)
        .bind(s.entry_threshold)
        .bind(s.exit_threshold)
        .bind(s.initial_capital)
        .bind(s.max_position_pct)
        .bind(s.risk_per_trade_pct)
        .bind(s.stop_loss_pct)
        .bind(s.take_profit_pct)
        .bind(s.trailing_stop_pct)
        .bind(s.max_open_positions)
        .bind(s.max_drawdown_pct)
        .fetch_optional(db)
        .await
    }

    pub async fn delete(db: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
        let r = sqlx::query("DELETE FROM strategies WHERE id = $1")
            .bind(id)
            .execute(db)
            .await?;
        Ok(r.rows_affected() > 0)
    }

    pub async fn test_strategy(&self, db: &PgPool, p: &TestParams) -> Result<TestResult, AppError> {
        let db_err = |e: sqlx::Error| AppError::Internal(e.to_string());

        let symbols: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT s.symbol FROM collections c
             JOIN holdings h ON h.collection_id = c.id
             JOIN stocks s ON s.id = h.stock_id
             WHERE c.strategy_id = $1 ORDER BY s.symbol",
        )
        .bind(self.id)
        .fetch_all(db)
        .await
        .map_err(db_err)?;
        if symbols.is_empty() {
            return Err(AppError::Validation(
                "strategy has no stocks in its collections".into()
            ));
        }

        //The strategy's indicators (role / params / weight) joined with their definitions
        let links = StrategyIndicator::get_for_strategy(db, self.id).await.map_err(db_err)?;
        let mut defs = Vec::new();
        for l in &links {
            let ind = Indicator::get_by_id(db, l.indicator_id).await.map_err(db_err)?
                .ok_or(AppError::NotFound)?;
            defs.push((l, ind));
        }
        if !defs.iter().any(|(l, _)| l.role == "entry") {
            return Err(AppError::Validation("strategy needs at least one entry indicator".into()));
        }

        // Price history, aligned on the most recent common number of bars
        let time_frame = p.time_frame.as_deref().unwrap_or("1y");
        let bar_size = p.bar_size.as_deref().unwrap_or("1d");
        let mut hist = Vec::new();
        for s in &symbols {
            hist.push(get_history(s, time_frame, bar_size).await?);
        }
        let n = hist.iter().map(|h| h.close_prices.len()).min().unwrap_or(0);
        if n < 2 { return Err(AppError::NotFound); }
        let open: Vec<&[f64]> = hist.iter().map(|h| &h.open_prices[h.open_prices.len() - n..]).collect();
        let close: Vec<&[f64]> = hist.iter().map(|h| &h.close_prices[h.close_prices.len() - n..]).collect();

        // Signals per stock per indicator
        let mut sig: Vec<Vec<Loaded>> = Vec::new();
        for c in &close {
            let mut v = Vec::new();
            for (l, ind) in &defs {
                v.push(Loaded { role: l.role.clone(), weight: l.weight, signal: compute_signal(ind, &l.params, c)? });
            }
            sig.push(v);
        }

        // simulate
        let m = symbols.len();
        let mut cash = self.initial_capital;
        let mut pos: Vec<Option<Pos>> = vec![None; m];
        let mut buys: Vec<usize> = Vec::new();
        let mut sells: Vec<(usize, &'static str)> = Vec::new();
        let mut trades = Vec::new();
        let mut curve = Vec::with_capacity(n);
        let (mut peak_eq, mut max_dd, mut halted) = (cash, 0.0_f64, false);

        for t in 0..n {
            //sell
            for (i, reason) in sells.drain(..) {
                if let Some(pp) = pos[i].take() {
                    close_pos(&mut cash, &mut trades, &symbols[i], pp, open[i][t], reason);
                }
            }

            //buy
            let queued: Vec<usize> = buys.drain(..).collect();
            for i in queued {
                if pos[i].is_some() || open[i][t] <= 0.0 { continue; }
                let equity = cash + (0..m).filter_map(|j| pos[j].as_ref().map(|q| q.shares * close[j][t.saturating_sub(1)])).sum::<f64>();
                let mut value = equity * self.max_position_pct / 100.0;
                if let Some(sl) = self.stop_loss_pct.filter(|s| *s > 0.0) {
                    value = value.min(equity * self.risk_per_trade_pct / sl);
                }
                value = value.min(cash);
                if value <= 0.0 { continue; }
                cash -= value;
                pos[i] = Some(Pos { shares: value / open[i][t], entry: open[i][t], peak: open[i][t] });
            }
            //risk
            for i in 0..m {
                if let Some(pp) = pos[i].as_mut() {
                    let c = close[i][t];
                    pp.peak = pp.peak.max(c);
                    let reason = if self.stop_loss_pct.map_or(false, |s| c <= pp.entry * (1.0 - s / 100.0)) { Some("stop_loss") }
                        else if self.take_profit_pct.map_or(false, |s| c >= pp.entry * (1.0 + s / 100.0)) { Some("take_profit") }
                        else if self.trailing_stop_pct.map_or(false, |s| c <= pp.peak * (1.0 - s / 100.0)) { Some("trailing_stop") }
                        else { None };
                    if let Some(r) = reason {
                        if !sells.iter().any(|(j, _)| *j == i) { sells.push((i, r)); }
                    }
                }
            }

            let equity = cash + (0..m).filter_map(|i| pos[i].as_ref().map(|q| q.shares * close[i][t])).sum::<f64>();
            curve.push(equity);
            peak_eq = peak_eq.max(equity);
            let dd = (peak_eq - equity) / peak_eq * 100.0;
            max_dd = max_dd.max(dd);
            if !halted && self.max_drawdown_pct.map_or(false, |lim| dd >= lim) {
                halted = true; // flatten and stop trading
                for i in 0..m {
                    if pos[i].is_some() && !sells.iter().any(|(j, _)| *j == i) { sells.push((i, "max_drawdown")); }
                }
            }
            if halted || t + 1 >= n { continue; }

            // signals on this close, executed next open
            for i in 0..m {
                let by = |role: &str| sig[i].iter().filter(|x| x.role == role).collect::<Vec<_>>();
                if pos[i].is_some() {
                    if combine(&self.exit_mode, &by("exit"), t, self.exit_threshold)
                        && !sells.iter().any(|(j, _)| *j == i) {
                        sells.push((i, "exit_signal"));
                    }
                } else if !buys.contains(&i) {
                    let open_count = pos.iter().filter(|q| q.is_some()).count() + buys.len();
                    let filters = by("filter");
                    let filters_ok = filters.iter().all(|f| f.signal[t]);
                    if open_count < self.max_open_positions as usize && filters_ok
                        && combine(&self.entry_mode, &by("entry"), t, self.entry_threshold) {
                        buys.push(i);
                    }
                }
            }
        }

        // liquidate whatever is left at the final close
        for i in 0..m {
            if let Some(pp) = pos[i].take() {
                close_pos(&mut cash, &mut trades, &symbols[i], pp, close[i][n - 1], "end_of_test");
            }
        }

        let wins = trades.iter().filter(|t| t.pnl > 0.0).count();
        Ok(TestResult {
            initial_capital: self.initial_capital,
            final_equity: cash,
            total_return_pct: (cash / self.initial_capital - 1.0) * 100.0,
            max_drawdown_pct: max_dd,
            trade_count: trades.len(),
            win_rate_pct: if trades.is_empty() { 0.0 } else { wins as f64 / trades.len() as f64 * 100.0 },
            equity_curve: curve,
            trades,
        })
    }
}

fn param_f(p: &Value, k: &str, default: f64) -> f64 {
    p.get(k).and_then(|v| v.as_f64()).unwrap_or(default)
}

/// Simple moving average. None until there are `n` bars.
fn sma(c: &[f64], n: usize) -> Vec<Option<f64>> {
    (0..c.len())
        .map(|i| (n > 0 && i + 1 >= n).then(|| c[i + 1 - n..=i].iter().sum::<f64>() / n as f64))
        .collect()
}

/// RSI with Wilder smoothing. None until there are `n + 1` bars.
fn rsi(c: &[f64], n: usize) -> Vec<Option<f64>> {
    let mut out = vec![None; c.len()];
    if n == 0 || c.len() <= n { return out; }

    let (mut gain, mut loss) = (0.0, 0.0);
    for i in 1..=n {
        let d = c[i] - c[i - 1];
        if d >= 0.0 { gain += d } else { loss -= d }
    }
    let (mut avg_gain, mut avg_loss) = (gain / n as f64, loss / n as f64);
    let val = |g: f64, l: f64| if l == 0.0 { 100.0 } else { 100.0 - 100.0 / (1.0 + g / l) };

    out[n] = Some(val(avg_gain, avg_loss));
    for i in n + 1..c.len() {
        let d = c[i] - c[i - 1];
        avg_gain = (avg_gain * (n as f64 - 1.0) + d.max(0.0)) / n as f64;
        avg_loss = (avg_loss * (n as f64 - 1.0) + (-d).max(0.0)) / n as f64;
        out[i] = Some(val(avg_gain, avg_loss));
    }
    out
}

/// One bool per bar: "this indicator is firing on this bar".
/// Only closing prices are available from the market data right now.
fn compute_signal(ind: &Indicator, params: &Value, close: &[f64]) -> Result<Vec<bool>, AppError> {
    match ind.builtin_key.as_deref() {
        // fast SMA above slow SMA. params: {"fast": 10, "slow": 30}
        Some("sma_cross") => {
            let fast = sma(close, param_f(params, "fast", 10.0) as usize);
            let slow = sma(close, param_f(params, "slow", 30.0) as usize);
            Ok(fast
                .iter()
                .zip(&slow)
                .map(|(f, s)| matches!((f, s), (Some(f), Some(s)) if f > s))
                .collect())
        }

        // RSI below/above a level. params: {"period": 14, "level": 30, "direction": "below"|"above"}
        Some("rsi") => {
            let r = rsi(close, param_f(params, "period", 14.0) as usize);
            let level = param_f(params, "level", 30.0);
            let below = params.get("direction").and_then(|v| v.as_str()) != Some("above");
            Ok(r.iter()
                .map(|v| v.map_or(false, |v| if below { v < level } else { v > level }))
                .collect())
        }

        Some(other) => Err(AppError::Validation(format!("unknown builtin indicator '{other}'"))),

        // kind = 'formula': your schema stores builtin_key as NULL for these
        None => Err(AppError::Validation(format!(
            "formula indicators aren't supported by the tester yet ('{}')",
            ind.name
        ))),
    }
}

fn close_pos(cash: &mut f64, trades: &mut Vec<Trade>, sym: &str, p: Pos, price: f64, reason: &'static str) {
    *cash += p.shares * price;
    trades.push(Trade {
        symbol: sym.to_string(),
        entry_price: p.entry,
        exit_price: price,
        shares: p.shares,
        pnl: (price - p.entry) * p.shares,
        reason,
    });
}

/// Combine the indicators of one role at bar `t` according to the strategy mode.
fn combine(mode: &str, items: &[&Loaded], t: usize, threshold: Option<f64>) -> bool {
    if items.is_empty() {
        return false;
    }
    match mode {
        "all" => items.iter().all(|i| i.signal[t]),
        "any" => items.iter().any(|i| i.signal[t]),
        "majority" => {
            let fired = items.iter().filter(|i| i.signal[t]).count();
            fired * 2 > items.len()
        }
        "weighted" => {
            let total: f64 = items.iter().map(|i| i.weight).sum();
            let fired: f64 = items.iter().filter(|i| i.signal[t]).map(|i| i.weight).sum();
            total > 0.0 && fired / total >= threshold.unwrap_or(0.5)
        }
        _ => false,
    }
}


//deepseek slop

// strategy.rs

/// Modes understood by `combine`. Keep in sync with that function.
const MODES: &[&str] = &["all", "any", "majority", "weighted"];

const MAX_NAME_LEN: usize = 255;

impl NewStrategy {
    pub fn validate(&self) -> Result<(), AppError> {
        validate_name(&self.name)?;
        self.settings.validate()
    }
}

impl UpdateStrategy {
    pub fn validate(&self) -> Result<(), AppError> {
        if let Some(n) = self.name.as_deref() {
            validate_name(n)?;
        }
        self.settings.validate()
    }
}

impl StrategySettings {
    pub fn validate(&self) -> Result<(), AppError> {
        if let Some(m) = self.entry_mode.as_deref() {
            check_mode("entry_mode", m)?;
        }
        if let Some(m) = self.exit_mode.as_deref() {
            check_mode("exit_mode", m)?;
        }

        // Percentages stored as numeric; DB CHECKs reject <= 0 or > 100.
        check_pct("max_position_pct",   self.max_position_pct)?;
        check_pct("risk_per_trade_pct", self.risk_per_trade_pct)?;
        check_pct("stop_loss_pct",      self.stop_loss_pct)?;
        check_pct("take_profit_pct",    self.take_profit_pct)?;
        check_pct("trailing_stop_pct",  self.trailing_stop_pct)?;
        check_pct("max_drawdown_pct",   self.max_drawdown_pct)?;

        // Only consumed by `weighted`; the fraction of total weight required.
        check_threshold("entry_threshold", self.entry_threshold)?;
        check_threshold("exit_threshold",  self.exit_threshold)?;

        if let Some(c) = self.initial_capital {
            if !c.is_finite() || c <= 0.0 {
                return Err(AppError::Validation(format!(
                    "initial_capital must be finite and > 0, got {c}"
                )));
            }
        }

        if let Some(n) = self.max_open_positions {
            if n < 1 {
                return Err(AppError::Validation(format!(
                    "max_open_positions must be >= 1, got {n}"
                )));
            }
        }

        

        Ok(())
    }
}

fn validate_name(n: &str) -> Result<(), AppError> {
    let t = n.trim();
    if t.is_empty() {
        return Err(AppError::Validation("name must not be empty".into()));
    }
  
    Ok(())
}

fn check_mode(field: &str, m: &str) -> Result<(), AppError> {
    if !MODES.contains(&m) {
        return Err(AppError::Validation(format!(
            "{field} must be one of {MODES:?}, got {m:?}"
        )));
    }
    Ok(())
}

fn check_pct(field: &str, v: Option<f64>) -> Result<(), AppError> {
    let Some(v) = v else { return Ok(()) };
    if !v.is_finite() || v <= 0.0 || v > 100.0 {
        return Err(AppError::Validation(format!(
            "{field} must be in (0, 100], got {v}"
        )));
    }
    Ok(())
}

fn check_threshold(field: &str, v: Option<f64>) -> Result<(), AppError> {
    let Some(v) = v else { return Ok(()) };
    if !v.is_finite() || !(0.0..=1.0).contains(&v) {
        return Err(AppError::Validation(format!(
            "{field} must be in [0, 1], got {v}"
        )));
    }
    Ok(())
}