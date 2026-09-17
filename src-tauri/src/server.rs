//! Server mode: raw HTTP/1.1 and SSE, no framework.
use crate::{
    assistant, bundle,
    db::{err, Db, Res},
    llm, pipeline,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::{atomic::AtomicBool, Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, RwLock};

const PAGE: &str = include_str!("../web/chat.html");

pub struct Args {
    pub host: String,
    pub port: u16,
    pub open: bool,
    pub data: Option<PathBuf>,
    /// Native window rather than a browser.
    pub gui: bool,
    pub explicit_port: bool,
}
/// Mode comes from the arguments, else from the executable name.
pub fn parse_args(argv: &[String]) -> Option<Args> {
    let explicit = argv.iter().any(|a| a == "--serve" || a == "--gui");
    let stem = std::env::current_exe()
        .ok()
        .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_lowercase()))
        .unwrap_or_default();
    let by_name = if stem.contains("chatbotgui") {
        Some(true)
    } else if stem.contains("chatbotweb") {
        Some(false)
    } else {
        None
    };
    if !explicit && by_name.is_none() {
        return None;
    }
    let mut a = Args {
        host: "127.0.0.1".into(),
        port: 8787,
        open: by_name == Some(false),
        data: None,
        gui: by_name == Some(true) || argv.iter().any(|x| x == "--gui"),
        explicit_port: false,
    };
    let mut it = argv.iter().skip(1);
    while let Some(x) = it.next() {
        match x.as_str() {
            "--host" => a.host = it.next().cloned().unwrap_or(a.host),
            "--port" => {
                a.port = it.next().and_then(|p| p.parse().ok()).unwrap_or(a.port);
                a.explicit_port = true;
            }
            "--open" => a.open = true,
            "--data" => a.data = it.next().map(PathBuf::from),
            "--bundle" => {
                if let Some(d) = it.next() {
                    std::env::set_var("LANGOLIER_BUNDLE_DIR", d);
                }
            }
            _ => {}
        }
    }
    Some(a)
}
struct App {
    db: Db,
    data_dir: PathBuf,
    swap: RwLock<()>,
    /// One generation at a time, web page and Telegram bridge together.
    generation_shared: Arc<Mutex<()>>,
    lockout: Mutex<Lockout>,
    chat_rate: StdMutex<HashMap<IpAddr, Vec<Instant>>>,
}
/// Wrong admin secrets: five strikes, then a fifteen minute pause.
#[derive(Default)]
struct Lockout {
    failures: u32,
    until: Option<std::time::Instant>,
}
const LOCKOUT_STRIKES: u32 = 5;
const LOCKOUT_PAUSE: Duration = Duration::from_secs(15 * 60);
/// Answering costs a whole model run, so one address gets a handful per
/// minute. It shields the machine when the page is served to a network.
const ASK_BURST: usize = 12;
const ASK_WINDOW: Duration = Duration::from_secs(60);
/// Records a question and says whether it is over the allowance.
fn over_chat_rate(app: &App, who: IpAddr) -> bool {
    let now = Instant::now();
    let Ok(mut seen) = app.chat_rate.lock() else {
        return false;
    };
    // Addresses that stopped asking must not pile up in memory.
    if seen.len() > 4096 {
        seen.retain(|_, hits| hits.iter().any(|t| now.duration_since(*t) < ASK_WINDOW));
    }
    let hits = seen.entry(who).or_default();
    hits.retain(|t| now.duration_since(*t) < ASK_WINDOW);
    if hits.len() >= ASK_BURST {
        return true;
    }
    hits.push(now);
    false
}
const IMPORT_MAX: u64 = 1 << 30;
pub fn main(args: Args) -> i32 {
    if args.gui {
        #[cfg(feature = "desktop")]
        return gui(args);
        #[cfg(not(feature = "desktop"))]
        {
            eprintln!("This binary is a headless server: use --serve.");
            return 2;
        }
    }
    let rt = tokio::runtime::Runtime::new().expect("tokio");
    let code = match rt.block_on(serve(args)) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("Error: {e}");
            1
        }
    };
    crate::engine::shutdown();
    code
}
#[cfg(feature = "desktop")]
static GUI: std::sync::OnceLock<tauri::AppHandle> = std::sync::OnceLock::new();
#[cfg(feature = "desktop")]
const CHAT_WINDOW: &str = "chat";
#[cfg(feature = "desktop")]
const PALETTE_WINDOW: &str = "palette";
#[cfg(feature = "desktop")]
fn gui(mut args: Args) -> i32 {
    args.host = "127.0.0.1".into();
    args.port = 0;
    let (tx, rx) = std::sync::mpsc::channel::<Result<Ready, String>>();
    let tx_err = tx.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("tokio");
        let result = rt.block_on(serve_with(args, move |r| {
            let _ = tx.send(Ok(r));
        }));
        if let Err(e) = result {
            let _ = tx_err.send(Err(e));
        }
    });
    let ready = match rx.recv() {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => {
            eprintln!("Error: {e}");
            return 1;
        }
        Err(_) => {
            eprintln!("Error: the local server did not start.");
            return 1;
        }
    };
    let r = ready.clone();
    let result = tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        toggle_palette(app);
                    }
                })
                .build(),
        )
        .on_window_event(move |window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } if window.label() == CHAT_WINDOW => {
                // With a tray icon the window only hides; without one, closing quits.
                if r.tray_icon || cfg!(target_os = "macos") {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
            tauri::WindowEvent::Focused(false) if window.label() == PALETTE_WINDOW => {
                let _ = window.hide();
            }
            _ => {}
        })
        .setup(move |app| {
            let _ = GUI.set(app.handle().clone());
            let target: tauri::Url = ready.url.parse().map_err(std::io::Error::other)?;
            let mut chat = tauri::WebviewWindowBuilder::new(
                app,
                CHAT_WINDOW,
                tauri::WebviewUrl::External(target),
            )
            .title(&ready.name)
            .inner_size(560.0, 800.0)
            .min_inner_size(380.0, 520.0)
            .visible(!ready.start_hidden);
            // Window icon for Windows and Linux; macOS takes the Dock icon below.
            if let Some(icon) = app.default_window_icon() {
                chat = chat.icon(icon.clone())?;
            }
            chat.build()?;
            // Dock icon: the avatar when there is one, else Langolier's.
            match ready.avatar.split_once(',') {
                Some((_, b64)) => crate::desktop::set_dock_icon(&crate::bundle::base64_decode(b64)),
                None => crate::desktop::set_dock_icon(crate::desktop::ICON_PNG),
            }
            if ready.tray_icon {
                if let Err(e) = build_tray(app.handle(), &ready.name) {
                    eprintln!("Tray: {e}");
                }
            }
            if !ready.palette_shortcut.is_empty() {
                use tauri_plugin_global_shortcut::GlobalShortcutExt;
                match ready
                    .palette_shortcut
                    .parse::<tauri_plugin_global_shortcut::Shortcut>()
                {
                    Ok(sc) => {
                        if let Err(e) = app.global_shortcut().register(sc) {
                            eprintln!("Shortcut: {e}");
                        }
                    }
                    Err(e) => eprintln!("Shortcut: {e}"),
                }
            }
            Ok(())
        })
        .build(crate::context())
        .map(|app| {
            app.run(|app, event| {
                // Dock click while the window is hidden.
                #[cfg(target_os = "macos")]
                if let tauri::RunEvent::Reopen { .. } = event {
                    use tauri::Manager;
                    if let Some(w) = app.get_webview_window(CHAT_WINDOW) {
                        let _ = w.show();
                        let _ = w.unminimize();
                        let _ = w.set_focus();
                    }
                }
                #[cfg(not(target_os = "macos"))]
                let _ = (app, event);
            })
        });
    crate::engine::shutdown();
    match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("Window: {e}");
            1
        }
    }
}
#[cfg(feature = "desktop")]
fn build_tray(app: &tauri::AppHandle, name: &str) -> Res<()> {
    use tauri::menu::{MenuBuilder, MenuItemBuilder};
    use tauri::Manager;
    let open = MenuItemBuilder::with_id("open", format!("Open {name}"))
        .build(app)
        .map_err(err)?;
    let ask = MenuItemBuilder::with_id("ask", "Ask")
        .build(app)
        .map_err(err)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit")
        .build(app)
        .map_err(err)?;
    let menu = MenuBuilder::new(app)
        .items(&[&open, &ask])
        .separator()
        .item(&quit)
        .build()
        .map_err(err)?;
    let mut builder = tauri::tray::TrayIconBuilder::with_id("chatbot")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .tooltip(name)
        .on_menu_event(|app, e| match e.id().as_ref() {
            "open" => {
                if let Some(w) = app.get_webview_window(CHAT_WINDOW) {
                    let _ = w.show();
                    let _ = w.unminimize();
                    let _ = w.set_focus();
                }
            }
            "ask" => toggle_palette(app),
            "quit" => {
                crate::engine::shutdown();
                app.exit(0)
            }
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone()).icon_as_template(false);
    }
    builder.build(app).map_err(err)?;
    Ok(())
}
/// Floating question bar on the same page, `#palette` mode: borderless,
/// always on top, hidden on blur or Escape.
#[cfg(feature = "desktop")]
fn toggle_palette(app: &tauri::AppHandle) {
    use tauri::Manager;
    let w = match app.get_webview_window(PALETTE_WINDOW) {
        Some(w) => w,
        None => {
            let Some(chat) = app.get_webview_window(CHAT_WINDOW) else {
                return;
            };
            let Ok(mut url) = chat.url() else { return };
            url.set_fragment(Some("palette"));
            let (width, height) = (720.0, 520.0);
            let mut b = tauri::WebviewWindowBuilder::new(
                app,
                PALETTE_WINDOW,
                tauri::WebviewUrl::External(url),
            )
            .title("Ask")
            .inner_size(width, height)
            .resizable(false)
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .shadow(false)
            .visible(false);
            if let Some(m) = app.primary_monitor().ok().flatten() {
                let (mw, mh) = (
                    m.size().width as f64 / m.scale_factor(),
                    m.size().height as f64 / m.scale_factor(),
                );
                b = b.position((mw - width) / 2.0, (mh * 0.18).max(40.0));
            } else {
                b = b.center();
            }
            match b.build() {
                Ok(w) => w,
                Err(e) => {
                    eprintln!("Palette: {e}");
                    return;
                }
            }
        }
    };
    if w.is_visible().unwrap_or(false) && w.is_focused().unwrap_or(false) {
        let _ = w.hide();
        return;
    }
    let _ = w.show();
    let _ = w.set_focus();
}
/// The page asks the launcher to hide the palette (Escape); loopback only.
pub fn hide_palette_window() {
    #[cfg(feature = "desktop")]
    {
        use tauri::Manager;
        if let Some(app) = GUI.get() {
            if let Some(w) = app.get_webview_window(PALETTE_WINDOW) {
                let _ = w.hide();
            }
        }
    }
}
/// What the window launcher needs to know once the server listens.
#[derive(Clone, Default)]
pub struct Ready {
    pub url: String,
    pub name: String,
    pub avatar: String,
    pub tray_icon: bool,
    pub palette_shortcut: String,
    pub start_hidden: bool,
}
pub async fn serve(args: Args) -> Res<()> {
    serve_with(args, |_| {}).await
}
/// `ready` fires once the port is open; window mode uses it.
pub async fn serve_with(args: Args, ready: impl FnOnce(Ready) + Send + 'static) -> Res<()> {
    let exe_dir = bundle::bundle_dir()?;
    let data_dir = match args.data {
        Some(d) => d,
        None => {
            let cands = bundle::candidates(&exe_dir, None)?;
            let slug = match cands.last() {
                Some(c) => {
                    let bytes = bundle::read_candidate(c)?;
                    let tmp = std::env::temp_dir()
                        .join(format!("langolier-probe-{}.langolier", std::process::id()));
                    std::fs::write(&tmp, bytes).map_err(err)?;
                    let s = bundle::slug_of(&tmp);
                    let _ = std::fs::remove_file(&tmp);
                    s
                }
                None => "chatbot".into(),
            };
            bundle::default_data_dir(&slug)?
        }
    };
    match bundle::sync(&exe_dir, &data_dir)? {
        Some(name) => println!("Bundle installed: {name}"),
        None if data_dir.join("langolier.sqlite3").exists() => {
            println!("Bundle already installed.")
        }
        None => {
            return Err(format!(
                "No .langolier file found in {} (nor embedded in the executable).",
                exe_dir.display()
            ))
        }
    }
    let db = Db::new(&data_dir)?;
    let s = db.settings()?;
    crate::engine::set_cache_dir(bundle::gguf_cache_dir(&db));
    crate::engine::set_idle_secs(s.engine_idle_minutes * 60);
    let profile = assistant::get(&db, &s.active_assistant)?;
    println!(
        "Assistant: {}",
        profile
            .as_ref()
            .map(|a| a.name.as_str())
            .unwrap_or("(no profile)")
    );
    println!("Engine: {} · {}", s.provider, s.model);
    if s.provider == "ollama"
        || (s.embedding_endpoint.trim() != llm::EMBEDDED && !llm::is_cloud(&s.provider))
    {
        prepare_ollama(&db).await;
    }
    let shared = Arc::new(Mutex::new(()));
    let app = Arc::new(App {
        db,
        data_dir: data_dir.clone(),
        swap: RwLock::new(()),
        generation_shared: shared,
        lockout: Mutex::new(Lockout::default()),
        chat_rate: StdMutex::new(HashMap::new()),
    });
    // Telegram bridge when the profile carries a token. LANGOLIER_DISABLE_TELEGRAM=1
    // keeps a test or a second instance from stealing the bot's updates.
    let telegram_off = std::env::var("LANGOLIER_DISABLE_TELEGRAM").is_ok_and(|v| v == "1");
    if profile
        .as_ref()
        .is_some_and(|a| !a.telegram_token.trim().is_empty())
    {
        println!(
            "{}",
            if telegram_off {
                "Telegram: bridge disabled by LANGOLIER_DISABLE_TELEGRAM."
            } else {
                "Telegram: bridge active for this profile."
            }
        );
    }
    if !telegram_off {
        let db = app.db.clone();
        let lock = app.generation_shared.clone();
        tokio::spawn(crate::telegram::poll(db, lock));
    }
    // Requested port, then the next ones.
    let explicit = args.explicit_port;
    let mut listener = None;
    let mut last_err = String::new();
    for offset in 0..(if explicit || args.port == 0 { 1 } else { 20 }) {
        let addr: SocketAddr = format!("{}:{}", args.host, args.port.saturating_add(offset))
            .parse()
            .map_err(err)?;
        match TcpListener::bind(addr).await {
            Ok(l) => {
                listener = Some(l);
                break;
            }
            Err(e) => last_err = format!("Cannot listen on {addr}: {e}"),
        }
    }
    let Some(listener) = listener else {
        return Err(last_err);
    };
    let port = listener.local_addr().map_err(err)?.port();
    let url = format!(
        "http://{}:{}/",
        if args.host == "0.0.0.0" {
            "localhost"
        } else {
            &args.host
        },
        port
    );
    println!("Chat page: {url}");
    ready(Ready {
        url: url.clone(),
        name: profile
            .as_ref()
            .map(|a| a.name.clone())
            .unwrap_or_else(|| "Assistant".into()),
        avatar: profile
            .as_ref()
            .map(|a| a.avatar.clone())
            .unwrap_or_default(),
        tray_icon: profile
            .as_ref()
            .is_some_and(|a| a.tray_icon || a.start_hidden),
        palette_shortcut: profile
            .as_ref()
            .map(|a| a.palette_shortcut.trim().to_string())
            .unwrap_or_default(),
        start_hidden: profile.as_ref().is_some_and(|a| a.start_hidden),
    });
    if args.open && !args.gui {
        let _ = std::process::Command::new(if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        })
        .arg(&url)
        .spawn();
    }
    {
        let app = app.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(20)).await;
                let _w = app.swap.write().await;
                match bundle::sync(&exe_dir, &data_dir) {
                    Ok(Some(name)) => println!("Update installed: {name}"),
                    Ok(None) => {}
                    Err(e) => eprintln!("Update refused: {e}"),
                }
            }
        });
    }
    loop {
        let (stream, peer) = listener.accept().await.map_err(err)?;
        let app = app.clone();
        tokio::spawn(async move {
            if let Err(e) = handle(stream, app, peer.ip()).await {
                eprintln!("Request: {e}");
            }
        });
    }
}
/// The local API the window switches on: loopback only, one shared engine
/// lock with the window, and the same routes the exported page answers.
/// Returns once the port is open; the listener lives until `stop` fires.
pub async fn serve_local(
    db: Db,
    data_dir: PathBuf,
    generation: Arc<Mutex<()>>,
    port: u16,
    stop: tokio::sync::oneshot::Receiver<()>,
) -> Res<u16> {
    // Loopback only. A local memory has no business on the network.
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port))
        .await
        .map_err(|e| format!("Cannot listen on 127.0.0.1:{port}: {e}"))?;
    let bound = listener.local_addr().map_err(err)?.port();
    let app = Arc::new(App {
        db,
        data_dir,
        swap: RwLock::new(()),
        generation_shared: generation,
        lockout: Mutex::new(Lockout::default()),
        chat_rate: StdMutex::new(HashMap::new()),
    });
    tokio::spawn(async move {
        tokio::pin!(stop);
        loop {
            tokio::select! {
                _ = &mut stop => return,
                accepted = listener.accept() => match accepted {
                    Ok((stream, peer)) => {
                        let app = app.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle(stream, app, peer.ip()).await {
                                eprintln!("Local API: {e}")
                            }
                        });
                    }
                    Err(e) => {
                        eprintln!("Local API: {e}");
                        return;
                    }
                },
            }
        }
    });
    Ok(bound)
}
/// Engine state, for the page.
async fn health(s: &crate::db::Settings) -> Value {
    let mut problems: Vec<String> = vec![];
    let mut degraded = false;
    let embedded_chat = s.provider == llm::EMBEDDED;
    let embedded_embed = s.embedding_endpoint.trim() == llm::EMBEDDED;
    if embedded_chat && !crate::engine::available(&s.model) {
        problems.push(format!(
            "Chat model missing: {} (expected in the models/ folder).",
            s.model
        ));
    }
    if embedded_embed && !crate::engine::available(&s.embedding_model) {
        problems.push(format!(
            "Embedding model missing: {} - word search only.",
            s.embedding_model
        ));
        degraded = true;
    }
    let chat_via_ollama = !embedded_chat && s.provider == "ollama";
    let embed_via_ollama = !embedded_embed;
    if chat_via_ollama || embed_via_ollama {
        // Both go through the Ollama embedding server.
        let base = s
            .embedding_endpoint
            .trim()
            .trim_end_matches('/')
            .to_string();
        let tags = match llm::client() {
            Ok(c) => c
                .get(format!("{base}/api/tags"))
                .timeout(Duration::from_secs(6))
                .send()
                .await
                .ok(),
            Err(_) => None,
        };
        match tags {
            Some(r) if r.status().is_success() => {
                let v: Value = r.json().await.unwrap_or(json!({}));
                let names: Vec<String> = v["models"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|m| m["name"].as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();
                let has = |m: &str| {
                    names.iter().any(|n| {
                        n == m || n.trim_end_matches(":latest") == m || *n == format!("{m}:latest")
                    })
                };
                if chat_via_ollama && !has(&s.model) {
                    problems.push(format!(
                        "Chat model \"{}\" missing from Ollama: `ollama pull {}`.",
                        s.model, s.model
                    ));
                }
                if embed_via_ollama && !has(&s.embedding_model) {
                    problems.push(format!("Embedding model \"{}\" missing from Ollama: `ollama pull {}` - word search only until then.", s.embedding_model, s.embedding_model));
                    degraded = true;
                }
            }
            _ => {
                if chat_via_ollama {
                    problems.push(format!(
                        "Ollama is not answering on {base}: answers will fail until it starts."
                    ));
                } else {
                    problems.push(format!(
                        "Ollama is not answering on {base}: word search only."
                    ));
                    degraded = true;
                }
            }
        }
    }
    json!({"ok": problems.is_empty(), "degraded": degraded, "problems": problems, "engine": if embedded_chat { "embedded" } else { s.provider.as_str() }, "model": s.model, "embedding_model": s.embedding_model})
}
/// Checks Ollama and pulls the missing models.
async fn prepare_ollama(db: &Db) {
    let Ok(s) = db.settings() else { return };
    let Ok(models) = llm::models(&s).await else {
        eprintln!("Ollama unreachable on {}: search will be lexical and answers will fail until it starts.", s.endpoint);
        return;
    };
    let names: Vec<String> = models
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|m| m["name"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let has = |m: &str| {
        names
            .iter()
            .any(|n| n == m || n.trim_end_matches(":latest") == m || n == &format!("{m}:latest"))
    };
    let wanted: Vec<String> = [
        (s.provider.as_str() != llm::EMBEDDED, s.model.clone()),
        (
            s.embedding_endpoint.trim() != llm::EMBEDDED,
            s.embedding_model.clone(),
        ),
    ]
    .into_iter()
    .filter(|(via_ollama, _)| *via_ollama)
    .map(|(_, m)| m)
    .collect();
    for model in wanted {
        if has(&model) {
            continue;
        }
        println!("Downloading model {model}…");
        let base = s.endpoint.trim_end_matches('/');
        let Ok(client) = llm::client() else { continue };
        match client
            .post(format!("{base}/api/pull"))
            .json(&json!({"model": model, "stream": false}))
            .timeout(Duration::from_secs(3600))
            .send()
            .await
        {
            Ok(r) if r.status().is_success() => println!("Model {model} ready."),
            Ok(r) => eprintln!("Download of {model} refused: {}", r.status()),
            Err(e) => eprintln!("Cannot download {model}: {e}"),
        }
    }
}
struct Request {
    method: String,
    path: String,
    query: HashMap<String, String>,
    body: Vec<u8>,
    length: u64,
    admin_token: String,
    bearer: String,
}
async fn read_head(reader: &mut BufReader<TcpStream>) -> Res<Request> {
    let mut line = String::new();
    reader.read_line(&mut line).await.map_err(err)?;
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let target = parts.next().unwrap_or("/").to_string();
    let mut length = 0u64;
    let mut admin_token = String::new();
    let mut bearer = String::new();
    loop {
        let mut h = String::new();
        reader.read_line(&mut h).await.map_err(err)?;
        let h = h.trim_end();
        if h.is_empty() {
            break;
        }
        let Some((k, v)) = h.split_once(':') else {
            continue;
        };
        match k.trim().to_ascii_lowercase().as_str() {
            "content-length" => length = v.trim().parse().unwrap_or(0),
            "x-admin-token" => admin_token = v.trim().to_string(),
            "authorization" => {
                bearer = v
                    .trim()
                    .strip_prefix("Bearer ")
                    .unwrap_or_default()
                    .trim()
                    .to_string()
            }
            _ => {}
        }
    }
    let (path, qs) = target.split_once('?').unwrap_or((&target, ""));
    let query = qs
        .split('&')
        .filter(|p| !p.is_empty())
        .filter_map(|p| {
            p.split_once('=')
                .map(|(k, v)| (k.to_string(), percent_decode(v)))
        })
        .collect();
    Ok(Request {
        method,
        path: path.to_string(),
        query,
        body: Vec::new(),
        length,
        admin_token,
        bearer,
    })
}
async fn read_body(reader: &mut BufReader<TcpStream>, req: &mut Request) -> Res<()> {
    if req.length > 2_000_000 {
        return Err("Body too large".into());
    }
    let mut body = vec![0u8; req.length as usize];
    reader.read_exact(&mut body).await.map_err(err)?;
    req.body = body;
    Ok(())
}
/// Streams a large body to disk instead of holding it in memory.
async fn body_to_file(
    reader: &mut BufReader<TcpStream>,
    length: u64,
    dest: &std::path::Path,
) -> Res<()> {
    if length == 0 || length > IMPORT_MAX {
        return Err(format!(
            "A bundle must be between 1 byte and {} MB.",
            IMPORT_MAX >> 20
        ));
    }
    let mut f = tokio::fs::File::create(dest).await.map_err(err)?;
    let mut left = length;
    let mut buf = vec![0u8; 1 << 16];
    while left > 0 {
        let n = reader
            .read(&mut buf[..(left.min(1 << 16)) as usize])
            .await
            .map_err(err)?;
        if n == 0 {
            return Err("Upload cut short.".into());
        }
        f.write_all(&buf[..n]).await.map_err(err)?;
        left -= n as u64;
    }
    f.flush().await.map_err(err)
}
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(if bytes[i] == b'+' { b' ' } else { bytes[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}
async fn respond(w: &mut TcpStream, status: &str, content_type: &str, body: &[u8]) -> Res<()> {
    respond_with(w, status, content_type, body, &[]).await
}
async fn respond_with(
    w: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
    extra: &[(&str, &str)],
) -> Res<()> {
    let mut headers = String::new();
    for (name, value) in extra {
        headers.push_str(&format!("{name}: {value}\r\n"));
    }
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: strict-origin-when-cross-origin\r\nX-Frame-Options: DENY\r\n{headers}Connection: close\r\n\r\n",
        body.len()
    );
    w.write_all(head.as_bytes()).await.map_err(err)?;
    w.write_all(body).await.map_err(err)?;
    w.flush().await.map_err(err)
}
async fn json_ok(w: &mut TcpStream, v: Value) -> Res<()> {
    respond(
        w,
        "200 OK",
        "application/json; charset=utf-8",
        v.to_string().as_bytes(),
    )
    .await
}
async fn json_err(w: &mut TcpStream, status: &str, msg: &str) -> Res<()> {
    respond(
        w,
        status,
        "application/json; charset=utf-8",
        json!({"error": msg}).to_string().as_bytes(),
    )
    .await
}
/// The page and the policy that lets it run: one nonce per response, so the
/// inline style and script need no unsafe-inline.
fn page_with_nonce(nonce: &str) -> (String, String) {
    let page = PAGE
        .replacen("<style>", &format!("<style nonce=\"{nonce}\">"), 1)
        .replacen("<script>", &format!("<script nonce=\"{nonce}\">"), 1);
    let csp = format!(
        "default-src 'none'; script-src 'nonce-{nonce}'; style-src 'nonce-{nonce}'; img-src 'self' data:; connect-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'"
    );
    (page, csp)
}
async fn handle(stream: TcpStream, app: Arc<App>, peer: IpAddr) -> Res<()> {
    let mut reader = BufReader::new(stream);
    let mut req = read_head(&mut reader).await?;
    if req.path.starts_with("/api/admin/") {
        return admin(reader, req, app).await;
    }
    read_body(&mut reader, &mut req).await?;
    let mut w = reader.into_inner();
    let _r = app.swap.read().await;
    match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/") | ("GET", "/index.html") => {
            let (page, csp) = page_with_nonce(&uuid::Uuid::new_v4().simple().to_string());
            respond_with(
                &mut w,
                "200 OK",
                "text/html; charset=utf-8",
                page.as_bytes(),
                &[("Content-Security-Policy", csp.as_str())],
            )
            .await
        }
        ("GET", "/api/config") => {
            let s = app.db.settings()?;
            let a = assistant::get(&app.db, &s.active_assistant)?.unwrap_or_default();
            json_ok(&mut w, json!({"name": a.name, "welcome": a.welcome, "avatar": a.avatar, "theme": a.theme, "show_sources": a.show_sources, "language": a.language, "admin": {"enabled": a.admin_enabled, "import": a.admin_enabled && a.admin_import, "restore": a.admin_enabled && a.admin_restore}})).await
        }
        ("GET", "/api/health") => {
            let s = app.db.settings()?;
            json_ok(&mut w, health(&s).await).await
        }
        ("POST", "/api/conversation") => {
            let s = app.db.settings()?;
            let id = pipeline::new_conversation(&app.db, Some(&s.active_assistant))?;
            json_ok(&mut w, json!({"id": id})).await
        }
        ("GET", "/api/messages") => {
            let id = req.query.get("id").cloned().unwrap_or_default();
            if !valid_id(&id) {
                return json_err(&mut w, "400 Bad Request", "invalid id").await;
            }
            // No SQLite connection may cross an await: it is not Send.
            let rows = {
                let c = app.db.conn()?;
                let mut st = c.prepare("SELECT role,content,sources FROM messages WHERE conversation_id=?1 ORDER BY rowid").map_err(err)?;
                let rows = st
                    .query_map([&id], |r| {
                        let sources: String = r.get(2)?;
                        Ok(json!({"role": r.get::<_, String>(0)?, "content": r.get::<_, String>(1)?, "sources": serde_json::from_str::<Value>(&sources).unwrap_or(json!([]))}))
                    })
                    .map_err(err)?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(err)?;
                rows
            };
            json_ok(&mut w, Value::Array(rows)).await
        }
        // One answer, one JSON object, no stream: what Shortcuts, Siri and
        // local agents can consume. Same pipeline as the page and the app.
        ("POST", "/api/ask") => {
            let s = app.db.settings()?;
            let expected = s.local_api_token.trim();
            if expected.is_empty() || req.bearer != expected {
                return json_err(&mut w, "401 Unauthorized", "Wrong or missing token.").await;
            }
            if over_chat_rate(&app, peer) {
                return json_err(
                    &mut w,
                    "429 Too Many Requests",
                    "Too many questions in a row. Try again in a minute.",
                )
                .await;
            }
            ask(&mut w, &req, &app, &s).await
        }
        ("POST", "/api/chat") => {
            if over_chat_rate(&app, peer) {
                return json_err(
                    &mut w,
                    "429 Too Many Requests",
                    "Too many questions in a row. Try again in a minute.",
                )
                .await;
            }
            chat(&mut w, &req, &app).await
        }
        ("POST", "/api/ui/hide") => {
            hide_palette_window();
            json_ok(&mut w, json!({"ok": true})).await
        }
        _ => json_err(&mut w, "404 Not Found", "not found").await,
    }
}
/// Remote administration of the running chatbot: import a bundle, list the
/// store, restore or delete an entry. Everything needs the profile's secret;
/// when administration is off the routes do not exist at all.
async fn admin(mut reader: BufReader<TcpStream>, mut req: Request, app: Arc<App>) -> Res<()> {
    let s = app.db.settings()?;
    let a = assistant::get(&app.db, &s.active_assistant)?.unwrap_or_default();
    if !a.admin_enabled {
        let mut w = reader.into_inner();
        return json_err(&mut w, "404 Not Found", "not found").await;
    }
    {
        let mut lock = app.lockout.lock().await;
        if let Some(until) = lock.until {
            if std::time::Instant::now() < until {
                let mut w = reader.into_inner();
                return json_err(
                    &mut w,
                    "429 Too Many Requests",
                    "Too many wrong secrets. Try again in a few minutes.",
                )
                .await;
            }
            lock.until = None;
            lock.failures = 0;
        }
        if !assistant::admin_secret_matches(&a, &req.admin_token) {
            lock.failures += 1;
            if lock.failures >= LOCKOUT_STRIKES {
                lock.until = Some(std::time::Instant::now() + LOCKOUT_PAUSE);
                lock.failures = 0;
            }
            let mut w = reader.into_inner();
            return json_err(&mut w, "401 Unauthorized", "Wrong secret.").await;
        }
        lock.failures = 0;
    }
    let is_import = req.method == "POST" && req.path == "/api/admin/import";
    let staged = app
        .data_dir
        .join(format!("upload-{}.langolier", std::process::id()));
    if is_import {
        if !a.admin_import {
            let mut w = reader.into_inner();
            return json_err(
                &mut w,
                "403 Forbidden",
                "Import is disabled for this chatbot.",
            )
            .await;
        }
        if let Err(e) = body_to_file(&mut reader, req.length, &staged).await {
            let _ = std::fs::remove_file(&staged);
            let mut w = reader.into_inner();
            return json_err(&mut w, "400 Bad Request", &e).await;
        }
    } else {
        read_body(&mut reader, &mut req).await?;
    }
    let mut w = reader.into_inner();
    let entries = |app: &App| -> Res<Value> {
        let list: Vec<Value> = bundle::store_entries(&app.data_dir)?
            .into_iter()
            .map(|e| json!({"id": e.id, "name": e.name, "size": e.size, "imported_at": e.modified, "active": e.active, "kit": e.kit}))
            .collect();
        Ok(json!({"bundles": list, "import": a.admin_import, "restore": a.admin_restore}))
    };
    let outcome: Res<Value> = match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/api/admin/bundles") => entries(&app),
        ("POST", "/api/admin/import") => {
            let bytes = std::fs::read(&staged).map_err(err)?;
            let _ = std::fs::remove_file(&staged);
            // Refuse a bundle whose embedder is missing here, unless forced:
            // it would silently fall back to word search.
            let forced = req.query.get("force").map(String::as_str) == Some("1");
            let mut refusal = None;
            if !forced {
                if let Ok(candidate) = bundle_settings(&bytes) {
                    let h = health(&candidate).await;
                    if h["degraded"] == true {
                        let why = h["problems"]
                            .as_array()
                            .map(|p| {
                                p.iter()
                                    .filter_map(Value::as_str)
                                    .collect::<Vec<_>>()
                                    .join(" ")
                            })
                            .unwrap_or_default();
                        refusal = Some(format!("This bundle needs an embedding model that is not available here: {why} Send again with force=1 to accept word search only."));
                    }
                }
            }
            match refusal {
                Some(why) => Err(why),
                None => {
                    let name = {
                        let _g = app.swap.write().await;
                        bundle::store_import(&app.data_dir, &bytes)
                    };
                    name.and_then(|name| {
                        println!("Imported from the page: {name}");
                        Ok(json!({"name": name, "list": entries(&app)?}))
                    })
                }
            }
        }
        ("POST", "/api/admin/restore") if !a.admin_restore => {
            Err("Restore is disabled for this chatbot.".into())
        }
        ("POST", "/api/admin/delete") if !a.admin_restore => {
            Err("Restore is disabled for this chatbot.".into())
        }
        ("POST", "/api/admin/restore") => {
            let id = admin_id(&req)?;
            let name = {
                let _g = app.swap.write().await;
                bundle::store_restore(&app.data_dir, &id)
            };
            name.and_then(|name| {
                println!("Restored from the page: {name}");
                Ok(json!({"name": name, "list": entries(&app)?}))
            })
        }
        ("POST", "/api/admin/delete") => admin_id(&req)
            .and_then(|id| bundle::store_delete(&app.data_dir, &id))
            .and_then(|_| Ok(json!({"list": entries(&app)?}))),
        _ => return json_err(&mut w, "404 Not Found", "not found").await,
    };
    match outcome {
        Ok(v) => json_ok(&mut w, v).await,
        Err(e) => json_err(&mut w, "400 Bad Request", &e).await,
    }
}
fn admin_id(req: &Request) -> Res<String> {
    let v: Value = serde_json::from_slice(&req.body)
        .map_err(|_| "Expected a JSON body with an id.".to_string())?;
    let id = v["id"].as_str().unwrap_or("").to_string();
    if id.is_empty() || id.contains('/') || id.contains("..") || !id.ends_with(".langolier") {
        return Err("Invalid bundle id.".into());
    }
    Ok(id)
}
/// Settings row of a bundle, to check it against this host before installing.
fn bundle_settings(bytes: &[u8]) -> Res<crate::db::Settings> {
    let tmp =
        std::env::temp_dir().join(format!("langolier-check-{}.langolier", std::process::id()));
    std::fs::write(&tmp, bytes).map_err(err)?;
    let out = (|| {
        let c =
            rusqlite::Connection::open_with_flags(&tmp, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(err)?;
        let raw: String = c
            .query_row("SELECT value FROM settings WHERE id=1", [], |r| r.get(0))
            .map_err(err)?;
        serde_json::from_str(&raw).map_err(err)
    })();
    let _ = std::fs::remove_file(&tmp);
    out
}
fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}
/// Answers once and returns the pipeline's own payload, so the app, the page
/// and this endpoint never drift into three slightly different answers.
async fn ask(w: &mut TcpStream, req: &Request, app: &Arc<App>, s: &crate::db::Settings) -> Res<()> {
    let body: Value = serde_json::from_slice(&req.body).unwrap_or(json!({}));
    let question = body["question"].as_str().unwrap_or("").trim().to_string();
    if question.is_empty() {
        return json_err(w, "400 Bad Request", "empty question").await;
    }
    let profile = assistant::get(&app.db, &s.active_assistant)?;
    let effective = assistant::effective(s, profile.as_ref())?;
    let conversation = pipeline::new_conversation(&app.db, Some(&s.active_assistant))?;
    // The engine is shared with the window: one generation at a time.
    let _gen = app.generation_shared.lock().await;
    let answered = pipeline::answer(
        &app.db,
        &effective,
        profile.as_ref(),
        &conversation,
        &question,
        "hybrid",
        false,
        Arc::new(AtomicBool::new(false)),
        &|_, _| {},
    )
    .await;
    match answered {
        Ok(mut v) => {
            // A shortcut reads one field; walking the sources array in the
            // Shortcuts editor is painful. "content" stays untouched for the
            // spoken answer, "text" carries the same thing plus its citations.
            v["text"] = json!(with_citations(&v));
            json_ok(w, v).await
        }
        Err(e) => json_err(w, "500 Internal Server Error", &e).await,
    }
}
/// The answer followed by the sources its [n] markers point at.
fn with_citations(answered: &Value) -> String {
    let content = answered["content"].as_str().unwrap_or_default();
    let sources = match answered["sources"].as_array() {
        Some(list) if !list.is_empty() => list,
        _ => return content.to_string(),
    };
    let list = sources
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let name = s["name"].as_str().unwrap_or("?");
            match s["locator"].as_str().unwrap_or_default() {
                "" => format!("[{}] {name}", i + 1),
                locator => format!("[{}] {name}, {locator}", i + 1),
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("{content}\n\nSources\n{list}")
}
async fn chat(w: &mut TcpStream, req: &Request, app: &Arc<App>) -> Res<()> {
    let body: Value = serde_json::from_slice(&req.body).unwrap_or(json!({}));
    let question = body["question"].as_str().unwrap_or("").trim().to_string();
    let mut conversation = body["conversation_id"].as_str().unwrap_or("").to_string();
    if question.is_empty() {
        return json_err(w, "400 Bad Request", "empty question").await;
    }
    let s = app.db.settings()?;
    if conversation.is_empty() || !valid_id(&conversation) {
        conversation = pipeline::new_conversation(&app.db, Some(&s.active_assistant))?;
    } else {
        let known: i64 = {
            let c = app.db.conn()?;
            c.query_row(
                "SELECT count(*) FROM conversations WHERE id=?1",
                [&conversation],
                |r| r.get(0),
            )
            .map_err(err)?
        };
        if known == 0 {
            conversation = pipeline::new_conversation(&app.db, Some(&s.active_assistant))?;
        }
    }
    let profile = assistant::get(&app.db, &s.active_assistant)?;
    let effective = assistant::effective(&s, profile.as_ref())?;
    w.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream; charset=utf-8\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n").await.map_err(err)?;
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(String, Value)>();
    let emit_tx = tx.clone();
    let emit = move |kind: &str, payload: Value| {
        let _ = emit_tx.send((kind.to_string(), payload));
    };
    let db = app.db.clone();
    let conv = conversation.clone();
    let gen_lock = app.generation_shared.lock().await;
    let cancel = Arc::new(AtomicBool::new(false));
    let worker = tokio::spawn(async move {
        let r = pipeline::answer(
            &db,
            &effective,
            profile.as_ref(),
            &conv,
            &question,
            "hybrid",
            false,
            cancel,
            &emit,
        )
        .await;
        let _ = tx.send((
            "__done".into(),
            match r {
                Ok(v) => v,
                Err(e) => json!({"error": e}),
            },
        ));
    });
    let _ = tx_event(w, "conversation", json!({"id": conversation})).await;
    while let Some((kind, payload)) = rx.recv().await {
        if kind == "__done" {
            let event = if payload["error"].is_null() {
                "done"
            } else {
                "error"
            };
            let _ = tx_event(w, event, payload).await;
            break;
        }
        if tx_event(w, &kind, payload).await.is_err() {
            break;
        }
    }
    let _ = worker.await;
    drop(gen_lock);
    Ok(())
}
async fn tx_event(w: &mut TcpStream, event: &str, payload: Value) -> Res<()> {
    let frame = format!("event: {event}\ndata: {}\n\n", payload);
    w.write_all(frame.as_bytes()).await.map_err(err)?;
    w.flush().await.map_err(err)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn app() -> Arc<App> {
        let dir = Box::leak(Box::new(tempfile::tempdir().unwrap()));
        Arc::new(App {
            db: Db::new(dir.path()).unwrap(),
            data_dir: dir.path().to_path_buf(),
            swap: RwLock::new(()),
            generation_shared: Arc::new(Mutex::new(())),
            lockout: Mutex::new(Lockout::default()),
            chat_rate: StdMutex::new(HashMap::new()),
        })
    }
    /// Reads one HTTP response off a socket, head and body.
    ///
    /// Routing reaches the palette window, so with the desktop feature on, the
    /// test binary links the Windows GUI stack and fails to start on images
    /// whose uxtheme.dll lacks the dark mode ordinals. The server it exercises
    /// is the headless one anyway, so these run without that feature.
    #[cfg(not(feature = "desktop"))]
    async fn roundtrip(app: Arc<App>, request: &str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, peer) = listener.accept().await.unwrap();
            let _ = handle(stream, app, peer.ip()).await;
        });
        let mut client = TcpStream::connect(addr).await.unwrap();
        client.write_all(request.as_bytes()).await.unwrap();
        client.flush().await.unwrap();
        let mut answer = vec![];
        client.read_to_end(&mut answer).await.unwrap();
        server.await.unwrap();
        String::from_utf8_lossy(&answer).to_string()
    }
    #[test]
    fn ids_from_the_query_string_are_bounded() {
        assert!(valid_id("abc-123"));
        assert!(!valid_id(""));
        assert!(!valid_id("../../etc/passwd"));
        assert!(!valid_id("a b"));
        assert!(!valid_id("'; DROP TABLE messages--"));
        assert!(!valid_id(&"a".repeat(65)));
        assert!(valid_id(&"a".repeat(64)))
    }
    /// The policy must name the nonce the page actually carries, and must not
    /// fall back to unsafe-inline.
    #[test]
    fn page_policy_matches_the_page() {
        let (page, csp) = page_with_nonce("abc123");
        assert!(page.contains("<style nonce=\"abc123\">"));
        assert!(page.contains("<script nonce=\"abc123\">"));
        assert!(csp.contains("script-src 'nonce-abc123'"));
        assert!(csp.contains("style-src 'nonce-abc123'"));
        assert!(!csp.contains("unsafe-inline"));
        assert!(csp.contains("frame-ancestors 'none'"));
        // Avatars are data URIs, and nothing else may load.
        assert!(csp.contains("img-src 'self' data:"));
        assert!(csp.starts_with("default-src 'none'"))
    }
    #[test]
    fn questions_are_rationed_per_address() {
        let app = app();
        let one: IpAddr = "10.0.0.1".parse().unwrap();
        let two: IpAddr = "10.0.0.2".parse().unwrap();
        for i in 0..ASK_BURST {
            assert!(!over_chat_rate(&app, one), "question {i} refused too early");
        }
        assert!(over_chat_rate(&app, one), "the allowance never ran out");
        // One noisy address must not silence the others.
        assert!(!over_chat_rate(&app, two))
    }
    #[cfg(not(feature = "desktop"))]
    #[tokio::test]
    async fn serves_the_page_with_its_policy() {
        let answer = roundtrip(app(), "GET / HTTP/1.1\r\nHost: x\r\n\r\n").await;
        assert!(answer.starts_with("HTTP/1.1 200 OK"));
        assert!(answer.contains("Content-Security-Policy: default-src 'none'"));
        assert!(answer.contains("Referrer-Policy: strict-origin-when-cross-origin"));
        assert!(answer.contains("X-Content-Type-Options: nosniff"));
        assert!(answer.contains("X-Frame-Options: DENY"));
        assert!(answer.contains("<!doctype html>"))
    }
    #[cfg(not(feature = "desktop"))]
    #[tokio::test]
    async fn unknown_paths_and_methods_are_refused() {
        let answer = roundtrip(app(), "GET /secrets HTTP/1.1\r\nHost: x\r\n\r\n").await;
        assert!(answer.starts_with("HTTP/1.1 404 Not Found"), "{answer}");
        let answer = roundtrip(app(), "DELETE / HTTP/1.1\r\nHost: x\r\n\r\n").await;
        assert!(answer.starts_with("HTTP/1.1 404 Not Found"), "{answer}")
    }
    #[cfg(not(feature = "desktop"))]
    #[tokio::test]
    async fn a_bad_conversation_id_is_rejected_before_the_database() {
        let answer = roundtrip(
            app(),
            "GET /api/messages?id=..%2F..%2Fetc HTTP/1.1\r\nHost: x\r\n\r\n",
        )
        .await;
        assert!(answer.starts_with("HTTP/1.1 400 Bad Request"), "{answer}");
        assert!(answer.contains("invalid id"))
    }
    /// The spoken answer and the shown answer come from one place: the same
    /// numbering, whatever the sources hold.
    #[test]
    fn citations_are_spelled_out_under_the_answer() {
        let answered = json!({
            "content": "L'eau se purifie par ebullition [1] ou filtration [2].",
            "sources": [
                {"name": "French-Edition.pdf", "locator": "Page 222"},
                {"name": "Wiseman.pdf", "locator": ""},
            ]
        });
        let text = with_citations(&answered);
        assert!(text.starts_with("L'eau se purifie"));
        assert!(text.contains("\n\nSources\n"));
        assert!(text.contains("[1] French-Edition.pdf, Page 222"));
        // No locator, no trailing comma.
        assert!(text.contains("[2] Wiseman.pdf\n") || text.ends_with("[2] Wiseman.pdf"));
        // Nothing to cite: the answer is returned as it stands.
        let bare = json!({"content": "Je n'ai pas cette information.", "sources": []});
        assert_eq!(with_citations(&bare), "Je n'ai pas cette information.");
        assert_eq!(with_citations(&json!({})), "")
    }
    /// The local API answers nobody without the right bearer token, and never
    /// at all while no token is set.
    #[cfg(not(feature = "desktop"))]
    #[tokio::test]
    async fn the_local_api_demands_its_token() {
        let app = app();
        let body = "{\"question\":\"salut\"}";
        let ask = |head: &str| {
            format!(
                "POST /api/ask HTTP/1.1\r\nHost: x\r\n{head}Content-Length: {}\r\n\r\n{body}",
                body.len()
            )
        };
        // No token configured: the endpoint stays shut even with a guess.
        let answer = roundtrip(app.clone(), &ask("Authorization: Bearer guess\r\n")).await;
        assert!(answer.starts_with("HTTP/1.1 401"), "{answer}");
        let mut s = app.db.settings().unwrap();
        s.local_api_token = "sesame".into();
        app.db.set_settings(&s).unwrap();
        for head in [
            "",
            "Authorization: Bearer \r\n",
            "Authorization: Bearer nope\r\n",
        ] {
            let answer = roundtrip(app.clone(), &ask(head)).await;
            assert!(answer.starts_with("HTTP/1.1 401"), "{head:?} -> {answer}");
        }
        // A GET must not carry the question either.
        let answer = roundtrip(
            app.clone(),
            "GET /api/ask?question=salut HTTP/1.1\r\nHost: x\r\n\r\n",
        )
        .await;
        assert!(answer.starts_with("HTTP/1.1 404"), "{answer}")
    }
    /// Administration is off by default: the routes must not even exist.
    #[cfg(not(feature = "desktop"))]
    #[tokio::test]
    async fn admin_routes_are_absent_until_enabled() {
        let answer = roundtrip(
            app(),
            "POST /api/admin/list HTTP/1.1\r\nHost: x\r\nContent-Length: 0\r\n\r\n",
        )
        .await;
        assert!(answer.starts_with("HTTP/1.1 404 Not Found"), "{answer}")
    }
}
