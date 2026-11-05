//! Test helper: a dummy X client with a given WM_CLASS that records
//! `_NET_ACTIVE_WINDOW` requests for it. Optionally publishes itself in
//! `_NET_CLIENT_LIST`, as an EWMH window manager would.
//!
//! Usage: x11-dummy INSTANCE CLASS LOGFILE [--client-list]

use std::io::Write;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    AtomEnum, ChangeWindowAttributesAux, ConnectionExt, CreateWindowAux, EventMask, PropMode,
    WindowClass,
};
use x11rb::protocol::Event;
use x11rb::wrapper::ConnectionExt as _;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [instance, class, log] = [&args[0], &args[1], &args[2]];
    let (conn, screen) = x11rb::connect(None).expect("connect to X");
    let root = conn.setup().roots[screen].root;
    let win = conn.generate_id().unwrap();
    conn.create_window(
        0,
        win,
        root,
        0,
        0,
        100,
        100,
        0,
        WindowClass::INPUT_OUTPUT,
        0,
        &CreateWindowAux::new(),
    )
    .unwrap();
    let mut wm_class = Vec::new();
    wm_class.extend_from_slice(instance.as_bytes());
    wm_class.push(0);
    wm_class.extend_from_slice(class.as_bytes());
    wm_class.push(0);
    conn.change_property8(
        PropMode::REPLACE,
        win,
        AtomEnum::WM_CLASS,
        AtomEnum::STRING,
        &wm_class,
    )
    .unwrap();
    let active = conn
        .intern_atom(false, b"_NET_ACTIVE_WINDOW")
        .unwrap()
        .reply()
        .unwrap()
        .atom;
    if args.iter().any(|a| a == "--client-list") {
        let list = conn
            .intern_atom(false, b"_NET_CLIENT_LIST")
            .unwrap()
            .reply()
            .unwrap()
            .atom;
        conn.change_property32(PropMode::APPEND, root, list, AtomEnum::WINDOW, &[win])
            .unwrap();
    }
    conn.change_window_attributes(
        root,
        &ChangeWindowAttributesAux::new().event_mask(EventMask::SUBSTRUCTURE_NOTIFY),
    )
    .unwrap();
    conn.map_window(win).unwrap();
    conn.flush().unwrap();
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log)
        .unwrap();
    writeln!(file, "ready 0x{win:x}").unwrap();
    loop {
        match conn.wait_for_event() {
            Ok(Event::ClientMessage(e)) if e.type_ == active && e.window == win => {
                writeln!(file, "activated 0x{win:x}").unwrap();
            }
            Ok(_) => {}
            Err(_) => return,
        }
    }
}
