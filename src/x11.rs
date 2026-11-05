//! Read-only window lookup.
//!
//! One short-lived connection per command: no event loop and no polling.

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt, Window};
use x11rb::rust_connection::RustConnection;

pub struct Windows {
    conn: RustConnection,
    root: Window,
}

#[derive(Debug, Clone)]
pub struct ClientWindow {
    pub id: Window,
    pub instance: String,
    pub class: String,
}

impl Windows {
    /// Connect to `$DISPLAY`; `None` when there is no X server.
    pub fn connect() -> Option<Self> {
        std::env::var_os("DISPLAY").filter(|d| !d.is_empty())?;
        let (conn, screen) = x11rb::connect(None).ok()?;
        let root = conn.setup().roots.get(screen)?.root;
        Some(Self { conn, root })
    }

    fn atom(&self, name: &str) -> Option<u32> {
        Some(
            self.conn
                .intern_atom(false, name.as_bytes())
                .ok()?
                .reply()
                .ok()?
                .atom,
        )
    }

    /// Managed client windows from `_NET_CLIENT_LIST`; without an EWMH window
    /// manager, the root window's direct children.
    fn client_ids(&self) -> Vec<Window> {
        if let Some(atom) = self.atom("_NET_CLIENT_LIST") {
            if let Some(reply) = self
                .conn
                .get_property(false, self.root, atom, AtomEnum::WINDOW, 0, 4096)
                .ok()
                .and_then(|cookie| cookie.reply().ok())
            {
                if reply.type_ != x11rb::NONE {
                    if let Some(values) = reply.value32() {
                        return values.collect();
                    }
                }
            }
        }
        self.conn
            .query_tree(self.root)
            .ok()
            .and_then(|cookie| cookie.reply().ok())
            .map(|tree| tree.children)
            .unwrap_or_default()
    }

    pub fn clients(&self) -> Vec<ClientWindow> {
        let ids = self.client_ids();
        let cookies: Vec<_> = ids
            .iter()
            .map(|&id| {
                self.conn
                    .get_property(false, id, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, 1024)
                    .ok()
            })
            .collect();
        let mut out = Vec::new();
        for (id, cookie) in ids.into_iter().zip(cookies) {
            let Some(reply) = cookie.and_then(|c| c.reply().ok()) else {
                continue;
            };
            let (instance, class) = split_wm_class(&reply.value);
            if instance.is_empty() && class.is_empty() {
                continue;
            }
            out.push(ClientWindow {
                id,
                instance,
                class,
            });
        }
        out
    }
}

/// WM_CLASS is two NUL-terminated strings: instance, then class.
pub fn split_wm_class(value: &[u8]) -> (String, String) {
    let mut parts = value.split(|&b| b == 0);
    let instance = String::from_utf8_lossy(parts.next().unwrap_or_default()).into_owned();
    let class = String::from_utf8_lossy(parts.next().unwrap_or_default()).into_owned();
    (instance, class)
}

#[cfg(test)]
mod tests {
    #[test]
    fn wm_class_is_split_on_nul() {
        assert_eq!(
            super::split_wm_class(b"teams.cloud.microsoft\0Helium\0"),
            ("teams.cloud.microsoft".into(), "Helium".into())
        );
        assert_eq!(super::split_wm_class(b""), (String::new(), String::new()));
    }
}
