use crate::simconfig;

use base64::Engine;
use crossbeam_channel::{bounded, unbounded, Receiver, Sender, TryRecvError};
use laminar::Metrics;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    fs::File,
    io::Read,
    net::IpAddr,
    sync::{
        atomic::{AtomicBool, Ordering::SeqCst},
        Arc, Mutex,
    },
    thread,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionMethod {
    Direct,
    Relay,
    CloudServer,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AppMessage {
    StartServer {
        username: String,
        is_ipv6: bool,
        use_upnp: bool,
        port: u16,
        method: ConnectionMethod,
    },
    Connect {
        username: String,
        session_id: Option<String>,
        isipv6: bool,
        ip: Option<IpAddr>,
        hostname: Option<String>,
        port: Option<u16>,
        method: ConnectionMethod,
    },
    TransferControl {
        target: String,
    },
    SetObserver {
        target: String,
        is_observer: bool,
    },
    LoadAircraft {
        config_file_name: String,
        sim: String,
    },
    Disconnect,
    Startup,
    RunUpdater,
    ForceTakeControl,
    UpdateConfig {
        new_config: simconfig::Config,
    },
    GoObserver,
    EmulatorRequestVars,
    EmulatorAddVar {
        name: String,
    },
    EmulatorRemoveVar {
        name: String,
    },
    EmulatorSetVar {
        name: String,
        value: f64,
    },
}

fn get_message_str(type_string: &str, data: &str) -> String {
    format!(
        r#"MessageReceived({})"#,
        serde_json::json!({"type": type_string, "data": data})
    )
}

pub trait App {
    fn exited(&self) -> bool;
    fn get_next_message(&self) -> Result<AppMessage, TryRecvError>;
    fn invoke(&self, type_string: &str, data: Option<&str>);

    fn error(&self, msg: &str) {
        self.invoke("error", Some(msg));
    }

    fn attempt(&self) {
        self.invoke("attempt", None);
    }

    fn connected(&self) {
        self.invoke("connected", None);
    }

    fn server_fail(&self, reason: &str) {
        self.invoke("server_fail", Some(reason));
    }

    fn client_fail(&self, reason: &str) {
        self.invoke("client_fail", Some(reason));
    }

    fn gain_control(&self) {
        self.invoke("control", None);
    }

    fn lose_control(&self) {
        self.invoke("lostcontrol", None);
    }

    fn server_started(&self) {
        self.invoke("server", None);
    }

    fn set_session_code(&self, code: &str) {
        self.invoke("session", Some(code));
    }

    fn new_connection(&self, name: &str) {
        self.invoke("newconnection", Some(name));
    }

    fn lost_connection(&self, name: &str) {
        self.invoke("lostconnection", Some(name));
    }

    fn observing(&self, observing: bool) {
        if observing {
            self.invoke("observing", None);
        } else {
            self.invoke("stop_observing", None);
        }
    }

    fn set_observing(&self, name: &str, observing: bool) {
        if observing {
            self.invoke("set_observing", Some(name));
        } else {
            self.invoke("set_not_observing", Some(name));
        }
    }

    fn set_incontrol(&self, name: &str) {
        self.invoke("set_incontrol", Some(name));
    }

    fn add_fs2020_aircraft(&self, name: &str) {
        self.invoke("add_fs2020_aircraft", Some(name));
    }

    fn add_fs2024_aircraft(&self, name: &str) {
        self.invoke("add_fs2024_aircraft", Some(name));
    }

    fn set_aircraft(&self, config: &str) {
        self.invoke("set_aircraft", Some(config));
    }

    fn version(&self, version: &str) {
        self.invoke("version", Some(version))
    }

    fn update_failed(&self) {
        self.invoke("update_failed", None);
    }

    fn send_config(&self, value: &str) {
        self.invoke("config_msg", Some(value));
    }

    fn send_network(&self, metrics: &Metrics) {
        self.invoke(
            "metrics",
            Some(
                json!({
                    "sentPackets": metrics.sent_packets,
                    "receivePackets": metrics.received_packets,
                    "sentBandwidth": metrics.sent_kbps,
                    "receiveBandwidth": metrics.receive_kbps,
                    "packetLoss": metrics.packet_loss,
                    "ping": metrics.rtt/2.0
                })
                .to_string()
                .as_str(),
            ),
        )
    }

    fn set_host(&self) {
        self.invoke("host", None);
    }

    fn emulator_enabled(&self, enabled: bool) {
        self.invoke(
            "emulator_enabled",
            Some(if enabled { "true" } else { "false" }),
        );
    }

    fn send_emulator_vars(&self, value: &str) {
        self.invoke("emulator_vars", Some(value));
    }

    fn send_emulator_var_value(&self, value: &str) {
        self.invoke("emulator_value", Some(value));
    }

    fn emulator_error(&self, reason: &str) {
        self.invoke("emulator_error", Some(reason));
    }
}

fn read_logo() -> Vec<u8> {
    let mut logo = vec![];
    File::open("assets/logo.png")
        .unwrap()
        .read_to_end(&mut logo)
        .ok();
    logo
}

pub struct WebViewApp {
    app_handle: Arc<Mutex<Option<web_view::Handle<i32>>>>,
    exited: Arc<AtomicBool>,
    rx: Receiver<AppMessage>,
}

impl WebViewApp {
    pub fn setup(title: String) -> Self {
        let (tx, rx) = unbounded();
        let logo = read_logo();

        let handle = Arc::new(Mutex::new(None));
        let handle_clone = handle.clone();
        let exited = Arc::new(AtomicBool::new(false));
        let exited_clone = exited.clone();

        thread::spawn(move || {
            let webview = web_view::builder()
                .title(&title)
                .content(web_view::Content::Html(format!(
                    r##"<!DOCTYPE html>
                <html>
                <head>
                    <style>
                        {bootstrapcss}
                        {css}
                    </style>
                </head>
                    <body class="themed">
                    <img src="data:image/png;base64,{logo}" class="logo-image"/>
                    {body}
                    {emulator_body}
                </body>
                <script>
                    {jquery}
                    {bootstrapjs}
                    {js1}
                    {js_emulator}
                    {js}
                </script>
                </html>
            "##,
                    css = include_str!("../web/stylesheet.css"),
                    js = include_str!("../web/main.js"),
                    js1 = include_str!("../web/list.js"),
                    js_emulator = include_str!("../web/emulator.js"),
                    body = include_str!("../web/index.html"),
                    emulator_body = include_str!("../web/emulator.html"),
                    jquery = include_str!("../web/jquery.min.js"),
                    bootstrapjs = include_str!("../web/bootstrap.bundle.min.js"),
                    bootstrapcss = include_str!("../web/bootstrap.min.css"),
                    logo = base64::engine::general_purpose::STANDARD_NO_PAD.encode(logo.as_slice())
                )))
                .invoke_handler(move |_, arg| {
                    tx.try_send(serde_json::from_str(arg).unwrap()).ok();

                    Ok(())
                })
                .user_data(0)
                .resizable(true)
                .size(1040, 860)
                .build()
                .unwrap();

            let mut handle = handle_clone.lock().unwrap();
            *handle = Some(webview.handle());
            std::mem::drop(handle);

            webview.run().ok();
            exited_clone.store(true, SeqCst);
        });

        // Run
        Self {
            app_handle: handle,
            exited,
            rx,
        }
    }
}

impl App for WebViewApp {
    fn exited(&self) -> bool {
        self.exited.load(SeqCst)
    }

    fn get_next_message(&self) -> Result<AppMessage, TryRecvError> {
        self.rx.try_recv()
    }

    fn invoke(&self, type_string: &str, data: Option<&str>) {
        let handle = self.app_handle.lock().unwrap();
        if handle.is_none() {
            return;
        }
        // Send data to javascript
        let data = data.unwrap_or_default().to_string();
        let type_string = type_string.to_owned();
        handle
            .as_ref()
            .unwrap()
            .dispatch(move |webview| {
                webview
                    .eval(get_message_str(type_string.as_str(), data.as_str()).as_str())
                    .ok();
                Ok(())
            })
            .ok();
    }
}

fn fetch_url_text(url: &str) -> rouille::Response {
    if let attohttpc::Result::Ok(resp) = attohttpc::get(url).send() {
        rouille::Response::text(resp.text().unwrap_or_default())
    } else {
        rouille::Response::empty_400()
    }
}

pub struct HttpApp {
    tx: Sender<String>,
    rx: Receiver<AppMessage>,
}

impl HttpApp {
    pub fn setup(address: &str) -> Self {
        // We want invoke() to block, so we will use a 0-bounded channel.
        let (invoke_tx, invoke_rx) = bounded(0);
        let (msg_tx, msg_rx) = unbounded();
        let address = address.to_string();
        let logo = read_logo();

        thread::spawn(move || {
            rouille::start_server(address, move |request| {
                rouille::router!(request,
                    (GET) (/) => {
                        rouille::Response::html(format!(
                    r##"<!DOCTYPE html>
                <html>
                <head>
                    <style>
                        {bootstrapcss}
                        {css}
                    </style>
                </head>
                    <body class="themed">
                    <img src="data:image/png;base64,{logo}" class="logo-image"/>
                    {body}
                    {emulator_body}
                </body>
                <script>
                    {jquery}
                    {bootstrapjs}
                    {js_http}
                    {js1}
                    {js_emulator}
                    {js}
                </script>
                </html>
            "##,
                    js_http = include_str!("../web/http.js"),
                    css = include_str!("../web/stylesheet.css"),
                    js = include_str!("../web/main.js"),
                    js1 = include_str!("../web/list.js"),
                    js_emulator = include_str!("../web/emulator.js"),
                    body = include_str!("../web/index.html"),
                    emulator_body = include_str!("../web/emulator.html"),
                    jquery = include_str!("../web/jquery.min.js"),
                    bootstrapjs = include_str!("../web/bootstrap.bundle.min.js"),
                    bootstrapcss = include_str!("../web/bootstrap.min.css"),
                    logo = base64::engine::general_purpose::STANDARD_NO_PAD.encode(logo.as_slice())
                ))},
                    (GET) (/invoke) => {
                        if let Ok(eval) = invoke_rx.try_recv() {
                            rouille::Response::text(eval)
                        } else {
                            rouille::Response::empty_204()
                        }
                    },
                    (PUT) (/invoke) => {
                        let msg = rouille::try_or_400!(rouille::input::json_input(request));
                        msg_tx.try_send(msg).ok();
                        rouille::Response::text("ok")
                    },
                    (GET) (/external-ip/v4) => {
                        fetch_url_text("https://api.ipify.org")
                    },
                    (GET) (/external-ip/v6) => {
                        fetch_url_text("https://api6.ipify.org")
                    },
                    _ => rouille::Response::empty_404()
                )
            });
        });

        Self {
            tx: invoke_tx,
            rx: msg_rx,
        }
    }
}

impl App for HttpApp {
    fn exited(&self) -> bool {
        false
    }

    fn get_next_message(&self) -> Result<AppMessage, TryRecvError> {
        self.rx.try_recv()
    }

    fn invoke(&self, type_string: &str, data: Option<&str>) {
        let eval = get_message_str(type_string, data.unwrap_or_default());
        // Block until the frontend executes, exactly like the webview version.
        self.tx.send(eval).unwrap();
    }
}
