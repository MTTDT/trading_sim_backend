

use reqwest::{Client, Method};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

const OK: &[u16] = &[200, 201];
const DEL: &[u16] = &[200, 204];
const CLIENT_ERR: &[u16] = &[400, 404, 409, 422];

struct T {
    c: Client,
    base: String,
    pass: u32,
    fail: u32,
}

impl T {
    async fn call(&mut self, label: &str, m: Method, path: &str, body: Option<Value>, expect: &[u16]) -> Value {
        let mut rb = self.c.request(m.clone(), format!("{}{}", self.base, path));
        if let Some(b) = body {
            rb = rb.json(&b);
        }
        let resp = match rb.send().await {
            Ok(r) => r,
            Err(e) => {
                self.fail += 1;
                println!("FAIL  {label}\n        {m} {path} -> request error: {e}");
                return Value::Null;
            }
        };
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        let json: Value = serde_json::from_str(&text).unwrap_or(Value::String(text));
        if expect.contains(&status) {
            self.pass += 1;
            println!("ok    {label}  [{status}]");
        } else {
            self.fail += 1;
            let mut shown = json.to_string();
            shown.truncate(300);
            println!("FAIL  {label}\n        {m} {path} -> {status}, expected one of {expect:?}\n        body: {shown}");
        }
        json
    }

    fn check(&mut self, label: &str, cond: bool) {
        if cond {
            self.pass += 1;
            println!("ok    {label}");
        } else {
            self.fail += 1;
            println!("FAIL  {label}");
        }
    }
}

fn id(v: &Value) -> String {
    v["id"].as_str().unwrap_or("").to_string()
}

#[tokio::main]
async fn main() {
    let base = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".into());
    let db_url = std::env::var("DATABASE_URL").expect("set DATABASE_URL (see header comment)");
    let db = PgPoolOptions::new().max_connections(2).connect(&db_url).await.expect("db connect");

    // leftovers from a crashed earlier run
    let _ = sqlx::query("DELETE FROM stocks WHERE exchange = 'SMOKE'").execute(&db).await;
    let _ = sqlx::query("DELETE FROM users WHERE email LIKE 'smoke+%@test.local'").execute(&db).await;

    let mut t = T { c: Client::new(), base, pass: 0, fail: 0 };
    let nil = Uuid::nil();
    let email = format!("smoke+{}@test.local", Uuid::new_v4().simple());

    println!("\n== health ==");
    t.call("GET /health", Method::GET, "/health", None, &[200]).await;

    println!("\n== users ==");
    let u = t.call("create user", Method::POST, "/users", Some(json!({"name":"Smoke","email":email})), OK).await;
    let uid = id(&u);

    let u = t.call("create the same user again", Method::POST, "/users", Some(json!({"name":"Smoke","email":email})), CLIENT_ERR).await;


    t.call("create user, missing field", Method::POST, "/users", Some(json!({"name":"x"})), CLIENT_ERR).await;
    t.call("get user", Method::GET, &format!("/users/{uid}"), None, &[200]).await;
    t.call("get user, bad uuid", Method::GET, "/users/not-a-uuid", None, CLIENT_ERR).await;
    t.call("get user, missing", Method::GET, &format!("/users/{nil}"), None, &[404]).await;
    t.call("update user", Method::PUT, &format!("/users/{uid}"), Some(json!({"name":"Smoke 2"})), &[200]).await;
    t.call("update user, missing", Method::PUT, &format!("/users/{nil}"), Some(json!({"name":"x"})), &[404]).await;
    t.call("list users", Method::GET, "/users", None, &[200]).await;
    t.call("list users, filter", Method::GET, "/users?page=0&search=Sm", None, OK).await;


    println!("\n== stocks ==");
    let aapl = t.call("create AAPL", Method::POST, "/stocks", Some(json!({"symbol":"AAPL","name":"Apple","exchange":"SMOKE"})), OK).await;
    let msft = t.call("create MSFT", Method::POST, "/stocks", Some(json!({"symbol":"MSFT","name":"Microsoft","exchange":"SMOKE"})), OK).await;
    let (aapl_id, msft_id) = (id(&aapl), id(&msft));
    // t.call("duplicate stock is a client error", Method::POST, "/stocks", Some(json!({"symbol":"AAPL","name":"Apple","exchange":"SMOKE"})), CLIENT_ERR).await;
    t.call("get stock", Method::GET, &format!("/stocks/{aapl_id}"), None, &[200]).await;
    t.call("get stock, missing", Method::GET, &format!("/stocks/{nil}"), None, &[404]).await;
    t.call("search stocks", Method::GET, "/stocks?q=AAP", None, &[200]).await;
    t.call("update stock", Method::PUT, &format!("/stocks/{aapl_id}"), Some(json!({"name":"Apple Inc"})), &[200]).await;
    t.call("update stock, missing", Method::PUT, &format!("/stocks/{nil}"), Some(json!({"name":"x"})), &[404]).await;

    println!("\n== indicators ==");
    let ind = t.call("create formula indicator", Method::POST, "/indicators",
        Some(json!({"user_id":uid,"name":"smoke formula","formula":"close > 1"})), OK).await;
    let ind_id = id(&ind);
    t.call("create indicator, missing formula", Method::POST, "/indicators", Some(json!({"user_id":uid,"name":"x"})), CLIENT_ERR).await;
    let list = t.call("list indicators (built-ins)", Method::GET, "/indicators", None, &[200]).await;
    let builtin = |key: &str| -> String {
        list.as_array()
            .and_then(|a| a.iter().find(|i| i["builtin_key"] == key))
            .map(id)
            .unwrap_or_default()
    };
    let (sma_id, rsi_id) = (builtin("sma_cross"), builtin("rsi"));
    t.check("built-ins sma_cross and rsi are seeded", !sma_id.is_empty() && !rsi_id.is_empty());
    t.call("get indicator", Method::GET, &format!("/indicators/{ind_id}"), None, &[200]).await;
    t.call("get indicator, missing", Method::GET, &format!("/indicators/{nil}"), None, &[404]).await;
    t.call("update own indicator", Method::PUT, &format!("/indicators/{ind_id}"), Some(json!({"name":"smoke formula 2"})), &[200]).await;
    t.call("update built-in is refused", Method::PUT, &format!("/indicators/{sma_id}"), Some(json!({"name":"hack"})), &[404]).await;
    t.call("delete built-in is refused", Method::DELETE, &format!("/indicators/{sma_id}"), None, &[404]).await;

    println!("\n== strategies ==");
    t.call("create strategy, unknown user", Method::POST, "/strategies", Some(json!({"user_id":nil,"name":"x"})), CLIENT_ERR).await;
    t.call("create strategy, bad entry_mode", Method::POST, "/strategies", Some(json!({"user_id":uid,"name":"x","entry_mode":"nope"})), CLIENT_ERR).await;
    t.call("create strategy, max_position_pct 0", Method::POST, "/strategies", Some(json!({"user_id":uid,"name":"x","max_position_pct":0})), CLIENT_ERR).await;
    let s = t.call("create strategy", Method::POST, "/strategies", Some(json!({
        "user_id": uid, "name": "smoke strategy", "entry_mode": "all", "exit_mode": "any",
        "initial_capital": 10000, "max_position_pct": 50, "risk_per_trade_pct": 1,
        "stop_loss_pct": 5, "take_profit_pct": 15, "trailing_stop_pct": 8,
        "max_open_positions": 5, "max_drawdown_pct": 30
    })), OK).await;
    let sid = id(&s);
    t.check("strategy defaults/values round-trip", s["initial_capital"].as_f64() == Some(10000.0) && s["stop_loss_pct"].as_f64() == Some(5.0));
    t.call("get strategy", Method::GET, &format!("/strategies/{sid}"), None, &[200]).await;
    t.call("get strategy, missing", Method::GET, &format!("/strategies/{nil}"), None, &[404]).await;
    t.call("list strategies by user", Method::GET, &format!("/strategies?user_id={uid}"), None, &[200]).await;
    t.call("update strategy", Method::PUT, &format!("/strategies/{sid}"), Some(json!({"description":"updated"})), &[200]).await;
    t.call("update strategy, missing", Method::PUT, &format!("/strategies/{nil}"), Some(json!({"name":"x"})), &[404]).await;

    println!("\n== backtest error cases (before seeding) ==");
    t.call("test: strategy not found", Method::POST, &format!("/strategies/{nil}/test"), Some(json!({})), &[404]).await;
    t.call("test: no stocks in collections -> client error", Method::POST, &format!("/strategies/{sid}/test"), Some(json!({})), CLIENT_ERR).await;

    println!("\n== collections & holdings ==");
    t.call("collection under missing strategy", Method::POST, &format!("/strategies/{nil}/collections"), Some(json!({"name":"x"})), CLIENT_ERR).await;
    let col = t.call("create collection", Method::POST, &format!("/strategies/{sid}/collections"), Some(json!({"name":"tech"})), OK).await;
    let cid = id(&col);
    t.call("duplicate collection name", Method::POST, &format!("/strategies/{sid}/collections"), Some(json!({"name":"tech"})), CLIENT_ERR).await;
    t.call("get collection", Method::GET, &format!("/strategies/{sid}/collections/{cid}"), None, &[200]).await;
    t.call("get collection via wrong strategy", Method::GET, &format!("/strategies/{nil}/collections/{cid}"), None, &[404]).await;
    t.call("rename collection", Method::PUT, &format!("/strategies/{sid}/collections/{cid}"), Some(json!({"name":"tech 2"})), &[200]).await;
    t.call("list collections", Method::GET, &format!("/strategies/{sid}/collections"), None, &[200]).await;

    let hpath = format!("/strategies/{sid}/collections/{cid}/holdings");
    let h = t.call("add holding AAPL", Method::POST, &hpath, Some(json!({"stock_id":aapl_id,"quantity":5})), OK).await;
    let hid = id(&h);
    t.call("add holding MSFT", Method::POST, &hpath, Some(json!({"stock_id":msft_id})), OK).await;
    t.call("duplicate holding is a client error", Method::POST, &hpath, Some(json!({"stock_id":aapl_id})), CLIENT_ERR).await;
    t.call("holding with unknown stock", Method::POST, &hpath, Some(json!({"stock_id":nil})), CLIENT_ERR).await;
    t.call("holding with negative quantity", Method::POST, &hpath, Some(json!({"stock_id":aapl_id,"quantity":-1})), CLIENT_ERR).await;
    t.call("get holding", Method::GET, &format!("{hpath}/{hid}"), None, &[200]).await;
    t.call("update holding", Method::PUT, &format!("{hpath}/{hid}"), Some(json!({"quantity":10,"notes":"n"})), &[200]).await;
    t.call("get holding, missing", Method::GET, &format!("{hpath}/{nil}"), None, &[404]).await;
    t.call("list holdings", Method::GET, &hpath, None, &[200]).await;

    println!("\n== backtest error case: stocks but no indicators ==");
    t.call("test: no entry indicator -> client error", Method::POST, &format!("/strategies/{sid}/test"), Some(json!({})), CLIENT_ERR).await;

    println!("\n== market data (informational) ==");
    t.call("GET /market-data", Method::GET, "/market-data?symbol=AAPL&time_frame=1mo&bar_size=1d", None, &[200, 400, 422]).await;

    println!("\n== backtest ==");
    let (sid_u, sma_u, rsi_u) = (Uuid::parse_str(&sid).unwrap(), Uuid::parse_str(&sma_id), Uuid::parse_str(&rsi_id));
    if let (Ok(sma_u), Ok(rsi_u)) = (sma_u, rsi_u) {
        let q = "INSERT INTO strategy_indicators (strategy_id, indicator_id, role, params, weight, position)
                 VALUES ($1, $2, $3, $4, $5::numeric, $6)";
        let r1 = sqlx::query(q).bind(sid_u).bind(sma_u).bind("entry")
            .bind(json!({"fast":5,"slow":20})).bind(1.0_f64).bind(0_i32).execute(&db).await;
        let r2 = sqlx::query(q).bind(sid_u).bind(rsi_u).bind("exit")
            .bind(json!({"period":14,"level":70,"direction":"above"})).bind(1.0_f64).bind(1_i32).execute(&db).await;
        t.check("seed strategy_indicators (entry sma_cross, exit rsi)", r1.is_ok() && r2.is_ok());

        let r = t.call("test: 1y daily backtest", Method::POST, &format!("/strategies/{sid}/test"), Some(json!({})), &[200]).await;
        if r.is_object() {
            let f = |k: &str| r[k].as_f64().unwrap_or(f64::NAN);
            let curve = r["equity_curve"].as_array().map(|a| a.len()).unwrap_or(0);
            println!(
                "\n  initial {:.2}  final {:.2}  return {:.2}%  max drawdown {:.2}%\n  trades {}  win rate {:.1}%  equity points {}",
                f("initial_capital"), f("final_equity"), f("total_return_pct"), f("max_drawdown_pct"),
                r["trade_count"], f("win_rate_pct"), curve
            );
            t.check("equity curve is not empty", curve > 1);
            t.check("final equity is finite and positive", f("final_equity").is_finite() && f("final_equity") > 0.0);
            t.check("win rate within 0..100", (0.0..=100.0).contains(&f("win_rate_pct")));
            t.check("max drawdown within 0..100", (0.0..=100.0).contains(&f("max_drawdown_pct")));
            t.check("trade_count matches trades array", r["trades"].as_array().map(|a| a.len() as u64) == r["trade_count"].as_u64());
        }
        t.call("test: custom range 6mo", Method::POST, &format!("/strategies/{sid}/test"), Some(json!({"time_frame":"6mo","bar_size":"1d"})), &[200]).await;
        t.call("test: invalid range -> client error", Method::POST, &format!("/strategies/{sid}/test"), Some(json!({"time_frame":"banana"})), CLIENT_ERR).await;
    } else {
        t.check("built-in indicators exist for the backtest", false);
    }

    println!("\n== cleanup / delete endpoints ==");
    t.call("delete holding", Method::DELETE, &format!("{hpath}/{hid}"), None, DEL).await;
    t.call("delete holding again -> 404", Method::DELETE, &format!("{hpath}/{hid}"), None, &[404]).await;
    t.call("delete collection", Method::DELETE, &format!("/strategies/{sid}/collections/{cid}"), None, DEL).await;
    t.call("delete strategy", Method::DELETE, &format!("/strategies/{sid}"), None, DEL).await;
    t.call("delete strategy again -> 404", Method::DELETE, &format!("/strategies/{sid}"), None, &[404]).await;
    t.call("delete indicator", Method::DELETE, &format!("/indicators/{ind_id}"), None, DEL).await;
    t.call("delete stock AAPL", Method::DELETE, &format!("/stocks/{aapl_id}"), None, DEL).await;
    t.call("delete stock MSFT", Method::DELETE, &format!("/stocks/{msft_id}"), None, DEL).await;
    t.call("delete user", Method::DELETE, &format!("/users/{uid}"), None, DEL).await;
    t.call("delete user again -> 404", Method::DELETE, &format!("/users/{uid}"), None, &[404]).await;

    println!("\n{} passed, {} failed", t.pass, t.fail);
    if t.fail > 0 {
        std::process::exit(1);
    }
}